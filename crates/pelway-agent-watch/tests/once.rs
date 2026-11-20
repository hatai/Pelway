//! `pelway-agent-watch --once --dry-run` の結合テスト。
//! 偽の `t3` / `gh` を一時ディレクトリに置いて PATH の先頭にし、状態ファイルは XDG_STATE_HOME で一時ディレクトリに向ける。

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("agent-watch-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join("fake")).unwrap();
        fs::create_dir_all(root.join("state/pelway")).unwrap();
        // 偽の t3: list は fake/list.json、show は fake/show-<id の : を _ にしたもの>.json を返す。
        // fake/t3-fail があれば終了コード 1。
        write_exec(
            &root.join("bin/t3"),
            r#"#!/usr/bin/env bash
F="$FAKE_DIR"
echo "t3 $*" >> "$F/t3-calls"
[ -e "$F/t3-fail" ] && exit 1
case "$1 $2" in
  "thread list") cat "$F/list.json" ;;
  "thread show") cat "$F/show-${3//:/_}.json" ;;
  *) exit 2 ;;
esac
"#,
        );
        // 偽の gh: 開いている PR はない。
        write_exec(&root.join("bin/gh"), "#!/usr/bin/env bash\necho '[]'\n");
        // 偽の fnox: 秘密は取れない（本物の webhook に送らないため。送信は「送れなかった」扱いになる）
        write_exec(&root.join("bin/fnox"), "#!/usr/bin/env bash\nexit 1\n");
        // 偽の git: rev-parse の --git-dir / --git-common-dir は fake/git-dir・fake/git-common-dir の
        // 中身を返す。無ければ両方 ".git"（本体 checkout）。worktree は中身を変えて模す。
        write_exec(
            &root.join("bin/git"),
            r#"#!/usr/bin/env bash
F="$FAKE_DIR"
if [ "$1" != "rev-parse" ]; then exit 2; fi
shift
while [ $# -gt 0 ]; do
  case "$1" in
    --git-dir) cat "$F/git-dir" 2>/dev/null || echo ".git" ;;
    --git-common-dir) cat "$F/git-common-dir" 2>/dev/null || echo ".git" ;;
  esac
  shift
done
"#,
        );
        Sandbox { root }
    }

    fn fake(&self, name: &str, body: &str) {
        fs::write(self.root.join("fake").join(name), body).unwrap();
    }

    fn state_file(&self) -> PathBuf {
        self.root.join("state/pelway/agent-watch.json")
    }

    fn lock_file(&self) -> PathBuf {
        self.root.join("state/pelway/agent-watch.lock")
    }

    /// 偽 t3 の呼び出し記録。見張りが動けば必ず `t3 thread list` が呼ばれるので、
    /// 記録が無いことは「見張りが動かなかった」ことの確認になる。
    fn t3_calls(&self) -> PathBuf {
        self.root.join("fake/t3-calls")
    }

    fn command(&self, args: &[&str]) -> Command {
        let path = format!(
            "{}:{}",
            self.root.join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_pelway-agent-watch"));
        cmd.args(args)
            .args(["--project", "."])
            .env("PELWAY_AGENT_WATCH_POLL_SECS", "0")
            .env("PATH", path)
            .env("FAKE_DIR", self.root.join("fake"))
            .env("XDG_STATE_HOME", self.root.join("state"));
        cmd
    }

    fn run(&self) -> Output {
        self.run_with(&["--once", "--dry-run"])
    }

    fn run_with(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_exec(path: &Path, body: &str) {
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// 子プロセスを drop 時に必ず止める（テストが panic しても残らないように）。
struct KillOnDrop(std::process::Child);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// `/proc/locks` に `path` の lock が載っているか調べる（lock 自体は取らない）。
/// `flock -n <path> true` で観測すると一瞬 lock を取ってしまい、観測対象の 1 回きりの
/// `flock -n` 試行と衝突して相手を失敗させ得る（CI で発生）ので、読むだけにする。
fn lock_is_held(path: &Path) -> bool {
    let ino = fs::metadata(path).map(|m| m.ino()).unwrap_or(0);
    if ino == 0 {
        return false;
    }
    let Ok(locks) = fs::read_to_string("/proc/locks") else {
        return false;
    };
    let ino = ino.to_string();
    locks.lines().any(|line| {
        line.split_whitespace()
            .nth(5)
            .is_some_and(|id| id.rsplit(':').next() == Some(ino.as_str()))
    })
}

/// lock ファイルが誰かに取られるまで待つ。保持側が lock を取る前に死んだら即座に失敗させる。
fn wait_locked(path: &Path, holder: &mut std::process::Child) {
    for _ in 0..1000 {
        if lock_is_held(path) {
            return;
        }
        if let Ok(Some(status)) = holder.try_wait() {
            panic!("lock holder exited with {status}: {}", path.display());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("lock was never taken: {}", path.display());
}

const LIST_DONE: &str = r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:b","status":"completed","updatedAt":"2026-11-10T16:00:00Z"}]"#;

const SHOW_A_DONE: &str = r#"{"thread":{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z",
  "branch":"agent/80-x","providerInstanceId":"opencode","model":"m","runCount":1,"pendingRequestCount":0,"link":"l"},
  "messages":[{"role":"assistant","text":"issue: #80\npr: https://github.com/hatai/Pelway/pull/81\nverify: pass\npr-size: 66 / 800"}]}"#;

const STATE_A_RUNNING: &str = r#"{"initialized":true,"threads":{
  "mcp:a":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:b":{"status":"completed","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null}},
  "ci_notified":{}}"#;

#[test]
fn once_first_run_prints_nothing() {
    let sb = Sandbox::new("first");
    sb.fake("list.json", LIST_DONE);
    let out = sb.run_with(&["--once"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stdout.is_empty());
    let state = fs::read_to_string(sb.state_file()).unwrap();
    assert!(state.contains("mcp:a") && state.contains("mcp:b"));
}

#[test]
fn once_emits_finished_with_report() {
    let sb = Sandbox::new("finished");
    sb.fake("list.json", LIST_DONE);
    sb.fake("show-mcp_a.json", SHOW_A_DONE);
    fs::write(sb.state_file(), STATE_A_RUNNING).unwrap();
    let out = sb.run();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "stdout: {stdout}");
    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["event"], "finished");
    assert_eq!(v["issue_number"], 80);
    assert!(v["report"].as_str().unwrap().starts_with("issue: #80"));
}

#[test]
fn once_t3_failure_keeps_state() {
    let sb = Sandbox::new("t3fail");
    fs::write(sb.state_file(), STATE_A_RUNNING).unwrap();
    sb.fake("t3-fail", "");
    let out = sb.run();
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    assert_eq!(
        fs::read_to_string(sb.state_file()).unwrap(),
        STATE_A_RUNNING
    );
}

#[test]
fn once_corrupt_state_is_baseline() {
    let sb = Sandbox::new("corrupt");
    sb.fake("list.json", LIST_DONE);
    fs::write(sb.state_file(), "{").unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    assert_eq!(state["initialized"], true);
}

#[test]
fn dry_run_does_not_touch_state() {
    let sb = Sandbox::new("dryrun-state");
    sb.fake("list.json", LIST_DONE);
    sb.fake("show-mcp_a.json", SHOW_A_DONE);
    fs::write(sb.state_file(), STATE_A_RUNNING).unwrap();
    let out = sb.run();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 1);
    assert_eq!(
        fs::read_to_string(sb.state_file()).unwrap(),
        STATE_A_RUNNING
    );
}

#[test]
fn unsent_payload_is_kept_in_outbox() {
    let sb = Sandbox::new("outbox");
    sb.fake("list.json", LIST_DONE);
    sb.fake("show-mcp_a.json", SHOW_A_DONE);
    fs::write(sb.state_file(), STATE_A_RUNNING).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    let outbox = state["outbox"].as_array().unwrap();
    assert_eq!(outbox.len(), 1);
    assert_eq!(outbox[0]["event"], "finished");
    assert_eq!(state["threads"]["mcp:a"]["status"], "completed");
}

#[test]
fn unwritable_state_does_not_repeat_events() {
    let sb = Sandbox::new("readonly");
    sb.fake("list.json", LIST_DONE);
    sb.fake("show-mcp_a.json", SHOW_A_DONE);
    fs::write(sb.state_file(), STATE_A_RUNNING).unwrap();
    // lock ファイルは起動時に作られるので、書き込めないディレクトリの中に先に置いておく
    fs::write(sb.lock_file(), "").unwrap();
    let dir = sb.state_file().parent().unwrap().to_path_buf();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o555)).unwrap();
    let out = sb.run_with(&["--cycles", "3"]);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        stderr.matches("queued event=finished").count(),
        1,
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("ERROR cannot save state"),
        "stderr: {stderr}"
    );
}

#[test]
fn state_file_is_owner_only() {
    // 状態ファイルには送信待ちの payload（報告行を含む）が入るので、所有者だけが読めるようにする
    let sb = Sandbox::new("mode");
    sb.fake("list.json", LIST_DONE);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success());
    let mode = fs::metadata(sb.state_file()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "mode {mode:o}");
}

#[test]
fn worktree_checkout_is_refused() {
    // --git-dir と --git-common-dir が違う（linked worktree）ところからは起動しない。
    // 拒否される側は --once（dry-run なし）で起動する: 拒否されずに動けば初回の周回で
    // 状態ファイルが作られ、偽 t3 が呼ばれる（--dry-run だと動いても両方とも起きず、
    // 「拒否された」を確かめられない）
    let sb = Sandbox::new("worktree");
    sb.fake("list.json", LIST_DONE);
    sb.fake("git-dir", "/repo/.git/worktrees/wt\n");
    sb.fake("git-common-dir", "/repo/.git\n");
    let out = sb.run_with(&["--once"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("main checkout"), "stderr: {stderr}");
    assert!(
        out.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!sb.state_file().exists(), "state file was written");
    assert!(!sb.t3_calls().exists(), "fake t3 was called");
}

#[test]
fn second_instance_is_refused() {
    // 起動中の agent-watch が lock を持ち続けているあいだ、2 つ目は起動しない。
    // 1 つ目は本物のバイナリをループで動かす（lock はプロセスが生きているあいだ有効）
    let sb = Sandbox::new("lock");
    sb.fake("list.json", LIST_DONE);
    let mut cmd = sb.command(&["--dry-run", "--cycles", "1000"]);
    cmd.env("PELWAY_AGENT_WATCH_POLL_SECS", "30")
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut first = KillOnDrop(cmd.spawn().unwrap());
    wait_locked(&sb.lock_file(), &mut first.0);
    // 2 つ目は --once（dry-run なし）で起動する。拒否されずに動けば状態ファイルが
    // 作られる（1 つ目は --dry-run なので作らず、判定を汚さない）
    let out = sb.run_with(&["--once"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("lock"), "stderr: {stderr}");
    assert!(
        out.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!sb.state_file().exists(), "state file was written");
}

#[test]
fn externally_held_lock_is_refused() {
    // 状態ディレクトリの lock を別プロセスが取っているだけでも起動しない
    let sb = Sandbox::new("flock");
    sb.fake("list.json", LIST_DONE);
    let mut hold = KillOnDrop(
        Command::new("flock")
            .arg("-n")
            .arg(sb.lock_file())
            .args(["sleep", "30"])
            .spawn()
            .unwrap(),
    );
    wait_locked(&sb.lock_file(), &mut hold.0);
    let out = sb.run_with(&["--once"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("lock"), "stderr: {stderr}");
    assert!(
        out.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(!sb.state_file().exists(), "state file was written");
    assert!(!sb.t3_calls().exists(), "fake t3 was called");
}

#[test]
fn main_checkout_starts_and_lock_is_owner_only() {
    // 本体 checkout では普通に起動し、lock ファイルは所有者だけが読み書きできる
    let sb = Sandbox::new("main");
    sb.fake("list.json", LIST_DONE);
    let out = sb.run_with(&["--once"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("refusing"), "stderr: {stderr}");
    assert!(sb.state_file().exists(), "state file was not written");
    let mode = fs::metadata(sb.lock_file()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600, "mode {mode:o}");
}
