//! QA-GDPR-13: 端末に会話・個人情報を残さないことの走査試験。
//!
//! 方針: このデモでは会話の本文・個人情報はメモリ上だけに置き、ディスク
//! （キャッシュファイル・設定・ログ・一時ファイル）に書かない。SQLite・
//! IndexedDB 等の本文キャッシュは作らない。共有コア（pelway-client-core）は
//! 通信もファイルの読み書きもしない（開発リード決定 2026-11-09）。
//!
//! 棚卸し（docs/worklog/client.md 第12枠）のとおり、現時点で書き込み・ログ出力の
//! 経路は存在しないため、この試験は「無いこと」を固定する:
//! - (a) 合成 canary 入りの会話状態をメモリに載せたあと、サンドボックス
//!   （`target/gdpr-13-sandbox/` 以下）と環境変数が指すディレクトリ（worktree 内の
//!   ものだけ）を走査し、canary が 0 件であること。
//! - (b) クライアントの動作中の標準出力・標準エラーに canary が出ないこと
//!   （このテストバイナリを子プロセスとして起動して吸い取る）。
//! - (c) 陽性対照: 検査関数に canary を含む文字列を渡すと「見つかった」と判定される
//!   こと（常に 0 件を返す欠陥を通さない）。
//! - (d) 共有コアに本文を書く API が無いこと（ソースの禁止語走査 +
//!   「本文を渡してもファイルができない」振る舞い）。
//! - (e) 走査の対象パスが worktree の外（`/tmp`・本物のホーム等）を指していないこと。
//!
//! canary について: 本文用・個人情報用の 2 つの合成トークン（実在の値ではない）を
//! 使う。完全な文字列をどこにも書かない: 完全な canary がソースやコンパイル済み
//! バイナリ（`target/` の成果物）に入ると走査の意味が無くなるため、前半を平文・
//! 後半を逆順に格納し、実行時に組み立てる。単純な 2 分割では不十分だった——
//! リンカが 2 つのリテラルを `.rodata` 上で隣接させたとき、バイト列として
//! 連結した完全な canary がバイナリに現れることを実際の走査で確認した
//! （docs/worklog/client.md 第12枠）。逆順格納ならどう並んでも連結できない。
//! 陽性対照は検査関数への引数としてだけ使い、ファイルには残さない（一時ファイルは
//! 確認後すぐ消す）。
//!
//! 窓は開かない（slint の backend も初期化しない。`MainWindow` も作らない）。
//! ウィンドウを開いたときだけ起きる端末残存はこの試験の対象外（未検証。

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use pelway_client_core::approval::{ApprovalMachine, CardView, CommentAttempt, Effect};
use pelway_client_core::contract::{ApproveError, CardId, TaskState};
use pelway_client_core::input::{Action, Focus, Key, Modifiers};
use pelway_client_ui::bridge::ShellBridge;
use pelway_client_ui::i18n::{FluentVars, Messages};
use pelway_client_ui::state::{TimelineEntry, sample};
use pelway_client_ui::testkit::{FixedClock, FixedKeySource};

// ---- canary（合成トークン。前半平文＋後半逆順の実行時組み立て。上の説明参照） ----

/// 本文用の canary。完全な文字列はどこにも書かない（先頭の説明のとおり、
/// 単純分割はリテラルの隣接配置で破られるため、後半は逆順に格納する）。
fn canary_body() -> String {
    let mut whole = String::from("PELWAY-CANARY-CLIENT-B");
    whole.push_str(&"27WK3R-YDO".chars().rev().collect::<String>());
    whole
}

/// 個人情報の代わり（表示名などの合成トークン）の canary。同じ組み立て方。
fn canary_pii() -> String {
    let mut whole = String::from("PELWAY-CANARY-CLIENT-PI");
    whole.push_str(&"6P4N8H-I".chars().rev().collect::<String>());
    whole
}

/// 走査で探す canary の一覧。
fn canaries() -> Vec<String> {
    vec![canary_body(), canary_pii()]
}

// ---- 検査関数（走査） ----

/// バイト列の中に needle があるか（エンコーディングを問わないバイト単位の照合）。
fn bytes_contain(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// ディレクトリ以下を再帰的に走査し、canary を含むファイルのパスを返す。
/// 読めない場所は何も無いのと同じ扱い。symlink は辿らない（worktree の外へ
/// 出ないようにする安全側）。
fn scan_tree(root: &Path, needles: &[String]) -> Vec<PathBuf> {
    let mut hits = Vec::new();
    walk_tree(root, needles, &mut hits);
    hits
}

fn walk_tree(dir: &Path, needles: &[String], hits: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_tree(&path, needles, hits);
        } else if file_type.is_file() {
            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            if needles.iter().any(|n| bytes_contain(&bytes, n.as_bytes())) {
                hits.push(path);
            }
        }
    }
}

// ---- 走査対象のパス（worktree の中だけ） ----

/// ワークスペースのルート（`crates/pelway-client-ui` から 2 つ上）。
fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .expect("manifest は crates/pelway-client-ui にあるため、2 つ上が worktree のルート");
    root.to_path_buf()
}

/// パスが root の中かどうかの純粋な（ファイルシステムに触れない）判定。
/// `..` で root の外へ出るパスは外とみなす。
fn lexically_inside(root: &Path, candidate: &Path) -> bool {
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let mut components: Vec<Component<'_>> = Vec::new();
    for component in joined.components() {
        match component {
            Component::ParentDir => match components.last() {
                None | Some(Component::RootDir) => return false,
                Some(_) => {
                    components.pop();
                }
            },
            Component::CurDir => {}
            other => components.push(other),
        }
    }
    PathBuf::from_iter(components).starts_with(root)
}

/// 候補のうち root の中にあるものだけを残す。外の候補はファイルシステムに
/// 触れずに捨てる（`/tmp` や本物のホームは読まない）。
fn keep_inside(root: &Path, candidates: &[PathBuf]) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for candidate in candidates {
        if lexically_inside(root, candidate) {
            kept.push(candidate.clone());
        } else {
            dropped.push(candidate.clone());
        }
    }
    (kept, dropped)
}

/// 走査対象のルート一覧。サンドボックス（`target/gdpr-13-sandbox`）と、環境変数
/// （HOME・TMPDIR・XDG_*）が指すディレクトリのうち worktree 内のもの。
fn scan_roots() -> Vec<PathBuf> {
    let root = workspace_root();
    let sandbox = root.join("target").join("gdpr-13-sandbox");
    // サンドボックスが無ければ作る（環境変数の向け先が整っていなくても走査はできる）。
    let _ = fs::create_dir_all(&sandbox);
    let mut candidates = vec![sandbox];
    for key in [
        "HOME",
        "TMPDIR",
        "XDG_DATA_HOME",
        "XDG_CACHE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_STATE_HOME",
    ] {
        if let Some(value) = std::env::var_os(key) {
            candidates.push(PathBuf::from(value));
        }
    }
    let (mut kept, _dropped) = keep_inside(&root, &candidates);
    kept.sort();
    kept.dedup();
    kept
}

/// サンドボックス全体を走査する試験どうしの並走を避ける鍵（(a)(c)(d) で共有）。
/// 陽性対照 (c) が一時的に canary 入りファイルを作るため、(a)(d) の走査と
/// 直列に動かす。
fn sandbox_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// ---- クライアントを canary 入りで動かす（メモリ上だけ） ----

/// 走査試験で使う承認カード（架空の値。canary は本文・表示名にだけ入れる）。
fn gdpr13_card() -> CardView {
    CardView {
        card_id: CardId::from("card-gdpr-13"),
        card_version: 1,
        snapshot_hash: "hash-gdpr-13".to_owned(),
        state: TaskState::AwaitingSendApproval,
    }
}

/// 表示用の状態と中核と橋に canary を載せて一通り動かす。出力もファイル書き込みも
/// しない（この関数の呼び出し中に端末やファイルへ出るものがあれば (a)(b)(d) が落ちる）。
fn exercise_client_with_canaries(body: &str, pii: &str) {
    // 会話状態（表示用の型。メモリ上）: 表示名を個人情報 canary に、
    // メッセージ本文・「あなただけに表示」・社外送信カード本文を本文 canary にする。
    let mut data = sample();
    data.rail.viewing_as = pii.to_owned();
    if let Some(person) = data.rail.people.first_mut() {
        person.name = pii.to_owned();
    }
    for entry in &mut data.channel.timeline {
        match entry {
            TimelineEntry::Message(message) => message.body = body.to_owned(),
            TimelineEntry::PrivateNote { body: note, .. } => *note = body.to_owned(),
            TimelineEntry::ExternalSend(card) => card.body = body.to_owned(),
            _ => {}
        }
    }
    assert!(
        data.channel
            .timeline
            .iter()
            .any(|entry| matches!(entry, TimelineEntry::Message(m) if m.body == body)),
        "会話状態に本文 canary が載っていること（メモリ上）"
    );

    // 中核: コメント本文（canary 入り）を渡して保存要求を作り、承認まで動かす。
    let mut machine =
        ApprovalMachine::new(gdpr13_card(), FixedKeySource::new(), FixedClock::new(0));
    let attempt = machine.request_save_comment(format!("確認します: {body}"));
    assert!(
        matches!(&attempt, CommentAttempt::Send(req) if req.body.contains(body)),
        "コメント保存要求が本文を運んでいること（メモリ上）"
    );
    assert!(matches!(machine.press_approve(), Some(Effect::Approve(_))));
    let _ = machine.on_approve_response(Err(ApproveError::ApprovalInProgress));
    let _ = machine.poll();

    // 橋: 承認ボタンでの Enter を中核まで通す。
    let mut shell = ShellBridge::new(gdpr13_card());
    shell.set_focus(Focus::ApproveButton);
    assert_eq!(
        shell.handle_key(Key::Enter, Modifiers::default()),
        Action::Approve
    );
    assert_eq!(shell.approval().approve_request_count(), 1);

    // 表示名（個人情報の代わり）が Fluent の整形を通るのはメモリ内だけ。
    let mut vars = FluentVars::sample();
    vars.name = pii.to_owned();
    let messages = Messages::new("ja").expect("ja の bundle が作れること");
    let formatted = messages
        .message("thread-working", &vars)
        .expect("thread-working が引けること");
    assert!(
        formatted.contains(pii),
        "整形結果に表示名が入っていること（メモリ上）"
    );
}

// ---- (a) ----

/// canary 入りの会話状態をメモリに載せたあと、走査対象に canary が 0 件であること。
#[test]
fn conversation_state_with_canaries_writes_no_files() {
    let _guard = sandbox_lock().lock().unwrap_or_else(|p| p.into_inner());
    exercise_client_with_canaries(&canary_body(), &canary_pii());

    let root = workspace_root();
    assert!(
        root.join("Cargo.lock").is_file(),
        "走査の基準はワークスペースのルートであること"
    );

    let roots = scan_roots();
    assert!(!roots.is_empty(), "走査対象が少なくとも 1 つあること");
    let mut hits = Vec::new();
    for scan_root in &roots {
        hits.extend(scan_tree(scan_root, &canaries()));
    }
    assert!(
        hits.is_empty(),
        "canary を含むファイルが見つかった（件数 {}）: {:?}",
        hits.len(),
        hits
    );
}

// ---- (b) ----

/// 子プロセスの印（このテストバイナリ自身を再実行するときに使う）。
const CHILD_FLAG: &str = "PELWAY_QA_GDPR_13_CHILD";

/// クライアントの動作中の標準出力・標準エラーに canary が出ないこと。
/// このテストバイナリを子プロセスとして起動し、canary 入りの状態を動かしたときの
/// 出力を吸い取る（ファイルログの仕組みは無いため、標準エラーの吸み取りで確かめる）。
#[test]
fn client_output_streams_carry_no_canary() {
    if std::env::var_os(CHILD_FLAG).is_some() {
        // 子プロセス側: canary を載せて動かすだけ。何も出力しない。
        exercise_client_with_canaries(&canary_body(), &canary_pii());
        std::process::exit(0);
    }
    let exe = std::env::current_exe().expect("テストバイナリのパスが取れること");
    let output = std::process::Command::new(exe)
        .arg("--exact")
        .arg("client_output_streams_carry_no_canary")
        // 子プロセスでは libtest の出力捕捉を切る。捕捉が有効だとテスト関数内の
        // eprintln! 等が libtest に溜め込まれて親から見えなくなり、この検査が
        // 空振りする（注入による赤確認で確認済み）。
        .arg("--nocapture")
        .env(CHILD_FLAG, "1")
        .output()
        .expect("子プロセスを起動できること");
    assert!(
        output.status.success(),
        "子プロセスが正常に終了すること: {:?}",
        output.status.code()
    );
    for (name, stream) in [("stdout", &output.stdout), ("stderr", &output.stderr)] {
        for needle in &canaries() {
            assert!(
                !bytes_contain(stream, needle.as_bytes()),
                "クライアントの動作中の {name} に canary が出ていないこと"
            );
        }
    }
}

// ---- (c) ----

/// 陽性対照。検査関数は canary を含む入力を「見つかった」と判定すること。
/// canary は検査関数への引数としてだけ使い、ファイルに残さない（一時ファイルは
/// 確認後すぐ消す）。
#[test]
fn scanner_detects_canaries_in_memory_and_in_files() {
    let _guard = sandbox_lock().lock().unwrap_or_else(|p| p.into_inner());
    let needles = canaries();

    // メモリ上の文字列（引数）: 2 つの canary とも必ず見つかること。
    let mut bait = String::from("prefix ");
    bait.push_str(&canary_body());
    bait.push_str(" middle ");
    bait.push_str(&canary_pii());
    bait.push_str(" suffix");
    let bait_bytes = bait.into_bytes();
    for needle in &needles {
        assert!(
            bytes_contain(&bait_bytes, needle.as_bytes()),
            "canary を含む入力を検出できること"
        );
    }
    // canary を含まない入力には反応しないこと。
    let clean = "通常の会話の本文にも表示名にも目印は無い".as_bytes();
    assert!(
        !needles
            .iter()
            .any(|needle| bytes_contain(clean, needle.as_bytes()))
    );

    // ファイル経路: scan_tree が canary 入りファイルを必ず検出できること
    // （走査が常に空を返す欠陥を通さない）。ファイルは消す。
    let probe = workspace_root()
        .join("target")
        .join("gdpr-13-sandbox")
        .join("positive-control");
    fs::create_dir_all(&probe).expect("陽性対照用のディレクトリを作れること");
    let probe_file = probe.join("bait.txt");
    fs::write(&probe_file, &bait_bytes).expect("陽性対照用のファイルを書けること");
    assert_eq!(
        scan_tree(&probe, &needles),
        vec![probe_file.clone()],
        "canary 入りファイルを検出できること"
    );
    fs::remove_file(&probe_file).expect("陽性対照のファイルを消すこと");
    fs::remove_dir(&probe).expect("陽性対照のディレクトリを消すこと");
    assert!(!probe.exists(), "陽性対照のファイルを残していないこと");
}

// ---- (d) ----

/// 共有コアのソースに置いてはいけない語（ファイル入出力・出力・プロセス起動）。
/// core は通信もファイルの読み書きもしないため、本文を書く API は無いことを
/// この一覧で固定する。core のコメントにこれらの語を書くだけでも落ちる
/// （書き方の自由を狭めるが、方針を固定する安全側）。
fn core_forbidden_tokens() -> Vec<String> {
    [
        "std::fs",
        "OpenOptions",
        "File::",
        "fs::",
        "read_dir",
        "create_dir",
        "remove_file",
        "remove_dir",
        "hard_link",
        "write!",
        "writeln!",
        "print!",
        "println!",
        "eprint!",
        "eprintln!",
        "dbg!",
        "std::process",
        "Command",
    ]
    .iter()
    .map(|token| (*token).to_owned())
    .collect()
}

/// ディレクトリ以下の `.rs` ファイルを列挙する。
fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 共有コアに本文を書く API が無いこと。① ソースの禁止語走査、② `[dependencies]`
/// が空のまま（依存ゼロ＝fs をする crate も入らない）、③ 振る舞いとして
/// 「本文を渡してもファイルができない」ことで固定する。
#[test]
fn core_api_surface_has_no_file_io() {
    let _guard = sandbox_lock().lock().unwrap_or_else(|p| p.into_inner());
    let root = workspace_root();

    // ① ソース走査。
    let core_src = root.join("crates").join("pelway-client-core").join("src");
    let mut files = Vec::new();
    collect_rs_files(&core_src, &mut files);
    assert!(!files.is_empty(), "core のソースが見つかること");
    let tokens = core_forbidden_tokens();
    let mut source_hits = Vec::new();
    for path in &files {
        let text = fs::read_to_string(path).expect("core のソースが読めること");
        for token in &tokens {
            if text.contains(token.as_str()) {
                source_hits.push(format!("{}: {}", path.display(), token));
            }
        }
    }
    assert!(
        source_hits.is_empty(),
        "core のソースにファイル入出力・出力・プロセス起動の禁止語がある:\n{}",
        source_hits.join("\n")
    );

    // ② Cargo.toml の [dependencies] が空のまま。
    let core_manifest = root
        .join("crates")
        .join("pelway-client-core")
        .join("Cargo.toml");
    let manifest_text =
        fs::read_to_string(&core_manifest).expect("core の Cargo.toml が読めること");
    let rest = manifest_text
        .split("[dependencies]")
        .nth(1)
        .expect("core の Cargo.toml に [dependencies] があること");
    let section = &rest[..rest.find('[').unwrap_or(rest.len())];
    let has_dependency = section.lines().any(|line| {
        let trimmed = line.trim();
        !trimmed.is_empty() && !trimmed.starts_with('#')
    });
    assert!(
        !has_dependency,
        "core の [dependencies] は空のまま（依存を足さない）"
    );

    // ③ 振る舞い: 空の探査ディレクトリを用意し、canary 入りの本文を渡して
    // 中核と橋を動かしても、ファイルが 1 つもできないこと。
    let probe = root
        .join("target")
        .join("gdpr-13-sandbox")
        .join("core-fs-probe");
    fs::create_dir_all(&probe).expect("探査ディレクトリを作れること");
    for entry in fs::read_dir(&probe)
        .expect("探査ディレクトリが読めること")
        .flatten()
    {
        let path = entry.path();
        if path.is_file() {
            fs::remove_file(&path).expect("前回の実行残りのファイルを消すこと");
        }
    }
    exercise_client_with_canaries(&canary_body(), &canary_pii());
    let remaining = fs::read_dir(&probe)
        .expect("探査ディレクトリが読めること")
        .flatten()
        .count();
    assert_eq!(remaining, 0, "本文を渡してもファイルができないこと");

    // サンドボックス全体にも canary が無いこと（(a) と同じ照合のこの試験内での確認）。
    let mut hits = Vec::new();
    for scan_root in scan_roots() {
        hits.extend(scan_tree(&scan_root, &canaries()));
    }
    assert!(
        hits.is_empty(),
        "canary を含むファイルが見つかった: {hits:?}"
    );
}

// ---- (e) ----

/// 走査の対象パスが worktree の外を指していないこと。
#[test]
fn scan_roots_stay_inside_the_worktree() {
    let root = workspace_root();

    // 純粋な判定: worktree の中は中、外は外（`..` で外へ出るものも外）。
    assert!(lexically_inside(&root, &root));
    assert!(lexically_inside(
        &root,
        &root.join("target").join("gdpr-13-sandbox").join("tmp")
    ));
    let outside = [
        PathBuf::from("/tmp"),
        PathBuf::from("/tmp/opencode-example"),
        PathBuf::from("/home/definitely-not-a-real-home"),
        root.join("target").join("..").join("..").join("elsewhere"),
        PathBuf::from("../sibling-of-worktree"),
    ];
    for candidate in &outside {
        assert!(
            !lexically_inside(&root, candidate),
            "worktree の外は外と判定すること: {}",
            candidate.display()
        );
    }
    let (kept, dropped) = keep_inside(&root, &outside);
    assert!(kept.is_empty(), "外の候補は残らないこと");
    assert_eq!(
        dropped.len(),
        outside.len(),
        "外の候補はすべて除外されること"
    );

    // 実際の走査対象: すべて worktree の中で、サンドボックスを含むこと。
    let roots = scan_roots();
    assert!(!roots.is_empty(), "走査対象が少なくとも 1 つあること");
    for scan_root in &roots {
        assert!(
            lexically_inside(&root, scan_root),
            "走査対象が worktree の中であること: {}",
            scan_root.display()
        );
    }
    assert!(
        roots
            .iter()
            .any(|path| path.ends_with("target/gdpr-13-sandbox")),
        "サンドボックスが走査対象に含まれること"
    );

    // 本物のホームが worktree の外にある実行（サンドボックス環境変数なしの実行）では、
    // それを走査対象にしないこと（外は読まない）。
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        if !lexically_inside(&root, &home) {
            assert!(!roots.contains(&home), "worktree の外の HOME は走査しない");
        }
    }
}
