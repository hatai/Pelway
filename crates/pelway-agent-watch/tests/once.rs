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
        Sandbox { root }
    }

    fn fake(&self, name: &str, body: &str) {
        fs::write(self.root.join("fake").join(name), body).unwrap();
    }

    fn state_file(&self) -> PathBuf {
        self.root.join("state/pelway/agent-watch.json")
    }

    fn run(&self) -> Output {
        let path = format!(
            "{}:{}",
            self.root.join("bin").display(),
            std::env::var("PATH").unwrap()
        );
        Command::new(env!("CARGO_BIN_EXE_pelway-agent-watch"))
            .args(["--once", "--dry-run", "--project", "."])
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
    let out = sb.run();
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
    let out = sb.run();
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    assert_eq!(state["initialized"], true);
}
