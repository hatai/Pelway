//! 外部コマンド（t3・gh・fnox・curl）とファイルの読み書き。
//! 秘密（webhook の URL と鍵）は引数・ログに出さない。

use std::fs;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use pelway_agent_watch::ci::RunInfo;
use pelway_agent_watch::t3::{parse_list, parse_show};
use pelway_agent_watch::watch::{ListedThread, State, ThreadDetail};
use serde_json::Value;

/// Grok Bot の webhook の URL と鍵を置く fnox 設定（pelway-agent-notify と共用）。
const NOTIFY_FNOX: &str = ".config/pelway-agent-notify/fnox.toml";

/// 外部コマンド 1 回あたりの制限時間（秒）。固まったコマンドで見張り役ごと止まらないため。
const COMMAND_TIMEOUT_SECS: &str = "30";

fn run(cmd: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    // coreutils の timeout で包む。制限を超えたら終了コード 124 で失敗扱いになる
    let out = Command::new("timeout")
        .arg(COMMAND_TIMEOUT_SECS)
        .arg(cmd)
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{cmd}: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{cmd} {}: exit {}",
            args.first().unwrap_or(&""),
            out.status.code().unwrap_or(-1)
        ));
    }
    String::from_utf8(out.stdout).map_err(|e| format!("{cmd}: {e}"))
}

pub fn t3_list(project: &Path) -> Result<Vec<ListedThread>, String> {
    let project = project.to_string_lossy();
    parse_list(&run(
        "t3",
        &["thread", "list", "--project", &project, "--json"],
        Path::new("."),
    )?)
}

pub fn t3_show(id: &str) -> Result<ThreadDetail, String> {
    parse_show(&run(
        "t3",
        &["thread", "show", id, "--json"],
        Path::new("."),
    )?)
}

/// 開いている PR のうち `agent/` ブランチのもの。
pub fn open_agent_branches(project: &Path) -> Result<Vec<String>, String> {
    let out = run(
        "gh",
        &["pr", "list", "--state", "open", "--json", "headRefName"],
        project,
    )?;
    let v: Value = serde_json::from_str(&out).map_err(|e| format!("gh pr list: {e}"))?;
    Ok(v.as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p["headRefName"].as_str())
        .filter(|b| b.starts_with("agent/"))
        .map(str::to_string)
        .collect())
}

pub fn gh_runs(project: &Path, branch: &str) -> Result<Vec<RunInfo>, String> {
    let out = run(
        "gh",
        &[
            "run",
            "list",
            "--workflow",
            "verify.yml",
            "--branch",
            branch,
            "--limit",
            "2",
            "--json",
            "conclusion,headSha",
        ],
        project,
    )?;
    serde_json::from_str(&out).map_err(|e| format!("gh run list: {e}"))
}

pub fn hostname() -> String {
    run("hostname", &["-s"], Path::new("."))
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
}

pub fn state_path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/state"));
    base.join("pelway/agent-watch.json")
}

/// 状態ファイルを読む。無ければ初回扱い、壊れていればログに残して初回扱いにする。
pub fn load_state(path: &Path) -> State {
    match fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
            eprintln!("agent-watch: state file is broken, starting from a baseline: {e}");
            State::default()
        }),
        Err(_) => State::default(),
    }
}

/// 一時ファイルに書いてから置き換える（途中で止まっても壊れたファイルを残さない）。
pub fn save_state(path: &Path, state: &State) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    let body = serde_json::to_string(state).map_err(|e| e.to_string())?;
    fs::write(&tmp, body).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

/// 送信の結果。ログに書くのは HTTP ステータスだけ。
pub enum SendResult {
    Sent(String),
    Failed(String),
    DryRunNoSecret,
}

fn fnox_get(name: &str, project: &Path) -> Option<String> {
    let cfg = home().join(NOTIFY_FNOX);
    let cfg = cfg.to_string_lossy();
    run("fnox", &["-c", &cfg, "get", name], project)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 送信本文の一時ファイルの置き場所。状態ディレクトリの下の、所有者だけが入れる（0700）ディレクトリ。
/// 共有の /tmp を使わないので、他のユーザーが同じ名前のファイルやシンボリックリンクを先に置けない。
fn private_body_path() -> std::io::Result<PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    let dir = state_path().with_file_name("agent-watch-tmp");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
    let path = dir.join(format!("body-{}.json", std::process::id()));
    // 前回の異常終了で残ったものは消してから作る（create_new が失敗しないように）
    let _ = fs::remove_file(&path);
    Ok(path)
}

fn curl_quote(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// payload を webhook に送る。URL と鍵は curl の設定として標準入力から渡す（引数・プロセス一覧に出さない）。
/// 本文は所有者だけが読める一時ファイルに置き、送ったら消す。
pub fn send(payload: &Value, project: &Path) -> SendResult {
    // 受け手が一時的に応答しないことがあるので、失敗したら 2 秒おいて最大 2 回まで送り直す
    let mut result = send_once(payload, project);
    for _ in 0..2 {
        if !matches!(result, SendResult::Failed(_)) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
        result = send_once(payload, project);
    }
    result
}

fn send_once(payload: &Value, project: &Path) -> SendResult {
    let (Some(url), Some(key)) = (
        fnox_get("PELWAY_NOTIFY_WEBHOOK_URL", project),
        fnox_get("PELWAY_NOTIFY_WEBHOOK_KEY", project),
    ) else {
        return SendResult::DryRunNoSecret;
    };
    let body_path = match private_body_path() {
        Ok(p) => p,
        Err(e) => return SendResult::Failed(format!("tmpfile: {e}")),
    };
    // 所有者だけが入れるディレクトリの中に、既存のファイルを開かない形（create_new）で作る
    let written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&body_path)
        .and_then(|mut f| f.write_all(payload.to_string().as_bytes()));
    if let Err(e) = written {
        let _ = fs::remove_file(&body_path);
        return SendResult::Failed(format!("tmpfile: {e}"));
    }
    let config = format!(
        "url = \"{}\"\nheader = \"Authorization: Bearer {}\"\n",
        curl_quote(&url),
        curl_quote(&key)
    );
    let data = format!("@{}", body_path.display());
    let result = Command::new("curl")
        .args([
            "-s",
            "--connect-timeout",
            "2",
            "--max-time",
            "10",
            "-X",
            "POST",
        ])
        .args([
            "-H",
            "Content-Type: application/json",
            "--data-binary",
            &data,
        ])
        .args(["-o", "/dev/null", "-w", "%{http_code}", "-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(config.as_bytes())?;
            }
            child.wait_with_output()
        });
    let _ = fs::remove_file(&body_path);
    match result {
        Ok(out) => {
            let code = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if code.len() == 3 && code.starts_with('2') {
                SendResult::Sent(code)
            } else {
                // curl の終了コードだけを残す（エラー文には URL のホスト名が入りうるので出さない）
                let exit = out
                    .status
                    .code()
                    .map_or_else(|| "signal".to_string(), |c| c.to_string());
                SendResult::Failed(format!(
                    "http={} curl_exit={exit}",
                    if code.is_empty() { "000" } else { &code }
                ))
            }
        }
        Err(e) => SendResult::Failed(format!("curl: {e}")),
    }
}
