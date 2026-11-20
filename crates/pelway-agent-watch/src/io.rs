//! 外部コマンド（t3・gh・fnox・curl）とファイルの読み書き。
//! 秘密（webhook の URL と鍵）は引数・ログに出さない。

use std::fs;
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use pelway_agent_watch::ci::{AgentPr, RunInfo, parse_agent_prs};
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
pub fn open_agent_prs(project: &Path) -> Result<Vec<AgentPr>, String> {
    let out = run(
        "gh",
        &[
            "pr",
            "list",
            "--state",
            "open",
            "--json",
            "headRefName,headRefOid,isCrossRepository,commits",
        ],
        project,
    )?;
    parse_agent_prs(&out)
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

/// `--git-dir`・`--git-common-dir`・`--show-toplevel`（絶対パス）。
/// 前二者が等しければ本体 checkout、違えば linked worktree（agent-worktree-prune と同じ判定）。
/// toplevel は `--project` が起動元の checkout を指すかの照合に使う。
pub fn git_dirs(project: &Path) -> Result<(String, String, String), String> {
    let out = run(
        "git",
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
            "--show-toplevel",
        ],
        project,
    )?;
    let mut lines = out.lines();
    let (Some(dir), Some(common), Some(toplevel)) = (lines.next(), lines.next(), lines.next())
    else {
        return Err("git rev-parse: unexpected output".to_string());
    };
    Ok((dir.to_string(), common.to_string(), toplevel.to_string()))
}

/// 状態ディレクトリの lock ファイル。二重起動の判定に使う。
pub fn lock_path() -> PathBuf {
    state_path().with_file_name("agent-watch.lock")
}

/// lock ファイル（0600）を flock（LOCK_EX | LOCK_NB）で取る。
/// 取れたら File を返す（持っている間ロックが効く）。別プロセスが持っていれば Ok(None)。
pub fn try_lock(path: &Path) -> Result<Option<fs::File>, String> {
    use std::os::unix::io::AsRawFd;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|e| e.to_string())?;
    // create(true) では既存ファイルの権限が変わらないので、0600 に揃え直す
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc == 0 {
        return Ok(Some(file));
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
        return Ok(None);
    }
    Err(err.to_string())
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
    // 送信待ちの payload（報告行を含む）が入るので、所有者だけが読み書きできる 0600 で書く。
    // 前回の一時ファイルが残っていれば消し、create_new で作り直す（既存の権限を引き継がない）
    let _ = fs::remove_file(&tmp);
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)
        .and_then(|mut f| f.write_all(body.as_bytes()))
        .map_err(|e| e.to_string())?;
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

/// 共有の git dir の `pelway/` は sandbox の中のエージェントも書ける場所なので、
/// agent-watch（ホストの権限で動く）は、置かれたものの種類を確かめてから使う。
/// `pelway/` を `O_DIRECTORY | O_NOFOLLOW` で 1 回開き、その fd で種類と所有者を確かめ、
/// 中のファイルはその fd からの `openat` で開く。パスで確かめてからパスで開くと、
/// その間に `pelway/` を別のディレクトリへの symlink に差し替えられるため。
fn open_pelway_dir(dir: &Path, create: bool) -> Result<fs::File, String> {
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};
    use std::os::unix::io::FromRawFd;
    if create && fs::symlink_metadata(dir).is_err() {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(dir)
            .map_err(|e| e.to_string())?;
    }
    let c = std::ffi::CString::new(dir.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
    let fd = unsafe {
        libc::open(
            c.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(format!(
            "pelway dir: {}",
            std::io::Error::last_os_error().kind()
        ));
    }
    // SAFETY: fd は直前の open が返した、ほかに持ち主の無い有効な fd
    let dirf = unsafe { fs::File::from_raw_fd(fd) };
    let meta = dirf.metadata().map_err(|e| e.to_string())?;
    if !meta.file_type().is_dir() {
        return Err("pelway dir is not a plain directory".to_string());
    }
    if meta.uid() != unsafe { libc::geteuid() } {
        return Err("pelway dir is not owned by this user".to_string());
    }
    Ok(dirf)
}

/// `pelway/` の fd から、symlink をたどらず（`O_NOFOLLOW`）、FIFO で止まらず（`O_NONBLOCK`）に開き、
/// 開いた fd で、通常ファイル・自分の所有・ハードリンクが 1 本だけ、を確かめる
/// （`scripts/conductor-watch.sh` のボード Status 更新キューと同じ型）。
fn open_in_pelway(dirf: &fs::File, name: &str, flags: libc::c_int) -> Result<fs::File, String> {
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::{AsRawFd, FromRawFd};
    let c = std::ffi::CString::new(name).map_err(|e| e.to_string())?;
    let fd = unsafe {
        libc::openat(
            dirf.as_raw_fd(),
            c.as_ptr(),
            flags | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            0o600 as libc::c_uint,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // SAFETY: fd は直前の openat が返した、ほかに持ち主の無い有効な fd
    let file = unsafe { fs::File::from_raw_fd(fd) };
    let meta = file.metadata().map_err(|e| e.to_string())?;
    if !meta.file_type().is_file() {
        return Err("not a regular file".to_string());
    }
    if meta.uid() != unsafe { libc::geteuid() } {
        return Err("not owned by this user".to_string());
    }
    if meta.nlink() != 1 {
        return Err("has other hard links".to_string());
    }
    Ok(file)
}

fn split_pelway_path(path: &Path) -> Result<(&Path, &str), String> {
    let dir = path.parent().ok_or("no parent dir")?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("bad file name")?;
    Ok((dir, name))
}

/// 受信箱の大きさの上限。切り詰めない設計でも数年分は収まる大きさ（1 件は数百バイト）。
/// 超えたもの（sandbox の中から置かれた巨大な sparse ファイルなど）には書かず、数えない。
const INBOX_MAX_BYTES: u64 = 256 * 1024 * 1024;

/// 受信箱に、1 件 1 行で追記する（ファイルは 0600。1 行を 1 回の書き込みで書く）。
/// 書いたあとの受信箱の行数を、同じ fd で数えて返す（path を開き直さない）。
pub fn append_inbox(path: &Path, lines: &[String]) -> Result<u64, String> {
    use std::io::{Read, Seek};
    let (dir, name) = split_pelway_path(path)?;
    let dirf = open_pelway_dir(dir, true)?;
    let mut file = open_in_pelway(&dirf, name, libc::O_RDWR | libc::O_APPEND | libc::O_CREAT)?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    if len > INBOX_MAX_BYTES {
        return Err("inbox is too large".to_string());
    }
    // 前回の追記が途中で失敗して改行の無い断片が残っていれば、先に改行で閉じる
    // （次の知らせを独立した 1 行として読めるようにする）
    if len > 0 {
        use std::os::unix::fs::FileExt;
        let mut last = [0u8; 1];
        file.read_exact_at(&mut last, len - 1)
            .map_err(|e| e.to_string())?;
        if last[0] != b'\n' {
            file.write_all(b"\n").map_err(|e| e.to_string())?;
        }
    }
    for line in lines {
        let mut buf = line.clone();
        buf.push('\n');
        file.write_all(buf.as_bytes()).map_err(|e| e.to_string())?;
    }
    file.rewind().map_err(|e| e.to_string())?;
    // 書いたあとの大きさまでしか読まない（書いている間に伸ばされても、上限の内で止める）
    let mut file = file.take(INBOX_MAX_BYTES + 1024 * 1024);
    let mut count = 0u64;
    let mut chunk = [0u8; 8192];
    loop {
        let n = file.read(&mut chunk).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        count += chunk[..n].iter().filter(|&&b| b == b'\n').count() as u64;
    }
    Ok(count)
}

/// 補助記録（`patrol-state.json`）の大きさの上限。これを超えるものは読まない。
const PATROL_STATE_MAX_BYTES: u64 = 256 * 1024;

/// 指揮役の補助記録（`patrol-state.json`）を、種類と大きさを確かめてから読む。
pub fn read_patrol_state(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let (dir, name) = split_pelway_path(path)?;
    let dirf = open_pelway_dir(dir, false)?;
    let file = open_in_pelway(&dirf, name, libc::O_RDONLY)?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    if size > PATROL_STATE_MAX_BYTES {
        return Err("patrol-state is too large".to_string());
    }
    let mut text = String::new();
    file.take(PATROL_STATE_MAX_BYTES)
        .read_to_string(&mut text)
        .map_err(|e| e.to_string())?;
    Ok(text)
}

/// いまの指揮役のスレッド ID を登録するホスト側のファイル。sandbox の中からは書けない場所
/// （webhook の秘密の設定と同じ `~/.config/pelway-agent-notify/`）。代行の開始で hatai が書き、終了で消す。
pub fn conductor_file() -> PathBuf {
    home().join(".config/pelway-agent-notify/conductor-thread-id")
}

/// 登録ファイルを読む。無ければ Ok(None)（指揮役が T3 のスレッドとして動いていない）。
pub fn read_conductor_file(path: &Path) -> Result<Option<String>, String> {
    use std::io::Read;
    match fs::File::open(path) {
        Ok(f) => {
            let mut text = String::new();
            f.take(4096)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            Ok(Some(text))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// 起こしに使う `t3` の実行ファイル。`T3_BIN` が指定されていればそれ、無ければ動いている
/// T3 サーバーと同じ実行ファイル、それも無ければ PATH の `t3`。
/// PATH の `t3` は版が同じでも、サーバーが持つ `~/.t3/userdata` を `ServerOwnsDatabaseError` で
/// 拒まれることがあるため（`scripts/conductor-watch.sh` の `T3_BIN` の解決と同じ考え方）。
pub fn wake_t3_bin() -> PathBuf {
    if let Some(bin) = std::env::var_os("T3_BIN").filter(|v| !v.is_empty()) {
        return PathBuf::from(bin);
    }
    find_server_t3(Path::new("/proc"), &home()).unwrap_or_else(|| PathBuf::from("t3"))
}

/// 動いている T3 サーバーの実行ファイルを `<proc>/<pid>/exe` から探す。
/// agent-watch はホストの権限で実行するので、受け入れるのは `<home>/.t3/wsl-runtime/` の下の
/// `t3` で、自分の所有の通常ファイルだけ（sandbox の中から書ける場所に置かれた偽物を選ばない）。
fn find_server_t3(proc: &Path, home: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::MetadataExt;
    let runtime = home.join(".t3/wsl-runtime");
    let me = unsafe { libc::geteuid() };
    let entries = fs::read_dir(proc).ok()?;
    for entry in entries.flatten() {
        if !entry
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        let Ok(link) = fs::read_link(entry.path().join("exe")) else {
            continue;
        };
        if !link.starts_with(&runtime) || link.file_name().is_none_or(|n| n != "t3") {
            continue;
        }
        if link
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            continue;
        }
        let Ok(meta) = fs::symlink_metadata(&link) else {
            continue;
        };
        if meta.file_type().is_file() && meta.uid() == me {
            return Some(link);
        }
    }
    None
}

/// 指揮役のスレッドへ決まった文を送る（`t3 thread send <thread> -`。本文は標準入力）。
/// 実行中のスレッドには次のターンとして届き、割り込まない。ログに出すのは終了コードだけ。
pub fn wake(bin: &Path, thread_id: &str, message: &str) -> Result<(), String> {
    let child = Command::new("timeout")
        .arg(COMMAND_TIMEOUT_SECS)
        .arg(bin)
        .args(["thread", "send", thread_id, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn: {}", e.kind()))?;
    let mut child = child;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(message.as_bytes())
            .map_err(|e| format!("stdin: {}", e.kind()))?;
    }
    let status = child.wait().map_err(|e| format!("wait: {}", e.kind()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "exit={}",
            status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string())
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    /// 偽の /proc と HOME を作り、`<pid>/exe` の向き先ごとに選ばれるかを確かめる。
    #[test]
    fn server_t3_only_under_home_runtime() {
        let root = std::env::temp_dir().join(format!("agent-watch-t3bin-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let home = root.join("home");
        let proc_dir = root.join("proc");
        // sandbox の中から書ける場所に置いた偽物（パスに /.t3/wsl-runtime/ を含むが HOME の下ではない）
        let fake = root.join("tmp/.t3/wsl-runtime/a/t3");
        // worktree の中に置いた偽物（HOME の下だが ~/.t3/wsl-runtime/ ではない）
        let wt_fake = home.join(".t3/worktrees/Pelway/x/.t3/wsl-runtime/a/t3");
        let real = home.join(".t3/wsl-runtime/b/t3");
        for f in [&fake, &wt_fake, &real] {
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, "").unwrap();
        }
        for (pid, target) in [("1", &fake), ("2", &wt_fake), ("3", &real)] {
            fs::create_dir_all(proc_dir.join(pid)).unwrap();
            symlink(target, proc_dir.join(pid).join("exe")).unwrap();
        }
        assert_eq!(find_server_t3(&proc_dir, &home), Some(real.clone()));

        // 本物が無ければ、偽物は選ばず None（PATH の t3 に落ちる）
        fs::remove_file(proc_dir.join("3/exe")).unwrap();
        assert_eq!(find_server_t3(&proc_dir, &home), None);
        let _ = fs::remove_dir_all(&root);
    }
}
