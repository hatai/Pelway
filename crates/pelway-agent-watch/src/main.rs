//! pelway-agent-watch: T3 のスレッドを見張り、状態の変化を Grok Bot の webhook に通知する。
//!
//! 使い方: `pelway-agent-watch [--once | --cycles <n>] [--dry-run] [--project <path>]`
//! - 既定は 30 秒ごとのループ（`PELWAY_AGENT_WATCH_POLL_SECS` で変えられる）。`--once` は 1 周、`--cycles <n>` は n 周で終わる
//! - `--dry-run` は送らずに新しいイベントの payload を 1 行ずつ標準出力に出す。状態ファイルは読むだけで書き換えない
//! - 状態は起動時に 1 回だけ読み、以降はメモリに持つ。保存に失敗しても同じイベントを繰り返し作らない
//! - 送れなかった payload は状態の送信待ちの列に残し、次の周回で送り直す
//! - ログは標準エラーに 1 行ずつ。秘密と返答の本文は出さない

mod io;

use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelway_agent_watch::ci::{ci_should_notify, newest_run_is_head};
use pelway_agent_watch::format_utc;
use pelway_agent_watch::outbox;
use pelway_agent_watch::payload::build_payload;
use pelway_agent_watch::watch::{Event, StallReason, State, ThreadDetail, needs_detail, step};
use serde_json::Value;

const DEFAULT_POLL_SECS: u64 = 30;
const CI_INTERVAL: Duration = Duration::from_secs(5 * 60);
const USAGE: &str =
    "usage: pelway-agent-watch [--once | --cycles <n>] [--dry-run] [--project <path>]";

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

struct Watcher {
    opts: Options,
    host: String,
    state_path: PathBuf,
    state: State,
    last_ci: Option<Instant>,
}

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("agent-watch: {e}");
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    };
    let state_path = io::state_path();
    let state = io::load_state(&state_path);
    let mut w = Watcher {
        host: io::hostname(),
        state_path,
        state,
        last_ci: None,
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
        let listed = match io::t3_list(&self.opts.project) {
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
                io::t3_show(&l.id)
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

        for (payload, tag) in payloads {
            eprintln!("agent-watch: queued {tag}");
            outbox::push(&mut self.state.outbox, payload);
        }
        let project = self.opts.project.clone();
        outbox::flush(&mut self.state.outbox, |payload| {
            let tag = tag(payload);
            match io::send(payload, &project) {
                io::SendResult::Sent(code) => {
                    eprintln!("agent-watch: {tag} mode=send http={code}");
                    true
                }
                io::SendResult::Failed(why) => {
                    eprintln!("agent-watch: {tag} mode=send-failed reason={why} (kept for retry)");
                    false
                }
                io::SendResult::DryRunNoSecret => {
                    eprintln!("agent-watch: {tag} mode=not-sent reason=no-secret (kept for retry)");
                    false
                }
            }
        });
        if let Err(e) = io::save_state(&self.state_path, &self.state) {
            eprintln!(
                "agent-watch: ERROR cannot save state {}: {e}",
                self.state_path.display()
            );
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
            let branch = pr.branch;
            let Ok(runs) =
                io::gh_runs(&self.opts.project, &branch).map_err(|e| eprintln!("agent-watch: {e}"))
            else {
                continue;
            };
            // 同じブランチ名の fork の実行を拾わないよう、最新の実行が PR の head に対するものだけを見る
            if !newest_run_is_head(&runs, &pr.head_sha) {
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
