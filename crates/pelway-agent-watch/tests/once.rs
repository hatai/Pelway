//! `pelway-agent-watch --once --dry-run` の結合テスト。
//! 偽の `t3` / `gh` を一時ディレクトリに置いて PATH の先頭にし、状態ファイルは XDG_STATE_HOME で一時ディレクトリに向ける。

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

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
        Sandbox { root }
    }

    fn fake(&self, name: &str, body: &str) {
        fs::write(self.root.join("fake").join(name), body).unwrap();
    }

    fn state_file(&self) -> PathBuf {
        self.root.join("state/pelway/agent-watch.json")
    }

    fn run(&self) -> Output {
        self.run_with(&["--once", "--dry-run"])
    }

    fn run_with(&self, args: &[&str]) -> Output {
        let path = format!(
            "{}:{}",
            self.root.join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        Command::new(env!("CARGO_BIN_EXE_pelway-agent-watch"))
            .args(args)
            .args(["--project", "."])
            .env("PELWAY_AGENT_WATCH_POLL_SECS", "0")
            .env("PATH", path)
            .env("FAKE_DIR", self.root.join("fake"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            .output()
            .unwrap()
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
