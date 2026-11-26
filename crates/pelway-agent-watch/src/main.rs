//! pelway-agent-watch: T3 のスレッドを見張り、状態の変化を Grok Bot の webhook に通知する。
//! 同じ知らせを指揮役の受信箱（共有の git dir の `pelway/conductor-inbox.jsonl`）にも書き、
//! ホストの登録ファイル（`~/.config/pelway-agent-notify/conductor-thread-id`）に登録された
//! いまの指揮役のスレッドを決まった文で起こす（Issue #555）。既読の位置は補助記録から読む。
//!
//! 使い方: `pelway-agent-watch [--once | --cycles <n>] [--dry-run] [--project <path>]`
//! 登録: `pelway-agent-watch --register <thread-id> | --unregister | --show-registration`
//! （hatai がホストで、インストールした版を実行する。見張りは始めない）
//! - 既定は 30 秒ごとのループ（`PELWAY_AGENT_WATCH_POLL_SECS` で変えられる）。`--once` は 1 周、`--cycles <n>` は n 周で終わる
//! - `--dry-run` は送らずに新しいイベントの payload を 1 行ずつ標準出力に出す。状態ファイルは読むだけで書き換えない
//! - 起動してよいのは本体 checkout からだけ。worktree では理由を 1 行ログに出して終わる
//! - 同時に動かせるのは 1 つだけ。状態ディレクトリの lock ファイルを flock で取り、取れなければ終わる
//! - 状態は起動時に 1 回だけ読み、以降はメモリに持つ。保存に失敗しても同じイベントを繰り返し作らない
//! - webhook に送れなかった payload は状態の送信待ちの列に残し、古い順に送り直す。直らない失敗
//!   （HTTP 400・404・413）は列から外し、送り直せる失敗が続く間は間隔を空ける（`outbox::flush`）
//! - ログは標準エラーに 1 行ずつ。秘密と返答の本文は出さない

mod io;

use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelway_agent_watch::ci::{ci_should_notify, runs_belong_to_pr};
use pelway_agent_watch::format_utc;
use pelway_agent_watch::inbox::{
    INBOX_FILE, NoticeBatch, PATROL_STATE_FILE, PatrolInfo, WAKE_MESSAGE, WakeDecision,
    WakeHistory, WakeNotice, classify_notices, decide_wake, inbox_line, parse_conductor_file,
    parse_patrol_state,
};
use pelway_agent_watch::outbox::{self, Outcome};
use pelway_agent_watch::payload::build_payload;
use pelway_agent_watch::watch::{Event, StallReason, State, ThreadDetail, needs_detail, step};
use serde_json::Value;

const DEFAULT_POLL_SECS: u64 = 30;
/// 起こしの最短間隔の既定（秒）。`PELWAY_AGENT_WATCH_WAKE_COOLDOWN_SECS` で変えられる（Issue #839）。
const DEFAULT_WAKE_COOLDOWN_SECS: i64 = 5 * 60;
const CI_INTERVAL: Duration = Duration::from_secs(5 * 60);
/// 1 周の webhook の送信の期限。期限が残る間は次の知らせを送り始め、秘密を読む fnox・curl・送り直しの待ちは
/// 期限で止める（`io::send`）ので、1 周の送信は列の長さによらず 45 秒以内に収まる。期限までに送れなかった知らせは列に残る。
const SEND_DEADLINE: Duration = Duration::from_secs(40);
const USAGE: &str =
    "usage: pelway-agent-watch [--once | --cycles <n>] [--dry-run] [--project <path>]
       pelway-agent-watch --register <thread-id> | --unregister | --show-registration";

struct Options {
    cycles: Option<u64>,
    dry_run: bool,
    project: PathBuf,
}

fn parse_args() -> Result<Options, String> {
    let mut opts = Options {
        cycles: None,
        dry_run: false,
        project: PathBuf::from("."),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--once" => opts.cycles = Some(1),
            "--cycles" => {
                let n = args.next().ok_or("--cycles needs a number")?;
                opts.cycles = Some(
                    n.parse()
                        .map_err(|_| format!("--cycles: not a number: {n}"))?,
                );
            }
            "--dry-run" => opts.dry_run = true,
            "--project" => {
                opts.project = PathBuf::from(args.next().ok_or("--project needs a path")?)
            }
            _ => return Err(format!("unknown argument: {a}")),
        }
    }
    Ok(opts)
}

/// 起動してよいかの確認。本体 checkout からの起動だけを許し（worktree は見張りが二重になる）、
/// 状態ディレクトリの lock で同時実行を 1 つに絞る。拒否は意図した終了なので終了コード 0
/// （pitchfork の再起動を呼ばない。agent-worktree-prune と同じ決まり）。確認自体に失敗したときは 1。
/// worktree の判定は見張り対象（`--project`）ではなく起動元（実プロセスの cwd）で行う:
/// worktree から `--project` に本体 checkout を渡して起動する抜け道をふさぐため。
/// さらに `--project` が起動元の checkout の toplevel と一致することも確認する:
/// 本体から別の checkout を見張る起動は本体の見張りとの重複・すり替わりになるため。
/// 拒否のログにはパスの種類だけを書き、中身や環境変数は出さない。
/// 起動してよければ、lock と、指揮役の受信箱を置く共有の git dir の `pelway/` を返す。
fn startup_guards(opts: &Options) -> (std::fs::File, PathBuf) {
    let cwd = match std::env::current_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("agent-watch: cannot check the launch directory: {e}");
            std::process::exit(1);
        }
    };
    let (dir, common, toplevel) = match io::git_dirs(&cwd) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("agent-watch: cannot check the checkout: {e}");
            std::process::exit(1);
        }
    };
    if dir != common {
        eprintln!("agent-watch: refusing to start: linked worktree is not the main checkout");
        std::process::exit(0);
    }
    let project = if opts.project.is_absolute() {
        opts.project.clone()
    } else {
        cwd.join(&opts.project)
    };
    match (
        std::fs::canonicalize(&project),
        std::fs::canonicalize(PathBuf::from(toplevel.trim())),
    ) {
        (Ok(p), Ok(t)) if p == t => {}
        (Ok(_), Ok(_)) => {
            eprintln!("agent-watch: refusing to start: --project is not the launch checkout");
            std::process::exit(0);
        }
        _ => {
            eprintln!("agent-watch: cannot check the watched checkout");
            std::process::exit(1);
        }
    }
    // 受信箱と補助記録の置き場（共有の git dir。主 checkout と全 worktree で同じ場所）
    let common = PathBuf::from(common.trim());
    let pelway_dir = if common.is_absolute() {
        common
    } else {
        cwd.join(common)
    }
    .join("pelway");
    let lock_path = io::lock_path();
    match io::try_lock(&lock_path) {
        Ok(Some(file)) => (file, pelway_dir),
        Ok(None) => {
            eprintln!("agent-watch: refusing to start: another instance holds the lock");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!(
                "agent-watch: cannot take the lock {}: {e}",
                lock_path.display()
            );
            std::process::exit(1);
        }
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0))
}

fn poll_interval() -> Duration {
    let secs = std::env::var("PELWAY_AGENT_WATCH_POLL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_POLL_SECS);
    Duration::from_secs(secs)
}

/// 起こしの最短間隔（秒）。読めない値は既定にする。
fn wake_cooldown_secs() -> i64 {
    std::env::var("PELWAY_AGENT_WATCH_WAKE_COOLDOWN_SECS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .filter(|s| *s >= 0)
        .unwrap_or(DEFAULT_WAKE_COOLDOWN_SECS)
}

struct Watcher {
    opts: Options,
    /// 共有の git dir の `pelway/`（受信箱 `conductor-inbox.jsonl` と補助記録 `patrol-state.json` の置き場）。
    pelway_dir: PathBuf,
    host: String,
    state_path: PathBuf,
    state: State,
    last_ci: Option<Instant>,
}

/// 登録の操作（`--register` など）なら実行して終わる。見張りの引数なら何もしない。
/// 登録はホスト側の登録ファイルだけを触り、checkout の確認と lock は要らない。
fn registration_command() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = io::conductor_file();
    let result = match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["--register", id] => io::register_conductor(&path, id).map(|()| "registered".to_string()),
        ["--unregister"] => io::unregister_conductor(&path).map(|()| "unregistered".to_string()),
        ["--show-registration"] => io::read_conductor_file(&path).map(|t| {
            t.and_then(|t| parse_conductor_file(&t))
                .unwrap_or_else(|| "(none)".to_string())
        }),
        ["--register", ..] => Err("--register needs one thread id".to_string()),
        _ => return,
    };
    match result {
        Ok(msg) => {
            println!("{msg}");
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("agent-watch: {e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    registration_command();
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("agent-watch: {e}");
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    let (_lock, pelway_dir) = startup_guards(&opts);
    let state_path = io::state_path();
    let state = io::load_state(&state_path);
    let mut w = Watcher {
        host: io::hostname(),
        state_path,
        state,
        last_ci: None,
        pelway_dir,
        opts,
    };
    let poll = poll_interval();
    let mut done = 0u64;
    loop {
        w.cycle();
        done += 1;
        if w.opts.cycles.is_some_and(|n| done >= n) {
            break;
        }
        sleep(poll);
    }
}

impl Watcher {
    /// 1 周: 一覧 → 変わったものだけ詳細 → 判定 → CI 確認 → 送信待ちの列に積む → 送る → 保存。
    fn cycle(&mut self) {
        // 一覧・詳細・起こしで同じ t3 を使う（PATH の t3 がサーバーと食い違っても止まらない）
        let t3 = io::t3_bin();
        let listed = match io::t3_list(&t3, &self.opts.project) {
            Ok(l) => l,
            Err(e) => {
                // 一時的な失敗で「全スレッドが消えた」と誤解しないよう、この周は何もしない
                eprintln!("agent-watch: skip cycle: {e}");
                return;
            }
        };
        let details: Vec<ThreadDetail> = listed
            .iter()
            .filter(|l| needs_detail(&self.state, l))
            .filter_map(|l| {
                io::t3_show(&t3, &l.id)
                    .map_err(|e| eprintln!("agent-watch: {e}"))
                    .ok()
            })
            .collect();
        let was_initialized = self.state.initialized;
        let now = now_epoch();
        let mut events = step(&mut self.state, &details, &listed, now);
        let check_ci =
            self.state.initialized && self.last_ci.is_none_or(|t| t.elapsed() >= CI_INTERVAL);
        if check_ci {
            // 初回の周回では、既にある連続失敗を記録するだけで通知しない
            events.extend(self.ci_events(was_initialized));
            self.last_ci = Some(Instant::now());
        }

        let occurred_at = format_utc(now);
        let payloads: Vec<(Value, String)> = events
            .iter()
            .map(|(event, detail)| {
                let payload = build_payload(event, detail, &self.host, &occurred_at);
                let tag = tag(&payload);
                (payload, tag)
            })
            .collect();

        if self.opts.dry_run {
            for (payload, tag) in &payloads {
                println!("{payload}");
                eprintln!("agent-watch: {tag} mode=dry-run");
            }
            return;
        }

        // 指揮役の受信箱に、知らせを出した時点で書く（送信待ちの列の再送では書かない）。
        // 書けなかった知らせは状態に残し、受信箱が直るまで周回ごとに古い順に書き直す
        // （webhook が外した知らせも、上限 100 件の内なら後で届く）。受信箱に書けなくても webhook の送信と見張りは続ける。
        // 更新前の版の状態では、列の知らせを書けたかが分からないので、最初の周回で列の写しを書き直す（重複しうる）
        let pending = self.state.inbox_pending.replace(Vec::new());
        let mut to_inbox: Vec<(Value, Option<String>)> = pending
            .unwrap_or_else(|| self.state.outbox.clone())
            .iter()
            .map(|item| {
                let stored = item
                    .get("thread_id")
                    .and_then(Value::as_str)
                    .and_then(|id| self.state.inbox_pending_creators.get(id).cloned());
                (item.clone(), stored)
            })
            .collect();
        to_inbox.extend(payloads.iter().map(|(p, _)| (p.clone(), None)));
        if !to_inbox.is_empty() {
            let lines: Vec<String> = to_inbox.iter().map(|(p, _)| inbox_line(p)).collect();
            // 起こしの判定に使う、知らせごとの種類（event・report の有無）と元のスレッドの立て手。
            // 立て手は詳細（t3 thread show）を優先し、無ければ一覧（t3 thread list）の createdBy を使う。
            // どちらにも無ければ立て手不明（None。起こす側に倒す。inbox::human_created）
            let creator_of = |id: &str| -> Option<String> {
                details
                    .iter()
                    .find(|d| d.id == id)
                    .and_then(|d| d.created_by.clone())
                    .or_else(|| {
                        listed
                            .iter()
                            .find(|l| l.id == id)
                            .and_then(|l| l.created_by.clone())
                    })
            };
            let notices: Vec<WakeNotice> = to_inbox
                .iter()
                .map(|(p, stored)| {
                    let thread_id = p
                        .get("thread_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    WakeNotice {
                        // 再試行の記録に判明済みの立て手があればそれを使う
                        // （完了済みのスレッドはこの周回で詳細を取り直さないため）
                        created_by: stored.clone().or_else(|| creator_of(&thread_id)),
                        thread_id,
                        event: p
                            .get("event")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        round: p
                            .get("round")
                            .and_then(Value::as_u64)
                            .and_then(|r| u32::try_from(r).ok())
                            // T3 が実行回数を返さないと 0 になる。0 は round として数えない
                            .filter(|r| *r > 0),
                        has_report: p.get("report").is_some_and(Value::is_string),
                    }
                })
                .collect();
            match io::append_inbox(&self.pelway_dir.join(INBOX_FILE), &lines) {
                Ok(total) => self.maybe_wake(&t3, total, &notices),
                Err(e) => {
                    eprintln!("agent-watch: ERROR inbox append failed: {e}");
                    let pending = self.state.inbox_pending.get_or_insert_default();
                    for ((p, _), notice) in to_inbox.iter().zip(&notices) {
                        // 記録の要素は payload そのものに留める（別の形に包むと、更新前の
                        // 版に戻したときに包んだ形のまま受信箱に書かれて読み手に拾えない）。
                        // 判明している立て手は別項目に thread_id で残す
                        outbox::push(pending, p.clone());
                        if let (Some(id), Some(c)) = (
                            p.get("thread_id").and_then(Value::as_str),
                            notice.created_by.as_deref(),
                        ) {
                            self.state
                                .inbox_pending_creators
                                .insert(id.to_string(), c.to_string());
                        }
                    }
                }
            }
        } else if self.state.wake_pending.is_some() {
            // 新しい知らせの無い周回でも、後に回した起こし（起こし待ちの印）を確かめる（Issue #839）。
            // 受信箱の行数は、行を足さない追記の手続きで数える（受信箱が無ければ空で作り、末尾の
            // 改行の欠けた断片は改行で閉じる）。書けない受信箱は、知らせを書く
            // 周回で `inbox append failed` として出るので、ここでは印を残したまま黙って次の周回に回す
            if let Ok(total) = io::append_inbox(&self.pelway_dir.join(INBOX_FILE), &[]) {
                self.maybe_wake(&t3, total, &[]);
            }
        }

        // 承認待ちの記録は、一覧の記録に残るスレッドの分だけを持つ
        let threads = &self.state.threads;
        self.state
            .wake_waiting_rounds
            .retain(|id, _| threads.contains_key(id));

        // キューに残った知らせの分だけ立て手を残す。全件書けた場合・上限で古い
        // 知らせが外れた場合・キューが空の場合は、対応する項目も消す
        // （残し続けると、追記の失敗が繰り返されるたびに状態ファイルが増え続ける）
        let live: std::collections::BTreeSet<String> = self
            .state
            .inbox_pending
            .iter()
            .flatten()
            .filter_map(|p| {
                p.get("thread_id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect();
        self.state
            .inbox_pending_creators
            .retain(|id, _| live.contains(id));

        for (payload, tag) in payloads {
            eprintln!("agent-watch: queued {tag}");
            outbox::push(&mut self.state.outbox, payload);
        }
        let project = self.opts.project.clone();
        let deadline = Instant::now() + SEND_DEADLINE;
        let backoff_before = self.state.outbox_backoff;
        outbox::flush(
            &mut self.state.outbox,
            &mut self.state.outbox_backoff,
            now_epoch,
            || io::has_time(deadline),
            |payload| {
                let tag = tag(payload);
                match io::send(payload, &project, deadline) {
                    io::SendResult::Sent(code) => {
                        eprintln!("agent-watch: {tag} mode=send http={code}");
                        Outcome::Sent
                    }
                    io::SendResult::Rejected(code) => {
                        eprintln!("agent-watch: {tag} mode=send-dropped http={code}");
                        Outcome::Rejected
                    }
                    io::SendResult::Failed(why) => {
                        eprintln!(
                            "agent-watch: {tag} mode=send-failed reason={why} (kept for retry)"
                        );
                        Outcome::Retry
                    }
                    io::SendResult::DryRunNoSecret => {
                        eprintln!(
                            "agent-watch: {tag} mode=not-sent reason=no-secret (kept for retry)"
                        );
                        Outcome::Retry
                    }
                }
            },
        );
        if let Some(b) = self
            .state
            .outbox_backoff
            .filter(|b| Some(*b) != backoff_before)
        {
            eprintln!(
                "agent-watch: outbox retry_after={}s queued={}",
                b.interval_secs,
                self.state.outbox.len()
            );
        }
        if let Err(e) = io::save_state(&self.state_path, &self.state) {
            eprintln!(
                "agent-watch: ERROR cannot save state {}: {e}",
                self.state_path.display()
            );
        }
    }

    /// 受信箱に書いたあと（または起こし待ちの印があるとき）、ホストの登録ファイルに登録された
    /// いまの指揮役を起こす。`notices` はその周に受信箱へ書いた知らせの要約（種類・round・元の
    /// スレッドの立て手。Issue #727）で、起こし待ちの印だけを確かめる周では空。
    /// 判定は `classify_notices`（指揮役自身の知らせ・同じ承認待ちの繰り返しを除く）と
    /// `decide_wake`（最短間隔・既読の位置）で行う（Issue #839）。
    /// 送れなかったときは記録を進めず、起こし待ちの印も消し、次の新しい知らせで改めて試みる
    /// （再送の列は持たない）。印だけを確かめる周（`notices` が空）では、起こさなかった理由の
    /// ログを出さない（印が残る間、周回ごとに同じ行が増えないように）。
    fn maybe_wake(&mut self, t3: &std::path::Path, inbox_lines: u64, notices: &[WakeNotice]) {
        let log = |reason: &str| {
            if !notices.is_empty() {
                eprintln!("agent-watch: WAKE skipped reason={reason}");
            }
        };
        // 宛先はホスト側の登録ファイルからだけ読む（sandbox から書ける補助記録の値は使わない）
        let registered = match io::read_conductor_file(&io::conductor_file()) {
            Ok(Some(text)) => parse_conductor_file(&text),
            Ok(None) => None,
            Err(_) => {
                log("conductor-file-unreadable");
                return;
            }
        };
        let Some(registered) = registered else {
            // 登録が無い・空（指揮役が T3 のスレッドとして動いていない）。起こす宛先が無いので、
            // 後に回した起こしも取り消す
            self.state.wake_pending = None;
            return;
        };
        // 前回の起こしの記録は、そのときの宛先にだけ使う（指揮役が替わったら一度も起こしていない扱い）
        let same_target = self.state.last_wake_thread_id.as_deref() == Some(registered.as_str());
        let batch = classify_notices(notices, &registered, &mut self.state.wake_waiting_rounds);
        // 起こすに足る新しい知らせのうち最後のものまでの受信箱の行数（知らせは受信箱の末尾に、
        // 並びの順に 1 行ずつ書いてある）
        let fresh_upto = match batch {
            NoticeBatch::Worthy(last) => {
                Some(inbox_lines.saturating_sub((notices.len() - 1 - last) as u64))
            }
            _ => None,
        };
        // 起こせなかった新しい知らせを、起こし待ちの印に入れる（後の周回で確かめ直す）
        let defer = |state: &mut State| state.wake_pending = state.wake_pending.max(fresh_upto);
        if fresh_upto.is_none() && self.state.wake_pending.is_none() {
            // 知らせの元が指揮役自身だけの回（巡回の終わりなど）は起こさない。起こすと、
            // 巡回の終わり → 知らせ → 起こし → 巡回、を繰り返すため
            let reason = match batch {
                NoticeBatch::OwnOnly => "own-event",
                NoticeBatch::RepeatWaiting => "repeat-waiting",
                NoticeBatch::HumanNonUrgent => "human-non-urgent",
                NoticeBatch::Empty | NoticeBatch::Worthy(_) => return,
            };
            eprintln!("agent-watch: WAKE skipped reason={reason}");
            return;
        }
        // 既読の位置は補助記録から読む。無い・書きかけ・壊れているときは、その回は起こさない
        // （conductor-watch が書き直している途中のこともある）
        let patrol_state = self.pelway_dir.join(PATROL_STATE_FILE);
        let inbox_offset =
            match io::read_patrol_state(&patrol_state).and_then(|t| parse_patrol_state(&t)) {
                Ok(info) => info.inbox_offset,
                Err(e) if e.contains("too large") => {
                    // 書きかけ・壊れているのと見分けられるよう、上限超えは別の理由で出す
                    log("patrol-state-too-large");
                    defer(&mut self.state);
                    return;
                }
                Err(_) => {
                    log("patrol-state-unreadable");
                    defer(&mut self.state);
                    return;
                }
            };
        // 既読の位置は受信箱の行数を超えられない。超える値（書き損じ・sandbox からの書き換え）は
        // 使わず保存もしない（保存すると、補助記録を直しても以後の起こしが止まり続けるため）
        if inbox_offset > inbox_lines {
            log("offset-out-of-range");
            defer(&mut self.state);
            return;
        }
        let history = WakeHistory {
            last_offset: self.state.last_wake_inbox_offset.filter(|_| same_target),
            last_at: self.state.last_wake_at.filter(|_| same_target),
            pending: self.state.wake_pending,
        };
        let info = PatrolInfo {
            conductor_thread_id: Some(registered),
            inbox_offset,
        };
        let cooldown = wake_cooldown_secs();
        match decide_wake(&info, &history, fresh_upto, now_epoch(), cooldown) {
            WakeDecision::Send(thread_id) => match io::wake(t3, &thread_id, WAKE_MESSAGE) {
                Ok(()) => {
                    eprintln!("agent-watch: WAKE sent inbox_lines={inbox_lines}");
                    self.state.last_wake_inbox_offset = Some(inbox_offset);
                    // 最短間隔は送り終えた時刻から数える（送るのにかかった時間の分、次の起こしを早めない）
                    self.state.last_wake_at = Some(now_epoch());
                    self.state.last_wake_thread_id = Some(thread_id);
                    self.state.wake_pending = None;
                }
                Err(why) => {
                    eprintln!("agent-watch: WAKE failed {why}");
                    // 送り直さない（次の新しい知らせで改めて試みる）
                    self.state.wake_pending = None;
                }
            },
            WakeDecision::Cooldown => {
                if self.state.wake_pending.is_none() {
                    eprintln!("agent-watch: WAKE deferred reason=cooldown");
                }
                defer(&mut self.state);
            }
            WakeDecision::AlreadyRead => {
                eprintln!("agent-watch: WAKE skipped reason=already-read");
                self.state.wake_pending = None;
            }
            WakeDecision::NoConductor | WakeDecision::Nothing => {}
            WakeDecision::InvalidThreadId => {
                log("invalid-thread-id");
            }
            WakeDecision::NotYetRead => {
                log("not-yet-read");
                defer(&mut self.state);
            }
        }
    }

    /// `agent/` ブランチの PR で verify が続けて失敗していれば `stalled(ci-failed-twice)`。
    /// payload に付くのはブランチだけで、スレッドの詳細は付かない。
    fn ci_events(&mut self, emit: bool) -> Vec<(Event, ThreadDetail)> {
        let prs = match io::open_agent_prs(&self.opts.project) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("agent-watch: {e}");
                return Vec::new();
            }
        };
        let mut out = Vec::new();
        for pr in prs {
            let branch = pr.branch.clone();
            let Ok(runs) =
                io::gh_runs(&self.opts.project, &branch).map_err(|e| eprintln!("agent-watch: {e}"))
            else {
                continue;
            };
            // 同じブランチ名の fork の実行を拾わないよう、PR 自身のコミットに対する実行だけを見る
            if !runs_belong_to_pr(&runs, &pr) {
                continue;
            }
            if ci_should_notify(&mut self.state, &branch, &runs, emit) {
                let detail = ThreadDetail {
                    branch: Some(branch),
                    ..ThreadDetail::default()
                };
                out.push((Event::Stalled(StallReason::CiFailedTwice), detail));
            }
        }
        out
    }
}

/// ログ用の短い識別（秘密と返答の本文は含めない）。
fn tag(payload: &Value) -> String {
    format!(
        "event={} thread={} branch={}",
        payload["event"].as_str().unwrap_or(""),
        payload["thread_id"].as_str().unwrap_or(""),
        payload["branch"].as_str().unwrap_or("-")
    )
}
