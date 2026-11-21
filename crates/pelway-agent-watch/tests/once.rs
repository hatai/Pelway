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
        // 共有の git dir の代わり。偽の git は既定で git-dir と git-common-dir にここを返す
        // （本体 checkout 扱い。受信箱と補助記録は gitcommon/pelway/ の下に置かれる）
        fs::create_dir_all(root.join("gitcommon")).unwrap();
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
  "thread send")
    [ -e "$F/t3-send-fail" ] && exit 1
    echo "$3 $4" >> "$F/t3-send-args"
    cat >> "$F/t3-send-stdin" ;;
  *) exit 2 ;;
esac
"#,
        );
        // 偽の gh: 開いている PR はない。
        write_exec(&root.join("bin/gh"), "#!/usr/bin/env bash\necho '[]'\n");
        // 偽の fnox: 秘密は取れない（本物の webhook に送らないため。送信は「送れなかった」扱いになる）
        // fake/fnox-ok があるときだけ偽の値を返す（送信が失敗する場合の試験に使う）
        write_exec(
            &root.join("bin/fnox"),
            "#!/usr/bin/env bash\n[ -e \"$FAKE_DIR/fnox-ok\" ] && { echo fake-secret; exit 0; }\nexit 1\n",
        );
        // 偽の curl: 本文（--data-binary @<file>）を fake/curl-body に写し、HTTP 500 を返す
        write_exec(
            &root.join("bin/curl"),
            r#"#!/usr/bin/env bash
cat > /dev/null
while [ $# -gt 0 ]; do
  if [ "$1" = "--data-binary" ]; then cp "${2#@}" "$FAKE_DIR/curl-body"; fi
  shift
done
printf 500
"#,
        );
        // 偽の git: rev-parse の --git-dir / --git-common-dir / --show-toplevel は、
        // rev-parse の実行先（$PWD）に同名ファイルがあればその中身、無ければ fake/ の同名
        // ファイル、それも無ければ ".git"・".git"・`pwd -P`（本体 checkout）を返す。
        write_exec(
            &root.join("bin/git"),
            r#"#!/usr/bin/env bash
F="$FAKE_DIR"
if [ "$1" != "rev-parse" ]; then exit 2; fi
shift
GD="$F/git-dir"; GC="$F/git-common-dir"; TL="$F/toplevel"
[ -f "$PWD/git-dir" ] && GD="$PWD/git-dir"
[ -f "$PWD/git-common-dir" ] && GC="$PWD/git-common-dir"
[ -f "$PWD/toplevel" ] && TL="$PWD/toplevel"
while [ $# -gt 0 ]; do
  case "$1" in
    --git-dir) cat "$GD" 2>/dev/null || echo "$F/../gitcommon" ;;
    --git-common-dir) cat "$GC" 2>/dev/null || echo "$F/../gitcommon" ;;
    --show-toplevel) cat "$TL" 2>/dev/null || pwd -P ;;
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

    fn pelway_dir(&self) -> PathBuf {
        self.root.join("gitcommon/pelway")
    }

    fn inbox(&self) -> PathBuf {
        self.pelway_dir().join("conductor-inbox.jsonl")
    }

    fn patrol_state(&self, body: &str) {
        fs::create_dir_all(self.pelway_dir()).unwrap();
        fs::write(self.pelway_dir().join("patrol-state.json"), body).unwrap();
    }

    /// ホスト側の登録ファイルに、いまの指揮役のスレッド ID を書く（`pelway-agent-watch --register` の代わり）。
    fn conductor(&self, id: &str) {
        let dir = self.root.join("home/.config/pelway-agent-notify");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("conductor-thread-id"), format!("{id}\n")).unwrap();
    }

    fn read_fake(&self, name: &str) -> String {
        fs::read_to_string(self.root.join("fake").join(name)).unwrap_or_default()
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
            // 起こしに使う t3 は偽物に向ける（この PC で動いている本物の T3 サーバーを呼ばない）
            .env("T3_BIN", self.root.join("bin/t3"))
            // 指揮役の登録ファイル（~/.config/pelway-agent-notify/conductor-thread-id）の置き場
            .env("HOME", self.root.join("home"))
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
fn worktree_launch_dir_is_refused_for_main_project() {
    // 起動元（cwd）が linked worktree なら、--project に本体 checkout を指しても起動しない。
    // ガードが見張り対象ではなく起動元を見ることの確認
    let sb = Sandbox::new("wtlaunch");
    sb.fake("list.json", LIST_DONE);
    let wt = sb.root.join("wt-checkout");
    let main_dir = sb.root.join("main-checkout");
    fs::create_dir_all(&wt).unwrap();
    fs::create_dir_all(&main_dir).unwrap();
    // wt-checkout は worktree（rev-parse が違う値を返す）。main-checkout はマーカー無しで本体
    fs::write(wt.join("git-dir"), "/repo/.git/worktrees/wt\n").unwrap();
    fs::write(wt.join("git-common-dir"), "/repo/.git\n").unwrap();
    let mut cmd = sb.command(&["--once"]);
    cmd.current_dir(&wt)
        .args(["--project", main_dir.to_str().unwrap()]);
    let out = cmd.output().unwrap();
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
fn main_launch_with_worktree_project_is_refused() {
    // 本体 checkout から起動しても、--project が起動元の toplevel と違うなら拒否する。
    // 本体から別の checkout を見張る起動は本体の見張りとの重複・すり替わりになるため
    let sb = Sandbox::new("wtproj");
    sb.fake("list.json", LIST_DONE);
    let main_dir = sb.root.join("main-checkout");
    let wt = sb.root.join("wt-checkout");
    fs::create_dir_all(&main_dir).unwrap();
    fs::create_dir_all(&wt).unwrap();
    // main-checkout は本体（同じ値を返す）。toplevel マーカーは置かず pwd -P に任せる
    fs::write(main_dir.join("git-dir"), "/repo/.git\n").unwrap();
    fs::write(main_dir.join("git-common-dir"), "/repo/.git\n").unwrap();
    let mut cmd = sb.command(&["--once"]);
    cmd.current_dir(&main_dir)
        .args(["--project", wt.to_str().unwrap()]);
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--project"), "stderr: {stderr}");
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

// ---- 指揮役の受信箱と起こし（Issue #555） ----

/// 5 種の知らせ（finished・failed・cancelled・waiting・stalled）がそろう一覧と詳細。
/// d は承認待ち（waiting）で、更新が古いので stalled も出る。e は更新が古いまま実行中（stalled）。
const LIST_FIVE: &str = r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:b","status":"failed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:c","status":"cancelled","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:d","status":"running","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:e","status":"running","updatedAt":"2026-11-10T16:00:00Z"}]"#;

const STATE_FIVE_RUNNING: &str = r#"{"initialized":true,"threads":{
  "mcp:a":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:b":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:c":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:d":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:e":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null}},
  "ci_notified":{}}"#;

fn show(id: &str, status: &str, updated: &str, pending: u32) -> String {
    format!(
        r#"{{"thread":{{"threadId":"{id}","status":"{status}","updatedAt":"{updated}","branch":"agent/80-x",
  "providerInstanceId":"opencode","model":"m","runCount":1,"pendingRequestCount":{pending},"link":"l"}},
  "messages":[{{"role":"assistant","text":"state: working\nhead: none\nnext: 手順 4"}}]}}"#
    )
}

/// 5 種の知らせがそろう状況を作る。
fn five_events(sb: &Sandbox) {
    sb.fake("list.json", LIST_FIVE);
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "completed", "2026-11-10T16:10:00Z", 0),
    );
    sb.fake(
        "show-mcp_b.json",
        &show("mcp:b", "failed", "2026-11-10T16:10:00Z", 0),
    );
    sb.fake(
        "show-mcp_c.json",
        &show("mcp:c", "cancelled", "2026-11-10T16:10:00Z", 0),
    );
    sb.fake(
        "show-mcp_d.json",
        &show("mcp:d", "running", "2026-11-10T16:10:00Z", 1),
    );
    sb.fake(
        "show-mcp_e.json",
        &show("mcp:e", "running", "2026-11-10T16:00:00Z", 0),
    );
    fs::write(sb.state_file(), STATE_FIVE_RUNNING).unwrap();
}

/// 1 件の知らせ（a の finished）が出る状況を作る。
fn one_event(sb: &Sandbox) {
    sb.fake("list.json", LIST_DONE);
    sb.fake("show-mcp_a.json", SHOW_A_DONE);
    fs::write(sb.state_file(), STATE_A_RUNNING).unwrap();
}

fn inbox_events(sb: &Sandbox) -> Vec<serde_json::Value> {
    fs::read_to_string(sb.inbox())
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).expect("each inbox line is one JSON value"))
        .collect()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).to_string()
}

#[test]
fn inbox_appends_every_event() {
    let sb = Sandbox::new("inbox-every");
    five_events(&sb);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let events = inbox_events(&sb);
    let kinds: std::collections::BTreeSet<String> = events
        .iter()
        .map(|e| e["event"].as_str().unwrap().to_string())
        .collect();
    for k in ["finished", "failed", "cancelled", "waiting", "stalled"] {
        assert!(kinds.contains(k), "{k} is missing: {kinds:?}");
    }
    // webhook と同じ形（schema と report を含む）
    assert!(
        events
            .iter()
            .all(|e| e["schema"] == "pelway.agent-watch/v1")
    );
    let mode = fs::metadata(sb.inbox()).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn inbox_written_when_webhook_fails() {
    // 偽の秘密を返し、偽の curl が HTTP 500 を返す: 送信処理は呼ばれて失敗する
    let sb = Sandbox::new("inbox-webhook-fails");
    one_event(&sb);
    sb.fake("fnox-ok", "");
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("mode=send-failed"),
        "{}",
        stderr(&out)
    );
    let events = inbox_events(&sb);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "finished");
    // 送信処理に渡した JSON と、受信箱の 1 行が同じ
    let sent: serde_json::Value = serde_json::from_str(&sb.read_fake("curl-body")).unwrap();
    assert_eq!(sent, events[0]);
}

#[test]
fn inbox_failure_does_not_stop_watch() {
    // pelway の位置に通常のファイルを置いて、受信箱を作れなくする（権限ではなく構造で失敗させる）
    let sb = Sandbox::new("inbox-fails");
    one_event(&sb);
    // 偽の秘密と、本文を写して HTTP 500 を返す偽の curl で、送信処理まで進むことを確かめる
    sb.fake("fnox-ok", "");
    fs::write(sb.pelway_dir(), "not a directory").unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let err = stderr(&out);
    assert_eq!(err.matches("inbox append failed").count(), 1, "{err}");
    // webhook の送信（curl に知らせの JSON が渡る）と状態の保存は続く
    let sent: serde_json::Value = serde_json::from_str(&sb.read_fake("curl-body")).unwrap();
    assert_eq!(sent["event"], "finished");
    assert_eq!(sent["thread_id"], "mcp:a");
    assert!(err.contains("mode=send-failed"), "{err}");
    let state = fs::read_to_string(sb.state_file()).unwrap();
    assert!(state.contains("\"outbox\":[{"), "{state}");
}

#[test]
fn wake_sent_to_registered_conductor() {
    let sb = Sandbox::new("wake-sent");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
    assert_eq!(
        sb.read_fake("t3-send-stdin"),
        pelway_agent_watch::inbox::WAKE_MESSAGE
    );
    // 起こしの文に知らせの中身（report）を含めない
    assert!(!sb.read_fake("t3-send-stdin").contains("issue: #80"));
    assert!(stderr(&out).contains("WAKE sent"));
}

#[test]
fn no_wake_without_conductor() {
    let sb = Sandbox::new("wake-none");
    one_event(&sb);
    // 登録ファイルが無い
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(sb.read_fake("t3-send-args").is_empty());

    // sandbox から書ける補助記録に宛先を書いても使わない（宛先はホスト側の登録ファイルからだけ読む）
    let sb = Sandbox::new("wake-none-missing");
    one_event(&sb);
    sb.patrol_state(r#"{"conductor_thread_id":"mcp:planted","inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(sb.read_fake("t3-send-args").is_empty());
}

#[test]
fn no_repeat_wake_until_read() {
    let sb = Sandbox::new("wake-repeat");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(sb.read_fake("t3-send-args").lines().count(), 1);

    // 次の知らせ（b の finished）。指揮役はまだ受信箱を読んでいない（inbox_offset は 0 のまま）
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:b","status":"completed","updatedAt":"2026-11-10T16:20:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_b.json",
        &show("mcp:b", "completed", "2026-11-10T16:20:00Z", 0),
    );
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    state["threads"]["mcp:b"]["status"] = "running".into();
    fs::write(sb.state_file(), state.to_string()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 2);
    assert_eq!(sb.read_fake("t3-send-args").lines().count(), 1);

    // 指揮役が 1 行目まで読んだ（前回の起こしの時点まで既読が進んだ）あとの知らせでは、また起こす
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":1}"#);
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:b","status":"failed","updatedAt":"2026-11-10T16:30:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_b.json",
        &show("mcp:b", "failed", "2026-11-10T16:30:00Z", 0),
    );
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    state["threads"]["mcp:b"]["status"] = "running".into();
    fs::write(sb.state_file(), state.to_string()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(sb.read_fake("t3-send-args").lines().count(), 2);
}

#[test]
fn unreadable_patrol_state_skips_wake() {
    let sb = Sandbox::new("wake-unreadable");
    one_event(&sb);
    // 書きかけで途中までしかない補助記録
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_off"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(stderr(&out).contains("mode=not-sent"));
    assert!(sb.read_fake("t3-send-args").is_empty());
}

#[test]
fn wake_failure_is_logged_once() {
    let sb = Sandbox::new("wake-fails");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    sb.fake("t3-send-fail", "");
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let err = stderr(&out);
    assert_eq!(err.matches("WAKE failed").count(), 1, "{err}");
    // 内部で送り直さない: 失敗した呼び出しも記録する t3-calls で、thread send がちょうど 1 回
    let calls = fs::read_to_string(sb.t3_calls()).unwrap_or_default();
    assert_eq!(
        calls
            .lines()
            .filter(|l| l.starts_with("t3 thread send"))
            .count(),
        1,
        "{calls}"
    );
    // 失敗した起こしは記録しない（次の新しい知らせで改めて試みる）
    let state = fs::read_to_string(sb.state_file()).unwrap();
    assert!(state.contains("\"last_wake_inbox_offset\":null"), "{state}");
}

#[test]
fn invalid_conductor_thread_id_is_not_used() {
    let sb = Sandbox::new("wake-invalid-id");
    one_event(&sb);
    sb.conductor("--help");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(stderr(&out).contains("WAKE skipped reason=invalid-thread-id"));
}

#[test]
fn dry_run_writes_no_inbox_and_no_wake() {
    let sb = Sandbox::new("inbox-dry-run");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run();
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(!sb.inbox().exists());
    assert!(sb.read_fake("t3-send-args").is_empty());
}

// ---- 受信箱・補助記録の置き場は sandbox の中から書けるので、種類を確かめて開く（PR #561 round 1） ----

/// 10 秒以内に終わらなければ止めて失敗にする（FIFO で止まらないことを確かめるため）。
fn run_bounded(sb: &Sandbox) -> Output {
    let child = sb
        .command(&["--once"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = KillOnDrop(child);
    for _ in 0..1000 {
        if let Ok(Some(_)) = child.0.try_wait() {
            let child = std::mem::replace(&mut child.0, Command::new("true").spawn().unwrap());
            return child.wait_with_output().unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("agent-watch did not finish within 10 seconds");
}

fn mkfifo(path: &Path) {
    let st = Command::new("mkfifo").arg(path).status().unwrap();
    assert!(st.success());
}

#[test]
fn inbox_symlink_is_not_followed() {
    let sb = Sandbox::new("inbox-symlink");
    one_event(&sb);
    let target = sb.root.join("target.txt");
    fs::write(&target, "").unwrap();
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    std::os::unix::fs::symlink(&target, sb.inbox()).unwrap();
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(fs::read_to_string(&target).unwrap(), "");
    assert!(stderr(&out).contains("inbox append failed"));
}

#[test]
fn inbox_hard_link_is_not_written() {
    let sb = Sandbox::new("inbox-hardlink");
    one_event(&sb);
    let target = sb.root.join("target.txt");
    fs::write(&target, "").unwrap();
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    fs::hard_link(&target, sb.inbox()).unwrap();
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(fs::read_to_string(&target).unwrap(), "");
    assert!(stderr(&out).contains("inbox append failed"));
}

#[test]
fn inbox_fifo_does_not_block() {
    let sb = Sandbox::new("inbox-fifo");
    one_event(&sb);
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    mkfifo(&sb.inbox());
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("inbox append failed"));
    assert!(stderr(&out).contains("mode=not-sent"));
}

#[test]
fn pelway_dir_symlink_is_not_used() {
    let sb = Sandbox::new("pelway-symlink");
    one_event(&sb);
    let elsewhere = sb.root.join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, sb.pelway_dir()).unwrap();
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(fs::read_dir(&elsewhere).unwrap().count(), 0);
    assert!(stderr(&out).contains("inbox append failed"));
}

#[test]
fn patrol_state_fifo_does_not_block() {
    let sb = Sandbox::new("patrol-fifo");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    mkfifo(&sb.pelway_dir().join("patrol-state.json"));
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(stderr(&out).contains("WAKE skipped reason=patrol-state-unreadable"));
}

#[test]
fn huge_sparse_inbox_does_not_stall_the_watch() {
    // sandbox の中から置ける巨大な sparse ファイル。数えずに、その回は書かず、webhook の処理まで進む
    let sb = Sandbox::new("inbox-huge");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    let f = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(sb.inbox())
        .unwrap();
    fs::set_permissions(sb.inbox(), fs::Permissions::from_mode(0o600)).unwrap();
    f.set_len(1 << 40).unwrap();
    drop(f);
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let err = stderr(&out);
    assert!(
        err.contains("inbox append failed: inbox is too large"),
        "{err}"
    );
    assert!(err.contains("mode=not-sent"), "{err}");
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert_eq!(fs::metadata(sb.inbox()).unwrap().len(), 1 << 40);
}

#[test]
fn own_event_does_not_wake_the_conductor() {
    // 知らせの元が登録された指揮役自身（巡回の終わり）だけの回は起こさない
    let sb = Sandbox::new("wake-own");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:conductor-1","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_conductor-1.json",
        &show("mcp:conductor-1", "completed", "2026-11-10T16:10:00Z", 0),
    );
    fs::write(
        sb.state_file(),
        r#"{"initialized":true,"threads":{"mcp:conductor-1":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null}},"ci_notified":{}}"#,
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(stderr(&out).contains("WAKE skipped reason=own-event"));
}

#[test]
fn missing_patrol_state_skips_wake() {
    let sb = Sandbox::new("wake-no-patrol");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(stderr(&out).contains("mode=not-sent"));
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(stderr(&out).contains("WAKE skipped reason=patrol-state-unreadable"));
}

#[test]
fn partial_line_is_closed_before_appending() {
    // 前回の追記が途中で失敗して、改行の無い断片が残っている
    let sb = Sandbox::new("inbox-partial");
    one_event(&sb);
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    fs::write(sb.inbox(), "{\"event\":\"finis").unwrap();
    fs::set_permissions(sb.inbox(), fs::Permissions::from_mode(0o600)).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let text = fs::read_to_string(sb.inbox()).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "{text}");
    assert_eq!(lines[0], "{\"event\":\"finis");
    let ev: serde_json::Value =
        serde_json::from_str(lines[1]).expect("the new event is its own line");
    assert_eq!(ev["event"], "finished");
}

// ---- PR #561 round 4 のレビューへの対応 ----

#[test]
fn list_and_wake_use_the_same_t3_as_the_server() {
    // PATH の t3 はサーバーと食い違って失敗する。T3_BIN（サーバーと同じ実行ファイル）で一覧から起こしまで進む
    let sb = Sandbox::new("t3-bin");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let server = sb.root.join("server");
    fs::create_dir_all(&server).unwrap();
    fs::copy(sb.root.join("bin/t3"), server.join("t3")).unwrap();
    write_exec(
        &sb.root.join("bin/t3"),
        "#!/usr/bin/env bash\necho \"path-t3 $*\" >> \"$FAKE_DIR/path-t3-calls\"\nexit 1\n",
    );
    let out = sb
        .command(&["--once"])
        .env("T3_BIN", server.join("t3"))
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1, "{}", stderr(&out));
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
    assert!(stderr(&out).contains("mode=not-sent"));
    assert_eq!(sb.read_fake("path-t3-calls"), "");
}

#[test]
fn existing_inbox_readable_by_others_is_not_written() {
    // 前からある受信箱に他人の権限があれば、すでに開かれているかもしれないので書かない（webhook は続ける）
    let sb = Sandbox::new("inbox-mode");
    one_event(&sb);
    sb.fake("fnox-ok", "");
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    fs::write(sb.inbox(), "").unwrap();
    fs::set_permissions(sb.inbox(), fs::Permissions::from_mode(0o644)).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(fs::read_to_string(sb.inbox()).unwrap(), "");
    assert!(stderr(&out).contains("inbox append failed"));
    let sent: serde_json::Value = serde_json::from_str(&sb.read_fake("curl-body")).unwrap();
    assert_eq!(sent["event"], "finished");
}

#[test]
fn wake_again_after_partly_reading() {
    // 2 件の知らせで起こした後、指揮役が 1 行だけ読んだ。次の知らせでは起こす
    let sb = Sandbox::new("wake-partly-read");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:b","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "completed", "2026-11-10T16:10:00Z", 0),
    );
    sb.fake(
        "show-mcp_b.json",
        &show("mcp:b", "completed", "2026-11-10T16:10:00Z", 0),
    );
    fs::write(
        sb.state_file(),
        r#"{"initialized":true,"threads":{
  "mcp:a":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:b":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null}},"ci_notified":{}}"#,
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 2);
    assert_eq!(sb.read_fake("t3-send-args").lines().count(), 1);

    sb.patrol_state(r#"{"inbox_offset":1}"#);
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"failed","updatedAt":"2026-11-10T16:30:00Z"},
  {"threadId":"mcp:b","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "failed", "2026-11-10T16:30:00Z", 0),
    );
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    state["threads"]["mcp:a"]["status"] = "running".into();
    fs::write(sb.state_file(), state.to_string()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 3);
    assert_eq!(
        sb.read_fake("t3-send-args").lines().count(),
        2,
        "{}",
        stderr(&out)
    );
}

#[test]
fn conductor_and_another_thread_in_one_cycle_wake_once() {
    // 指揮役自身の知らせと別のスレッドの知らせが同じ回に出たら、起こす（own-event にしない）
    let sb = Sandbox::new("wake-own-and-other");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:conductor-1","status":"completed","updatedAt":"2026-11-10T16:10:00Z"},
  {"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_conductor-1.json",
        &show("mcp:conductor-1", "completed", "2026-11-10T16:10:00Z", 0),
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "completed", "2026-11-10T16:10:00Z", 0),
    );
    fs::write(
        sb.state_file(),
        r#"{"initialized":true,"threads":{
  "mcp:conductor-1":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:a":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null}},"ci_notified":{}}"#,
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 2);
    assert_eq!(sb.read_fake("t3-send-args").lines().count(), 1);
}

#[test]
fn empty_registration_does_not_wake() {
    let sb = Sandbox::new("wake-empty-reg");
    one_event(&sb);
    sb.conductor("");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(sb.read_fake("t3-send-args").is_empty());
}

fn reg_file(sb: &Sandbox) -> PathBuf {
    sb.root
        .join("home/.config/pelway-agent-notify/conductor-thread-id")
}

fn reg(sb: &Sandbox, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pelway-agent-watch"))
        .args(args)
        .env("HOME", sb.root.join("home"))
        .env("PATH", sb.root.join("bin"))
        .env("FAKE_DIR", sb.root.join("fake"))
        .output()
        .unwrap()
}

#[test]
fn register_show_and_unregister() {
    let sb = Sandbox::new("register");
    let out = reg(&sb, &["--show-registration"]);
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "(none)");

    let out = reg(&sb, &["--register", "mcp:conductor-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(
        fs::read_to_string(reg_file(&sb)).unwrap(),
        "mcp:conductor-1\n"
    );
    let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&reg_file(&sb)), 0o600);
    assert_eq!(mode(reg_file(&sb).parent().unwrap()), 0o700);
    let out = reg(&sb, &["--show-registration"]);
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "mcp:conductor-1"
    );
    // 登録は見張りを始めない（t3 を呼ばない）
    assert!(!sb.t3_calls().exists());

    let out = reg(&sb, &["--unregister"]);
    assert!(out.status.success());
    assert!(!reg_file(&sb).exists());
    let out = reg(&sb, &["--unregister"]);
    assert!(out.status.success());
}

#[test]
fn register_rejects_bad_ids_and_targets() {
    let sb = Sandbox::new("register-bad");
    for bad in ["--help", "-x", "a b", "a;rm", ""] {
        let out = reg(&sb, &["--register", bad]);
        assert_eq!(out.status.code(), Some(1), "{bad}");
        assert!(!reg_file(&sb).exists(), "{bad}");
    }
    let out = reg(&sb, &["--register"]);
    assert!(!out.status.success());

    // 登録ファイルの位置がディレクトリなら、中に置かずに失敗する
    fs::create_dir_all(reg_file(&sb)).unwrap();
    let out = reg(&sb, &["--register", "mcp:conductor-1"]);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(fs::read_dir(reg_file(&sb)).unwrap().count(), 0);
    fs::remove_dir(reg_file(&sb)).unwrap();

    // symlink があれば、たどらずに symlink 自体を置き換える
    let target = sb.root.join("target.txt");
    fs::write(&target, "keep").unwrap();
    std::os::unix::fs::symlink(&target, reg_file(&sb)).unwrap();
    let out = reg(&sb, &["--register", "mcp:conductor-1"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(fs::read_to_string(&target).unwrap(), "keep");
    assert!(
        !fs::symlink_metadata(reg_file(&sb))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn registration_symlink_or_fifo_is_not_followed_by_the_watch() {
    let sb = Sandbox::new("reg-fifo");
    one_event(&sb);
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    fs::create_dir_all(reg_file(&sb).parent().unwrap()).unwrap();
    mkfifo(&reg_file(&sb));
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(stderr(&out).contains("WAKE skipped reason=conductor-file-unreadable"));

    fs::remove_file(reg_file(&sb)).unwrap();
    let elsewhere = sb.root.join("elsewhere");
    fs::write(&elsewhere, "mcp:planted\n").unwrap();
    std::os::unix::fs::symlink(&elsewhere, reg_file(&sb)).unwrap();
    one_event(&sb);
    let out = run_bounded(&sb);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(sb.read_fake("t3-send-args").is_empty());
}

// ---- PR #561 round 6 のレビューへの対応 ----

#[test]
fn out_of_range_offset_is_not_kept() {
    // 受信箱の行数を超える既読の位置では起こさず、基準にも残さない。正しい値に戻せば起こす
    let sb = Sandbox::new("wake-offset-range");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":18446744073709551615}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("WAKE skipped reason=offset-out-of-range"));
    assert!(sb.read_fake("t3-send-args").is_empty());
    let state = fs::read_to_string(sb.state_file()).unwrap();
    assert!(state.contains("\"last_wake_inbox_offset\":null"), "{state}");

    sb.patrol_state(r#"{"inbox_offset":0}"#);
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"failed","updatedAt":"2026-11-10T16:30:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "failed", "2026-11-10T16:30:00Z", 0),
    );
    let mut state: serde_json::Value = serde_json::from_str(&state).unwrap();
    state["threads"]["mcp:a"]["status"] = "running".into();
    fs::write(sb.state_file(), state.to_string()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(
        sb.read_fake("t3-send-args").lines().count(),
        1,
        "{}",
        stderr(&out)
    );
}

#[test]
fn new_conductor_is_woken_without_waiting_for_the_old_baseline() {
    // 前の指揮役を起こしたときの基準は、登録が替わったら使わない
    let sb = Sandbox::new("wake-new-conductor");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");

    sb.conductor("mcp:conductor-2");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"failed","updatedAt":"2026-11-10T16:30:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "failed", "2026-11-10T16:30:00Z", 0),
    );
    let mut state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    state["threads"]["mcp:a"]["status"] = "running".into();
    fs::write(sb.state_file(), state.to_string()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let sends = sb.read_fake("t3-send-args");
    assert_eq!(
        sends.lines().last(),
        Some("mcp:conductor-2 -"),
        "{}",
        stderr(&out)
    );
}
