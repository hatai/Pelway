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
        // fake/fnox-ok があるときだけ偽の値を返す（送信が失敗する場合の試験に使う）。
        // fake/fnox-sleep・fake/curl-sleep があれば、その秒数だけ待ってから答える。
        // 呼び出しの引数は fake/fnox-calls に 1 行ずつ記録する（-c の場所の確認に使う）
        write_exec(
            &root.join("bin/fnox"),
            "#!/usr/bin/env bash\necho \"$*\" >> \"$FAKE_DIR/fnox-calls\"\nsleep \"$(cat \"$FAKE_DIR/fnox-sleep\" 2>/dev/null || echo 0)\"\n[ -e \"$FAKE_DIR/fnox-ok\" ] && { echo fake-secret; exit 0; }\nexit 1\n",
        );
        // 偽の curl: 本文（--data-binary @<file>）を fake/curl-body に写し、fake/curl-bodies に 1 行ずつ足す。
        // 返す HTTP ステータスは fake/curl-code（無ければ 500）。fake/curl-code-by-thread の
        // `<thread_id> <コード>` の行に本文の thread_id が当たれば、そのコード。000 は接続の失敗（終了コード 7）
        write_exec(
            &root.join("bin/curl"),
            r#"#!/usr/bin/env bash
cat > /dev/null
while [ $# -gt 0 ]; do
  if [ "$1" = "--data-binary" ]; then body="${2#@}"; fi
  shift
done
cp "$body" "$FAKE_DIR/curl-body"
{ cat "$body"; echo; } >> "$FAKE_DIR/curl-bodies"
sleep "$(cat "$FAKE_DIR/curl-sleep" 2>/dev/null || echo 0)"
code=$(cat "$FAKE_DIR/curl-code" 2>/dev/null || echo 500)
while read -r tid c; do
  grep -qF "\"thread_id\":\"$tid\"" "$body" && code=$c
done < <(cat "$FAKE_DIR/curl-code-by-thread" 2>/dev/null)
printf %s "$code"
[ "$code" = 000 ] && exit 7
exit 0
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
            // 起こしの最短間隔は既定で 0（間隔の試験だけが値を変える）
            .env("PELWAY_AGENT_WATCH_WAKE_COOLDOWN_SECS", "0")
            .env("PATH", path)
            .env("FAKE_DIR", self.root.join("fake"))
            // 起こしに使う t3 は偽物に向ける（この PC で動いている本物の T3 サーバーを呼ばない）
            .env("T3_BIN", self.root.join("bin/t3"))
            // 指揮役の登録ファイル（~/.config/pelway-agent-notify/conductor-thread-id）の置き場
            .env("HOME", self.root.join("home"))
            .env("XDG_STATE_HOME", self.root.join("state"))
            // 通知用 fnox 設定の場所は試験ごとに決める（実行環境の値を拾わない）
            .env_remove("PELWAY_NOTIFY_FNOX_CONFIG");
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

/// `PELWAY_NOTIFY_FNOX_CONFIG` の絶対パスが `fnox -c` に渡る。
#[test]
fn notify_fnox_config_env_override() {
    let sb = Sandbox::new("notify-cfg-override");
    one_event(&sb);
    sb.fake("fnox-ok", "");
    let cfg = sb.root.join("fake/notify-elsewhere.toml");
    let out = sb
        .command(&["--once"])
        .env("PELWAY_NOTIFY_FNOX_CONFIG", &cfg)
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    // 秘密は URL と鍵の 2 件を、上書きした設定の場所から読みにいく
    let calls = sb.read_fake("fnox-calls");
    assert_eq!(
        calls,
        format!(
            "-c {0} get PELWAY_NOTIFY_WEBHOOK_URL\n-c {0} get PELWAY_NOTIFY_WEBHOOK_KEY\n",
            cfg.display()
        )
    );
}

/// 環境変数が無ければ既定の `~/.config/pelway-agent-notify/fnox.toml` のまま（既定は変えない）。
#[test]
fn notify_fnox_config_default() {
    let sb = Sandbox::new("notify-cfg-default");
    one_event(&sb);
    sb.fake("fnox-ok", "");
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let cfg = sb.root.join("home/.config/pelway-agent-notify/fnox.toml");
    let calls = sb.read_fake("fnox-calls");
    assert_eq!(
        calls,
        format!(
            "-c {0} get PELWAY_NOTIFY_WEBHOOK_URL\n-c {0} get PELWAY_NOTIFY_WEBHOOK_KEY\n",
            cfg.display()
        )
    );
}

/// 空文字・相対パスは設定として使わない（fnox を呼ばず、送らずに `reason=no-secret` と同じ扱い。
/// 相対パスを `--project` のカレントから解決して、sandbox の中から置かれた設定を読むこともない）。
#[test]
fn notify_fnox_config_invalid_is_not_used() {
    for (name, value) in [
        ("notify-cfg-empty", ""),
        ("notify-cfg-relative", "conf/notify.toml"),
    ] {
        let sb = Sandbox::new(name);
        one_event(&sb);
        sb.fake("fnox-ok", "");
        let out = sb
            .command(&["--once"])
            .env("PELWAY_NOTIFY_FNOX_CONFIG", value)
            .output()
            .unwrap();
        assert!(out.status.success(), "stderr: {}", stderr(&out));
        let err = stderr(&out);
        assert!(
            err.contains("mode=not-sent reason=no-secret"),
            "{name}: {err}"
        );
        assert!(
            sb.read_fake("fnox-calls").is_empty(),
            "{name}: fnox を呼んでいる"
        );
        // 知らせは送信待ちの列に残る（no-secret と同じ扱い）
        let state: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
        assert_eq!(state["outbox"].as_array().unwrap().len(), 1, "{name}");
    }
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
    // 出るはずの知らせが、過不足なく 1 行ずつある（スレッド・種類・止まりの理由の組で比べる）。
    // 試験の時刻は古いので、実行中の d は waiting に加えて no-activity の stalled も出る
    let mut got: Vec<String> = events
        .iter()
        .map(|e| {
            format!(
                "{} {} {}",
                e["thread_id"].as_str().unwrap_or(""),
                e["event"].as_str().unwrap_or(""),
                e["stall_reason"].as_str().unwrap_or("-")
            )
        })
        .collect();
    got.sort();
    let want = [
        "mcp:a finished -",
        "mcp:b failed -",
        "mcp:c cancelled -",
        "mcp:d stalled no-activity",
        "mcp:d waiting -",
        "mcp:e stalled no-activity",
    ];
    assert_eq!(got, want);
    // webhook と同じ形（schema が pelway.agent-watch/v1。payload 全体の一致は inbox_written_when_webhook_fails で確かめる）
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

// ---- 受信箱・補助記録の置き場は sandbox の中から書けるので、種類を確かめて開く ----

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

// ---- 人間が立てたスレッドの知らせの起こし（Issue #727） ----

/// 一覧の 1 要素に `createdBy` を付けた list の JSON。
fn list_with_creator(id: &str, status: &str, updated: &str, created_by: &str) -> String {
    format!(
        r#"[{{"threadId":"{id}","status":"{status}","updatedAt":"{updated}","createdBy":"{created_by}"}}]"#
    )
}

/// 報告の行にならない最後の返答（`finished` で `report` が null になる）を持つ show の JSON。
fn show_prose(id: &str, status: &str, updated: &str) -> String {
    format!(
        r#"{{"thread":{{"threadId":"{id}","status":"{status}","updatedAt":"{updated}","branch":null,
  "providerInstanceId":"opencode","model":"m","runCount":1,"pendingRequestCount":0,"link":"l"}},
  "messages":[{{"role":"assistant","text":"確認しました。"}}]}}"#
    )
}

/// 報告の行にならない最後の返答で、thread に `createdBy` を持たせた show の JSON。
fn show_prose_with_creator(id: &str, status: &str, updated: &str, created_by: &str) -> String {
    format!(
        r#"{{"thread":{{"threadId":"{id}","status":"{status}","updatedAt":"{updated}","branch":null,
  "providerInstanceId":"opencode","model":"m","runCount":1,"pendingRequestCount":0,"link":"l","createdBy":"{created_by}"}},
  "messages":[{{"role":"assistant","text":"確認しました。"}}]}}"#
    )
}

/// `show` と同じ報告行のメッセージで、thread に `createdBy` を持たせた show の JSON。
fn show_with_creator(id: &str, status: &str, updated: &str, created_by: &str) -> String {
    format!(
        r#"{{"thread":{{"threadId":"{id}","status":"{status}","updatedAt":"{updated}","branch":"agent/80-x",
  "providerInstanceId":"opencode","model":"m","runCount":1,"pendingRequestCount":0,"link":"l","createdBy":"{created_by}"}},
  "messages":[{{"role":"assistant","text":"state: working\nhead: none\nnext: 手順 4"}}]}}"#
    )
}

/// 1 本のスレッドが前回まで実行中だった状態ファイルの中身。
fn state_one_running(id: &str) -> String {
    format!(
        r#"{{"initialized":true,"threads":{{"{id}":{{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null}}}},"ci_notified":{{}}}}"#
    )
}

#[test]
fn human_finished_without_report_keeps_inbox_but_does_not_wake() {
    // 人間が立てたスレッド（createdBy が agent 以外）の、報告の無い finished:
    // 受信箱には今までどおり書くが、指揮役は起こさない（次の定期巡回で読ませる）
    let sb = Sandbox::new("wake-human-quiet");
    sb.fake(
        "list.json",
        &list_with_creator("mcp:a", "completed", "2026-11-10T16:10:00Z", "user"),
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose("mcp:a", "completed", "2026-11-10T16:10:00Z"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let events = inbox_events(&sb);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "finished");
    assert_eq!(events[0]["report"], serde_json::Value::Null);
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(
        stderr(&out).contains("WAKE skipped reason=human-non-urgent"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn human_finished_with_report_wakes() {
    // 人間が立てたスレッドでも、報告の行のある finished は起こす。
    // createdBy は詳細（show）にだけある形（一覧に無いときの取り込みを兼ねる）
    let sb = Sandbox::new("wake-human-report");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show_with_creator("mcp:a", "completed", "2026-11-10T16:10:00Z", "user"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
}

#[test]
fn human_failed_wakes() {
    // 人間が立てたスレッドの failed は起こす（waiting・stalled も同じ）
    let sb = Sandbox::new("wake-human-failed");
    sb.fake(
        "list.json",
        &list_with_creator("mcp:a", "failed", "2026-11-10T16:10:00Z", "user"),
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose("mcp:a", "failed", "2026-11-10T16:10:00Z"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
}

#[test]
fn agent_finished_without_report_wakes() {
    // エージェントが立てたスレッドの扱いは変えない: 報告の無い finished でも起こす
    let sb = Sandbox::new("wake-agent-quiet");
    sb.fake(
        "list.json",
        &list_with_creator("mcp:a", "completed", "2026-11-10T16:10:00Z", "agent"),
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose("mcp:a", "completed", "2026-11-10T16:10:00Z"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
}

#[test]
fn unknown_creator_finished_without_report_wakes() {
    // createdBy が無い（立て手不明）は、起こし漏れを避けるため従来どおり起こす
    let sb = Sandbox::new("wake-unknown-creator");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose("mcp:a", "completed", "2026-11-10T16:10:00Z"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
}

#[test]
fn detail_only_creator_finished_without_report_does_not_wake() {
    // createdBy が詳細（show）にだけあり一覧には無い人間スレッドの、報告の無い finished:
    // 詳細から立て手を取り込んで起こさない（人間×報告ありで確かめると、
    // 立て手不明でも同じように起こすので詳細の取り込みを検査できないため）
    let sb = Sandbox::new("wake-human-detail-only");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose_with_creator("mcp:a", "completed", "2026-11-10T16:10:00Z", "user"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 1);
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(
        stderr(&out).contains("WAKE skipped reason=human-non-urgent"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn human_waiting_wakes() {
    // 人間が立てたスレッドの waiting は起こす（承認・入力の待ちは報告が無くても急ぐ）
    let sb = Sandbox::new("wake-human-waiting");
    sb.fake(
        "list.json",
        &list_with_creator("mcp:a", "running", "2026-11-10T16:10:00Z", "user"),
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "running", "2026-11-10T16:10:00Z", 1),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let events = inbox_events(&sb);
    assert!(events.iter().any(|e| e["event"] == "waiting"), "{events:?}");
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
}

#[test]
fn human_stalled_wakes() {
    // 人間が立てたスレッドの stalled も起こす（迷走の知らせは急ぐ）
    let sb = Sandbox::new("wake-human-stalled");
    sb.fake(
        "list.json",
        &list_with_creator("mcp:a", "running", "2026-11-10T16:00:00Z", "user"),
    );
    sb.fake(
        "show-mcp_a.json",
        &show("mcp:a", "running", "2026-11-10T16:00:00Z", 0),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let events = inbox_events(&sb);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "stalled");
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
}

#[test]
fn own_and_human_quiet_notices_do_not_wake() {
    // 指揮役自身のスレッドの知らせと、人間スレッドの報告の無い finished が同じ周回でも
    // 起こさない（自身の知らせは起こすに足るかの判定に数えない）。2 件とも受信箱には残る
    let sb = Sandbox::new("wake-own-plus-human");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:conductor-1","status":"completed","updatedAt":"2026-11-10T16:10:00Z","createdBy":"agent"},
  {"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z","createdBy":"user"}]"#,
    );
    sb.fake(
        "show-mcp_conductor-1.json",
        &show_with_creator(
            "mcp:conductor-1",
            "completed",
            "2026-11-10T16:10:00Z",
            "agent",
        ),
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose("mcp:a", "completed", "2026-11-10T16:10:00Z"),
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
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(
        stderr(&out).contains("WAKE skipped reason=human-non-urgent"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn inbox_retry_keeps_resolved_creator() {
    // 受信箱への追記が失敗したあとの再試行でも、その時点で判明していた立て手を使う:
    // 人間スレッドの報告の無い finished は、再試行で書けても起こさない。
    // 完了済みのスレッドは次の周回で詳細を取り直さないので、記録に立て手を残さないと
    // 立て手不明として起こしてしまう（createdBy は詳細にだけある形で確かめる）
    let sb = Sandbox::new("wake-retry-creator");
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}]"#,
    );
    sb.fake(
        "show-mcp_a.json",
        &show_prose_with_creator("mcp:a", "completed", "2026-11-10T16:10:00Z", "user"),
    );
    fs::write(sb.state_file(), state_one_running("mcp:a")).unwrap();
    sb.conductor("mcp:conductor-1");
    // 受信箱を symlink にして追記を失敗させる
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    std::os::unix::fs::symlink(sb.root.join("elsewhere"), sb.inbox()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("inbox append failed"),
        "{}",
        stderr(&out)
    );
    // 再試行の記録は payload そのものに留める（別の形に包むと、更新前の版に戻した
    // ときに包んだ形のまま受信箱に書かれて、読み手が拾えない行になる）。判明して
    // いた立て手は別項目に残る
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    let pending = &state["inbox_pending"][0];
    assert_eq!(pending["schema"], "pelway.agent-watch/v1", "{state}");
    assert_eq!(pending["event"], "finished", "{state}");
    assert!(
        pending.get("notice").is_none() && pending.get("created_by").is_none(),
        "{state}"
    );
    assert_eq!(state["inbox_pending_creators"]["mcp:a"], "user", "{state}");
    // 受信箱を直してもう一周。次の周回では詳細を取り直さないため、
    // 記録に残した立て手で判定する
    fs::remove_file(sb.inbox()).unwrap();
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let events = inbox_events(&sb);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "finished");
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(
        stderr(&out).contains("WAKE skipped reason=human-non-urgent"),
        "{}",
        stderr(&out)
    );
    // 書き直せてキューが空になったので、立て手の記録も消える
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    assert_eq!(
        state["inbox_pending_creators"].as_object().unwrap().len(),
        0,
        "{state}"
    );
}

#[test]
fn inbox_pending_creators_pruned_with_queue() {
    // 立て手の記録は、キューに残る知らせの元のスレッドの分だけ残る:
    // 追記に失敗して上限を超えて古い知らせが外れた分（とキューに無い古い項目）は消え、
    // 書き直せてキューが空になったら全部消える
    let sb = Sandbox::new("inbox-creators-pruned");
    sb.fake("list.json", "[]");
    let pending: Vec<String> = (0..101)
        .map(|i| {
            format!(
                r#"{{"schema":"pelway.agent-watch/v1","event":"finished","thread_id":"mcp:p{i}","report":null}}"#
            )
        })
        .collect();
    let creators: Vec<String> = (0..101)
        .map(|i| format!(r#""mcp:p{i}":"user""#))
        .chain(std::iter::once(r#""mcp:old":"user""#.to_string()))
        .collect();
    fs::write(
        sb.state_file(),
        format!(
            r#"{{"initialized":true,"threads":{{}},"inbox_pending":[{}],"inbox_pending_creators":{{{}}},"ci_notified":{{}}}}"#,
            pending.join(","),
            creators.join(",")
        ),
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
    // 追記を失敗させる: 101 件がキューに戻り、上限（100 件）でいちばん古い mcp:p0 が外れる
    fs::create_dir_all(sb.pelway_dir()).unwrap();
    std::os::unix::fs::symlink(sb.root.join("elsewhere"), sb.inbox()).unwrap();
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    assert_eq!(
        state["inbox_pending"].as_array().unwrap().len(),
        100,
        "{state}"
    );
    let m = state["inbox_pending_creators"].as_object().unwrap();
    assert_eq!(m.len(), 100, "{state}");
    assert!(
        !m.contains_key("mcp:p0") && !m.contains_key("mcp:old"),
        "{state}"
    );
    assert_eq!(m["mcp:p100"], "user", "{state}");
    // 受信箱を直すと全部書けて、立て手の記録は空になる
    fs::remove_file(sb.inbox()).unwrap();
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(inbox_events(&sb).len(), 100);
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    assert_eq!(
        state["inbox_pending"].as_array().unwrap().len(),
        0,
        "{state}"
    );
    assert_eq!(
        state["inbox_pending_creators"].as_object().unwrap().len(),
        0,
        "{state}"
    );
}

#[test]
fn legacy_inbox_pending_is_still_read() {
    // 立て手を持たない前の版の再試行の記録（payload そのもの）も読んで受信箱に書く。
    // 立て手はその周の一覧・詳細から取り直し、分からなければ従来どおり起こす
    let sb = Sandbox::new("inbox-legacy-pending");
    sb.fake("list.json", "[]");
    fs::write(
        sb.state_file(),
        r#"{"initialized":true,"threads":{},"inbox_pending":[{"schema":"pelway.agent-watch/v1","event":"finished","thread_id":"mcp:x","report":null}],"ci_notified":{}}"#,
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    let events = inbox_events(&sb);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["thread_id"], "mcp:x");
    assert_eq!(sb.read_fake("t3-send-args").trim(), "mcp:conductor-1 -");
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

// ---- t3 の解決、受信箱の権限、own-event の混在、空の登録、登録の操作 ----

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

// ---- 既読の位置の範囲、宛先の切り替え ----

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

// ---- 既読の位置の型 ----

#[test]
fn odd_inbox_offset_skips_wake() {
    // 既読の位置が 0 以上の整数でないときは、その回は起こさず、追記と webhook は続ける
    for (i, bad) in [r#""3""#, "-1"].iter().enumerate() {
        let sb = Sandbox::new(&format!("wake-odd-offset-{i}"));
        one_event(&sb);
        sb.fake("fnox-ok", "");
        sb.conductor("mcp:conductor-1");
        sb.patrol_state(&format!(r#"{{"inbox_offset":{bad}}}"#));
        let out = sb.run_with(&["--once"]);
        assert!(out.status.success(), "stderr: {}", stderr(&out));
        assert_eq!(inbox_events(&sb).len(), 1);
        assert!(sb.read_fake("t3-send-args").is_empty(), "{bad}");
        assert!(stderr(&out).contains("WAKE skipped reason=patrol-state-unreadable"));
        assert!(!sb.read_fake("curl-body").is_empty());
    }
}

#[test]
fn oversized_patrol_state_has_its_own_reason() {
    let sb = Sandbox::new("wake-patrol-too-large");
    one_event(&sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(&format!(
        r#"{{"inbox_offset":0,"pad":"{}"}}"#,
        "x".repeat(300 * 1024)
    ));
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert!(sb.read_fake("t3-send-args").is_empty());
    assert!(stderr(&out).contains("WAKE skipped reason=patrol-state-too-large"));
}

// ---- webhook の送信の失敗と送信待ちの列（Issue #513） ----

/// 送信待ちの列に置く payload。ログに出してよいのは event・thread・branch だけで、report は出さない。
fn queued_payload(thread: &str) -> serde_json::Value {
    serde_json::json!({
        "schema": "pelway.agent-watch/v1", "event": "finished", "urgency": "immediate",
        "thread_id": thread, "provider": "opencode", "model": "m", "branch": "agent/80-x",
        "issue_number": 80, "round": 1, "report": "issue: #80\nverify: pass", "stall_reason": null,
        "link": "l", "host": "h", "occurred_at": "2026-11-10T16:10:00Z"
    })
}

/// 新しい知らせが出ない状態で、送信待ちの列に `threads` の payload を古い順に置く。
/// 状態は更新前の版の形（`inbox_pending` が無い）で、受信箱にはまだ何も書かれていない。
fn queue(sb: &Sandbox, threads: &[&str]) {
    sb.fake("list.json", "[]");
    let outbox: Vec<_> = threads.iter().map(|t| queued_payload(t)).collect();
    let state = serde_json::json!({"initialized": true, "threads": {}, "ci_notified": {}, "outbox": outbox});
    fs::write(sb.state_file(), state.to_string()).unwrap();
}

fn thread_ids<'a>(payloads: impl Iterator<Item = &'a serde_json::Value>) -> Vec<String> {
    payloads
        .map(|p| p["thread_id"].as_str().unwrap().to_string())
        .collect()
}

/// 送信待ちの列に残っている payload の thread_id（古い順）。
fn outbox_threads(sb: &Sandbox) -> Vec<String> {
    let state: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap();
    thread_ids(state["outbox"].as_array().unwrap().iter())
}

/// webhook（偽の curl）に渡った payload の thread_id（送った順。送り直しも 1 回と数える）。
fn sent_threads(sb: &Sandbox) -> Vec<String> {
    let sent: Vec<serde_json::Value> = sb
        .read_fake("curl-bodies")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    thread_ids(sent.iter())
}

#[test]
fn permanent_failure_is_dropped_and_next_is_sent() {
    for code in ["400", "404", "413"] {
        let sb = Sandbox::new(&format!("send-dropped-{code}"));
        queue(&sb, &["mcp:a", "mcp:b"]);
        sb.fake("fnox-ok", "");
        sb.fake("curl-code-by-thread", &format!("mcp:a {code}\nmcp:b 200\n"));
        let out = sb.run_with(&["--once"]);
        let err = stderr(&out);
        assert!(out.status.success(), "{err}");
        // 直らない失敗を返された 1 件目は送り直さずに列から外し、同じ周回で 2 件目を送る
        assert_eq!(sent_threads(&sb), ["mcp:a", "mcp:b"], "{code}: {err}");
        assert!(outbox_threads(&sb).is_empty(), "{code}: {err}");
        // 更新前の版の列は受信箱に書けたかが分からないので書き直す。列から外した 1 件目も受信箱には届く
        let inbox = thread_ids(inbox_events(&sb).iter());
        assert_eq!(inbox, ["mcp:a", "mcp:b"], "{code}: {err}");
        let dropped: Vec<&str> = err
            .lines()
            .filter(|l| l.contains("mode=send-dropped"))
            .collect();
        let want = format!(
            "agent-watch: event=finished thread=mcp:a branch=agent/80-x mode=send-dropped http={code}"
        );
        assert_eq!(dropped, [want.as_str()], "{err}");
        // 秘密（偽の fnox が URL と鍵に返す値）と payload の本文はログに出ない
        assert!(
            !err.contains("fake-secret") && !err.contains("issue: #80"),
            "{err}"
        );
    }
}

#[test]
fn retryable_failure_is_kept() {
    // 1 件ごとに送り直しの待ち（2 秒 × 2）があるので、別々の sandbox で並べて走らせる
    let cases = ["401", "403", "429", "500", "503", "000", "no-secret"];
    let runs: Vec<_> = cases
        .iter()
        .map(|&case| {
            std::thread::spawn(move || {
                let sb = Sandbox::new(&format!("retryable-{case}"));
                queue(&sb, &["mcp:a"]);
                if case != "no-secret" {
                    sb.fake("fnox-ok", "");
                    sb.fake("curl-code", case);
                }
                let out = sb.run_with(&["--once"]);
                (
                    case,
                    out.status.success(),
                    stderr(&out),
                    outbox_threads(&sb),
                )
            })
        })
        .collect();
    for run in runs {
        let (case, ok, err, kept) = run.join().unwrap();
        assert!(ok, "{case}: {err}");
        assert_eq!(kept, ["mcp:a"], "{case}: {err}");
        let logged = if case == "no-secret" {
            "mode=not-sent reason=no-secret".to_string()
        } else {
            format!("mode=send-failed reason=http={case}")
        };
        assert!(err.contains(&logged), "{case}: {err}");
        assert!(!err.contains("mode=send-dropped"), "{case}: {err}");
    }
}

#[test]
fn send_time_per_cycle_is_bounded() {
    let sb = Sandbox::new("send-bounded");
    let threads: Vec<String> = (0..10).map(|i| format!("mcp:q{i}")).collect();
    let refs: Vec<&str> = threads.iter().map(String::as_str).collect();
    queue(&sb, &refs);
    sb.fake("fnox-ok", "");
    let started = std::time::Instant::now();
    let out = sb.run_with(&["--once"]);
    let took = started.elapsed();
    let err = stderr(&out);
    assert!(out.status.success(), "{err}");
    assert!(took < std::time::Duration::from_secs(45), "{took:?} {err}");
    // 送り直せる失敗（500）が出たら、その周は後ろの知らせを送らず、10 件とも列に残る
    let sent = sent_threads(&sb);
    assert!(
        !sent.is_empty() && sent.iter().all(|t| t == "mcp:q0"),
        "{sent:?}"
    );
    assert_eq!(outbox_threads(&sb), threads);
}

#[test]
fn next_notice_is_sent_while_the_deadline_remains() {
    let sb = Sandbox::new("send-after-12s");
    queue(&sb, &["mcp:a", "mcp:b"]);
    sb.fake("fnox-ok", "");
    sb.fake("fnox-sleep", "6");
    sb.fake("curl-code-by-thread", "mcp:a 400\nmcp:b 200\n");
    let err = stderr(&sb.run_with(&["--once"]));
    // 1 件目の 400 は秘密の取得（6 秒 × 2）の後、送信の開始から約 12 秒で返る。
    // 期限（40 秒）までの時間が残っているので、同じ周回で 2 件目を送る
    assert_eq!(sent_threads(&sb), ["mcp:a", "mcp:b"], "{err}");
    assert!(outbox_threads(&sb).is_empty(), "{err}");
}

#[test]
fn backoff_is_kept_across_restart() {
    let sb = Sandbox::new("backoff-restart");
    queue(&sb, &["mcp:a"]);
    sb.fake("fnox-ok", "");
    let out = sb.run_with(&["--once"]);
    let err = stderr(&out);
    assert!(out.status.success(), "{err}");
    assert!(err.contains("outbox retry_after=30s queued=1"), "{err}");
    let tried = sent_threads(&sb).len();
    assert!(tried > 0);
    // 次に送る時刻（30 秒後）より前に起動し直した周回では、webhook に送らない
    let out = sb.run_with(&["--once"]);
    let err = stderr(&out);
    assert!(out.status.success(), "{err}");
    assert_eq!(sent_threads(&sb).len(), tried, "{err}");
    assert!(!err.contains("mode=send"), "{err}");
    assert_eq!(outbox_threads(&sb), ["mcp:a"]);
    // 更新前の版の列を受信箱に書き直すのは、最初の周回の 1 回だけ
    assert_eq!(thread_ids(inbox_events(&sb).iter()), ["mcp:a"]);
}

/// 次の周回に、b の実行が終わった新しい知らせを出す（a は前の周回の知らせのまま）。
fn b_finishes(sb: &Sandbox) {
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
}

/// 指揮役を登録し、2 周続けて新しい知らせを出す（2 周目の前に、指揮役は 1 周目の 1 行を読んでいる）。
/// 周ごとに、受信箱の行数・起こしの回数・webhook に渡った回数（累計）・標準エラーを返す。
fn two_cycles_with_conductor(sb: &Sandbox) -> Vec<(usize, usize, usize, String)> {
    one_event(sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let mut cycles = Vec::new();
    for cycle in 0..2 {
        if cycle == 1 {
            sb.patrol_state(r#"{"inbox_offset":1}"#);
            b_finishes(sb);
        }
        let out = sb.run_with(&["--once"]);
        assert!(out.status.success(), "stderr: {}", stderr(&out));
        cycles.push((
            inbox_events(sb).len(),
            sb.read_fake("t3-send-args").lines().count(),
            sent_threads(sb).len(),
            stderr(&out),
        ));
    }
    cycles
}

#[test]
fn wake_sent_when_webhook_rejects_permanently() {
    let sb = Sandbox::new("wake-webhook-400");
    sb.fake("fnox-ok", "");
    sb.fake("curl-code", "400");
    let mut sent_before = 0;
    for (i, (inbox, wakes, sent, err)) in two_cycles_with_conductor(&sb).into_iter().enumerate() {
        assert_eq!((inbox, wakes), (i + 1, i + 1), "cycle {i}: {err}");
        // その周回でも webhook に送って 400 を返されている
        assert!(sent > sent_before, "cycle {i}: {err}");
        assert!(err.contains("http=400"), "cycle {i}: {err}");
        sent_before = sent;
    }
}

#[test]
fn wake_sent_without_webhook_secret() {
    let sb = Sandbox::new("wake-no-secret");
    for (i, (inbox, wakes, sent, err)) in two_cycles_with_conductor(&sb).into_iter().enumerate() {
        assert_eq!((inbox, wakes), (i + 1, i + 1), "cycle {i}: {err}");
        assert_eq!(sent, 0, "cycle {i}: {err}");
    }
    assert_eq!(outbox_threads(&sb), ["mcp:a", "mcp:b"]);
}

#[test]
fn send_deadline_covers_secrets_and_curl() {
    // fnox が 1 回 25 秒かかる場合と、fnox が 12 秒・curl が 30 秒かかる場合（どちらも残り時間を超える）
    let cases = [("fnox", "25", "0"), ("curl", "12", "30")];
    let runs: Vec<_> = cases
        .iter()
        .map(|&(name, fnox, curl)| {
            std::thread::spawn(move || {
                let sb = Sandbox::new(&format!("deadline-{name}"));
                queue(&sb, &["mcp:a", "mcp:b"]);
                sb.fake("fnox-ok", "");
                sb.fake("fnox-sleep", fnox);
                sb.fake("curl-sleep", curl);
                let started = std::time::Instant::now();
                let first = stderr(&sb.run_with(&["--once"]));
                let took = started.elapsed();
                let restart = stderr(&sb.run_with(&["--once"]));
                (name, took, first, restart, outbox_threads(&sb))
            })
        })
        .collect();
    for run in runs {
        let (name, took, first, restart, kept) = run.join().unwrap();
        assert!(
            took < std::time::Duration::from_secs(45),
            "{name}: {took:?} {first}"
        );
        assert!(
            first.contains("mcp:a branch=agent/80-x mode=send-failed reason=deadline"),
            "{name}: {first}"
        );
        // 期限までに送れなかった知らせは列に残り、送信は初めの間隔（30 秒）より長くかかったが、
        // 直後に起動し直した周回は間隔を空けて送らない
        assert_eq!(kept, ["mcp:a", "mcp:b"], "{name}: {first}");
        assert!(!restart.contains("mode="), "{name}: {restart}");
    }
}

#[test]
fn notice_not_in_inbox_survives_permanent_rejection() {
    let sb = Sandbox::new("inbox-fails-rejected");
    one_event(&sb);
    sb.fake("fnox-ok", "");
    sb.fake("curl-code-by-thread", "mcp:a 400\nmcp:b 200\n");
    fs::write(sb.pelway_dir(), "not a directory").unwrap();
    for cycle in 0..3 {
        match cycle {
            1 => b_finishes(&sb),
            2 => fs::remove_file(sb.pelway_dir()).unwrap(),
            _ => {}
        }
        let out = sb.run_with(&["--once"]);
        assert!(out.status.success(), "cycle {cycle}: {}", stderr(&out));
    }
    // webhook が外した a は後ろの b の送信を止めず、受信箱が直った周回で a と b が古い順に書かれる
    assert_eq!(sent_threads(&sb), ["mcp:a", "mcp:b"]);
    assert!(outbox_threads(&sb).is_empty());
    assert_eq!(thread_ids(inbox_events(&sb).iter()), ["mcp:a", "mcp:b"]);
}

// ---- 空振りの起こしを減らす（Issue #839） ----

/// 実行回数（round）と承認待ちの件数を指定した show の JSON。
fn show_round(id: &str, status: &str, updated: &str, pending: u32, round: u32) -> String {
    format!(
        r#"{{"thread":{{"threadId":"{id}","status":"{status}","updatedAt":"{updated}","branch":"agent/80-x",
  "providerInstanceId":"opencode","model":"m","runCount":{round},"pendingRequestCount":{pending},"link":"l","createdBy":"agent"}},
  "messages":[{{"role":"assistant","text":"state: working\nhead: none\nnext: 手順 4"}}]}}"#
    )
}

fn read_state(sb: &Sandbox) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(sb.state_file()).unwrap()).unwrap()
}

fn write_state(sb: &Sandbox, state: &serde_json::Value) {
    fs::write(sb.state_file(), state.to_string()).unwrap();
}

fn sends(sb: &Sandbox) -> usize {
    sb.read_fake("t3-send-args").lines().count()
}

/// 指揮役 `mcp:conductor-1` と、承認待ちを繰り返す実装スレッド `mcp:d` の一覧と状態を作る。
/// d の停滞（no-activity）は記録済みにしておき、承認待ちの知らせだけが出るようにする。
fn conductor_and_waiting_thread(sb: &Sandbox) {
    sb.fake(
        "list.json",
        r#"[{"threadId":"mcp:conductor-1","status":"completed","updatedAt":"2026-11-10T16:10:00Z","createdBy":"agent"},
  {"threadId":"mcp:d","status":"running","updatedAt":"2026-11-10T16:10:00Z","createdBy":"agent"}]"#,
    );
    sb.fake(
        "show-mcp_conductor-1.json",
        &show_round("mcp:conductor-1", "completed", "2026-11-10T16:10:00Z", 0, 1),
    );
    sb.fake(
        "show-mcp_d.json",
        &show_round("mcp:d", "running", "2026-11-10T16:10:00Z", 1, 1),
    );
    fs::write(
        sb.state_file(),
        r#"{"initialized":true,"threads":{
  "mcp:conductor-1":{"status":"completed","updated_at":"2026-11-10T16:10:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:d":{"status":"running","updated_at":"2026-11-10T16:10:00Z","run_count":1,"pending_requests":0,"stalled_for":"2026-11-10T16:10:00Z"}},
  "ci_notified":{}}"#,
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
}

/// 次の周回で、指揮役の巡回が終わり（finished）、d がまた承認待ちになる（同じ round）ように状態を戻す。
fn conductor_finishes_and_d_waits_again(sb: &Sandbox) {
    let mut state = read_state(sb);
    state["threads"]["mcp:conductor-1"]["status"] = "running".into();
    state["threads"]["mcp:d"]["pending_requests"] = 0.into();
    write_state(sb, &state);
}

fn inbox_lines(sb: &Sandbox) -> usize {
    inbox_events(sb).len()
}

#[test]
fn own_finished_and_repeated_waiting_do_not_wake() {
    let sb = Sandbox::new("wake-839-repeat");
    conductor_and_waiting_thread(&sb);
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    // 1 周目: d の round 1 の最初の承認待ちでは起こす
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));

    // 2〜5 周目: 指揮役が受信箱を読み終え（既読が進む）、自分の finished と d の同じ round の
    // 承認待ちだけが続けて来る。受信箱には書くが、何回来ても起こしは増えない
    for cycle in 2..=5 {
        sb.patrol_state(&format!(r#"{{"inbox_offset":{}}}"#, inbox_lines(&sb)));
        conductor_finishes_and_d_waits_again(&sb);
        let before = inbox_lines(&sb);
        let out = sb.run_with(&["--once"]);
        assert!(out.status.success(), "cycle {cycle}: {}", stderr(&out));
        assert_eq!(inbox_lines(&sb), before + 2, "cycle {cycle}");
        assert_eq!(sends(&sb), 1, "cycle {cycle}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("WAKE skipped reason=repeat-waiting"),
            "cycle {cycle}: {}",
            stderr(&out)
        );
    }

    // d が次の round に進んでからの承認待ちでは、また起こす
    sb.patrol_state(&format!(r#"{{"inbox_offset":{}}}"#, inbox_lines(&sb)));
    sb.fake(
        "show-mcp_d.json",
        &show_round("mcp:d", "running", "2026-11-10T16:10:00Z", 1, 2),
    );
    let mut state = read_state(&sb);
    state["threads"]["mcp:d"]["pending_requests"] = 0.into();
    write_state(&sb, &state);
    let out = sb.run_with(&["--once"]);
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    assert_eq!(sends(&sb), 2, "{}", stderr(&out));
}

/// 起こしの最短間隔を 300 秒にして 1 周だけ走らせ、成功したことを確かめる。
fn run_cooldown(sb: &Sandbox) -> Output {
    let out = sb
        .command(&["--once"])
        .env("PELWAY_AGENT_WATCH_WAKE_COOLDOWN_SECS", "300")
        .output()
        .unwrap();
    assert!(out.status.success(), "stderr: {}", stderr(&out));
    out
}

/// b が失敗する一覧と詳細にし、状態では b を実行中に戻す（次の周回で b の failed が出る）。
fn b_fails_at(sb: &Sandbox, updated: &str) {
    sb.fake(
        "list.json",
        &format!(
            r#"[{{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}},
  {{"threadId":"mcp:b","status":"failed","updatedAt":"{updated}"}}]"#
        ),
    );
    sb.fake("show-mcp_b.json", &show("mcp:b", "failed", updated, 0));
    let mut state = read_state(sb);
    state["threads"]["mcp:b"]["status"] = "running".into();
    write_state(sb, &state);
}

/// 前回の起こしを、最短間隔より前のことにする。
fn backdate_last_wake(sb: &Sandbox) {
    let mut state = read_state(sb);
    let at = state["last_wake_at"]
        .as_i64()
        .expect("last_wake_at is recorded");
    state["last_wake_at"] = (at - 1000).into();
    write_state(sb, &state);
}

/// a の finished で 1 回起こし、最短間隔の内に b の failed を後に回す（受信箱 2 行、印の値は 2）。
/// `read` は b の failed の前に指揮役が読んだ行数（補助記録の `inbox_offset`）。
fn wake_then_defer_b_failure(sb: &Sandbox, read: u64) -> Output {
    one_event(sb);
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    let out = run_cooldown(sb);
    assert_eq!(sends(sb), 1, "{}", stderr(&out));
    sb.patrol_state(&format!(r#"{{"inbox_offset":{read}}}"#));
    b_fails_at(sb, "2026-11-10T16:30:00Z");
    let out = run_cooldown(sb);
    assert_eq!(sends(sb), 1, "{}", stderr(&out));
    assert_eq!(read_state(sb)["wake_pending"], 2);
    out
}

#[test]
fn notices_within_cooldown_wake_once_after_it() {
    let sb = Sandbox::new("wake-839-cooldown");
    // 最短間隔の内に、指揮役が読み終えてから b の failed が来た: 起こさず、起こし待ちの印を付ける
    let out = wake_then_defer_b_failure(&sb, 1);
    assert_eq!(inbox_lines(&sb), 2);
    assert!(
        stderr(&out).contains("WAKE deferred reason=cooldown"),
        "{}",
        stderr(&out)
    );

    // 新しい知らせの無い周回でも、最短間隔の内なら起こさない
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));

    // 最短間隔がたった: 新しい知らせが無くても、溜まった分で 1 回だけ起こす
    backdate_last_wake(&sb);
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 2, "{}", stderr(&out));
    assert!(read_state(&sb)["wake_pending"].is_null());

    // 次の周回では、もう起こさない
    backdate_last_wake(&sb);
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 2, "{}", stderr(&out));
}

#[test]
fn pending_wake_is_dropped_when_conductor_already_read() {
    let sb = Sandbox::new("wake-839-already-read");
    wake_then_defer_b_failure(&sb, 1);

    // 最短間隔の間に、指揮役が定期巡回で受信箱を末尾まで読んだ: 溜まった起こしは送らずに取り消す
    sb.patrol_state(r#"{"inbox_offset":2}"#);
    backdate_last_wake(&sb);
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("WAKE skipped reason=already-read"),
        "{}",
        stderr(&out)
    );
    assert!(read_state(&sb)["wake_pending"].is_null());
}

#[test]
fn quiet_lines_after_reading_do_not_revive_pending_wake() {
    // 起こし待ちの間に指揮役が読み終え、その後に起こさない行（指揮役自身の finished）だけが
    // 足されても、溜まった起こしは送らない（未読は起こさない行だけ）
    let sb = Sandbox::new("wake-839-quiet-after-read");
    let list = |b: &str, conductor: &str| {
        format!(
            r#"[{{"threadId":"mcp:a","status":"completed","updatedAt":"2026-11-10T16:10:00Z"}},
  {{"threadId":"mcp:b","status":"{b}","updatedAt":"2026-11-10T16:30:00Z"}},
  {{"threadId":"mcp:conductor-1","status":"{conductor}","updatedAt":"2026-11-10T16:40:00Z"}}]"#
        )
    };
    sb.fake("list.json", &list("running", "running"));
    sb.fake("show-mcp_a.json", SHOW_A_DONE);
    sb.fake(
        "show-mcp_b.json",
        &show("mcp:b", "running", "2026-11-10T16:30:00Z", 0),
    );
    sb.fake(
        "show-mcp_conductor-1.json",
        &show("mcp:conductor-1", "running", "2026-11-10T16:40:00Z", 0),
    );
    fs::write(
        sb.state_file(),
        r#"{"initialized":true,"threads":{
  "mcp:a":{"status":"running","updated_at":"2026-11-10T16:00:00Z","run_count":1,"pending_requests":0,"stalled_for":null},
  "mcp:b":{"status":"running","updated_at":"2026-11-10T16:30:00Z","run_count":1,"pending_requests":0,"stalled_for":"2026-11-10T16:30:00Z"},
  "mcp:conductor-1":{"status":"running","updated_at":"2026-11-10T16:40:00Z","run_count":1,"pending_requests":0,"stalled_for":"2026-11-10T16:40:00Z"}},
  "ci_notified":{}}"#,
    )
    .unwrap();
    sb.conductor("mcp:conductor-1");
    sb.patrol_state(r#"{"inbox_offset":0}"#);
    // a の finished で起こす
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));
    // 最短間隔の内に b の failed: 後に回す
    sb.patrol_state(r#"{"inbox_offset":1}"#);
    sb.fake("list.json", &list("failed", "running"));
    sb.fake(
        "show-mcp_b.json",
        &show("mcp:b", "failed", "2026-11-10T16:30:00Z", 0),
    );
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));
    assert_eq!(inbox_lines(&sb), 2);
    // 指揮役が末尾（2 行）まで読み、そのあと自分の巡回の finished が足された
    sb.patrol_state(r#"{"inbox_offset":2}"#);
    sb.fake("list.json", &list("failed", "completed"));
    sb.fake(
        "show-mcp_conductor-1.json",
        &show("mcp:conductor-1", "completed", "2026-11-10T16:40:00Z", 0),
    );
    let out = run_cooldown(&sb);
    assert_eq!(inbox_lines(&sb), 3);
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));
    // 最短間隔がたっても、未読は起こさない行だけなので送らない
    backdate_last_wake(&sb);
    let out = run_cooldown(&sb);
    assert_eq!(sends(&sb), 1, "{}", stderr(&out));
    assert!(
        stderr(&out).contains("WAKE skipped reason=already-read"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn pending_wake_is_not_resent_after_failure_and_stays_quiet() {
    let sb = Sandbox::new("wake-839-pending-fail");
    let send_calls = |sb: &Sandbox| {
        fs::read_to_string(sb.t3_calls())
            .unwrap_or_default()
            .lines()
            .filter(|l| l.starts_with("t3 thread send"))
            .count()
    };
    // 最短間隔の内に b の failed。指揮役はまだ読んでいない（既読は 0 のまま）
    wake_then_defer_b_failure(&sb, 0);

    // 間隔がたったが既読が進んでいない: 印は残し、印だけを確かめる周では理由のログを出さない
    backdate_last_wake(&sb);
    for _ in 0..2 {
        let out = run_cooldown(&sb);
        assert!(!stderr(&out).contains("WAKE"), "{}", stderr(&out));
    }
    assert_eq!(read_state(&sb)["wake_pending"], 2);
    assert_eq!(send_calls(&sb), 1);

    // 既読が進んだ周で送りに失敗した: 印を消し、次の周回で送り直さない
    sb.patrol_state(r#"{"inbox_offset":1}"#);
    sb.fake("t3-send-fail", "");
    let out = run_cooldown(&sb);
    assert!(stderr(&out).contains("WAKE failed"), "{}", stderr(&out));
    assert!(read_state(&sb)["wake_pending"].is_null());
    let out = run_cooldown(&sb);
    assert!(!stderr(&out).contains("WAKE"), "{}", stderr(&out));
    assert_eq!(send_calls(&sb), 2);
}

#[test]
fn pending_wake_is_dropped_when_unregistered() {
    let sb = Sandbox::new("wake-839-pending-unregistered");
    wake_then_defer_b_failure(&sb, 1);
    // 登録を外した（代行の終わり）: 後に回した起こしも取り消す
    fs::remove_file(
        sb.root
            .join("home/.config/pelway-agent-notify/conductor-thread-id"),
    )
    .unwrap();
    run_cooldown(&sb);
    assert!(read_state(&sb)["wake_pending"].is_null());
}
