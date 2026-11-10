//! pelway-agent-watch: T3 のスレッドを見張り、状態の変化を Grok Bot の webhook に通知する。
//!
//! 使い方: `pelway-agent-watch [--once] [--dry-run] [--project <path>]`
//! - 既定は 30 秒ごとのループ。`--once` は 1 サイクルで終わる
//! - `--dry-run` は送らずに payload を 1 行ずつ標準出力に出す
//! - ログは標準エラーに 1 行ずつ。秘密と返答の本文は出さない

mod io;

use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pelway_agent_watch::ci::ci_should_notify;
use pelway_agent_watch::format_utc;
use pelway_agent_watch::payload::build_payload;
use pelway_agent_watch::watch::{Event, StallReason, State, ThreadDetail, needs_detail, step};

const POLL: Duration = Duration::from_secs(30);
const CI_INTERVAL: Duration = Duration::from_secs(5 * 60);

struct Options {
    once: bool,
    dry_run: bool,
    project: PathBuf,
}

fn parse_args() -> Result<Options, String> {
    let mut opts = Options {
        once: false,
        dry_run: false,
        project: PathBuf::from("."),
    };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--once" => opts.once = true,
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

fn main() {
    let opts = match parse_args() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("agent-watch: {e}");
            eprintln!("usage: pelway-agent-watch [--once] [--dry-run] [--project <path>]");
            std::process::exit(2);
        }
    };
    let host = io::hostname();
    let state_path = io::state_path();
    let mut last_ci: Option<Instant> = None;
    loop {
        let check_ci = last_ci.is_none_or(|t| t.elapsed() >= CI_INTERVAL);
        cycle(&opts, &host, &state_path, check_ci);
        if check_ci {
            last_ci = Some(Instant::now());
        }
        if opts.once {
            break;
        }
        sleep(POLL);
    }
}

/// 1 サイクル: 一覧 → 変わったものだけ詳細 → 判定 → CI 確認 → 送信 → 状態の保存。
fn cycle(opts: &Options, host: &str, state_path: &Path, check_ci: bool) {
    let listed = match io::t3_list(&opts.project) {
        Ok(l) => l,
        Err(e) => {
            // 一時的な失敗で「全スレッドが消えた」と誤解しないよう、このサイクルは何もしない
            eprintln!("agent-watch: skip cycle: {e}");
            return;
        }
    };
    let mut state = io::load_state(state_path);
    let details: Vec<ThreadDetail> = listed
        .iter()
        .filter(|l| needs_detail(&state, l))
        .filter_map(|l| {
            io::t3_show(&l.id)
                .map_err(|e| eprintln!("agent-watch: {e}"))
                .ok()
        })
        .collect();
    let now = now_epoch();
    let mut events = step(&mut state, &details, &listed, now);
    if check_ci && state.initialized {
        events.extend(ci_events(opts, &mut state));
    }
    let occurred_at = format_utc(now);
    for (event, detail) in &events {
        emit(opts, host, &occurred_at, event, detail);
    }
    if let Err(e) = io::save_state(state_path, &state) {
        eprintln!("agent-watch: cannot save state: {e}");
    }
}

/// `agent/` ブランチの PR で verify が続けて失敗していれば `stalled(ci-failed-twice)`。
/// その PR を作ったスレッドが分かれば、そのスレッドの詳細を付ける。
fn ci_events(opts: &Options, state: &mut State) -> Vec<(Event, ThreadDetail)> {
    let branches = match io::open_agent_branches(&opts.project) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("agent-watch: {e}");
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for branch in branches {
        let Ok(runs) =
            io::gh_runs(&opts.project, &branch).map_err(|e| eprintln!("agent-watch: {e}"))
        else {
            continue;
        };
        if ci_should_notify(state, &branch, &runs) {
            let detail = ThreadDetail {
                branch: Some(branch),
                ..ThreadDetail::default()
            };
            out.push((Event::Stalled(StallReason::CiFailedTwice), detail));
        }
    }
    out
}

fn emit(opts: &Options, host: &str, occurred_at: &str, event: &Event, detail: &ThreadDetail) {
    let payload = build_payload(event, detail, host, occurred_at);
    let tag = format!(
        "event={} thread={} branch={}",
        payload["event"].as_str().unwrap_or(""),
        detail.id,
        detail.branch.as_deref().unwrap_or("-")
    );
    if opts.dry_run {
        println!("{payload}");
        eprintln!("agent-watch: {tag} mode=dry-run");
        return;
    }
    match io::send(&payload, &opts.project) {
        io::SendResult::Sent(code) => eprintln!("agent-watch: {tag} mode=send http={code}"),
        io::SendResult::Failed(why) => {
            eprintln!("agent-watch: {tag} mode=send-failed reason={why}")
        }
        io::SendResult::DryRunNoSecret => {
            eprintln!("agent-watch: {tag} mode=dry-run reason=no-secret")
        }
    }
}
