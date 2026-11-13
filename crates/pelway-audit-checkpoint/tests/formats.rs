//! 形式検査とエラー経路のテスト（Issue #158）。合成データだけを使う。
//!
//! `tamper.rs` が改ざん検出の受け入れ条件を確かめるのに対し、こちらは
//! signed-note 本文・署名行・チェックポイント本文・保管先ログの形式エラーが
//! 期待どおりの `Err` で拒否されることを確かめる。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::SigningKey;

use pelway_audit_checkpoint::chain::GENESIS;
use pelway_audit_checkpoint::checkpoint::{self, CheckpointError, VerifyError, parse_note_text};
use pelway_audit_checkpoint::note::{self, NoteError, NoteSigner, NoteVerifier};
use pelway_audit_checkpoint::store::CheckpointLog;

const ORIGIN: &str = "pelway.example/w/ws-1";

/// 有効な Ed25519 公開鍵（合成の固定秘密鍵から導出）。
fn public_key_bytes() -> [u8; 32] {
    SigningKey::from_bytes(&[0x77; 32])
        .verifying_key()
        .to_bytes()
}

/// `tamper.rs` と同じ方式の一意な一時ファイル名（ファイル自体は作らない）。
fn tmp_path(tag: &str) -> PathBuf {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::var("CARGO_TARGET_TMPDIR")
        .map_or_else(|_| std::env::temp_dir().join("opencode"), PathBuf::from);
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!(
        "checkpoint-format-{tag}-{}-{}.log",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

// --- チェックポイント本文（3 行）の形式 ---

#[test]
fn parse_note_text_rejects_wrong_line_count() {
    let hash = BASE64.encode([0u8; 32]);
    // 1 行・2 行・4 行はすべて形式エラー。
    assert_eq!(parse_note_text("o\n"), Err(CheckpointError::Lines));
    assert_eq!(parse_note_text("o\n1\n"), Err(CheckpointError::Lines));
    assert_eq!(
        parse_note_text(&format!("o\n1\n{hash}\nextra\n")),
        Err(CheckpointError::Lines)
    );
    // 最終改行が無い本文も形式エラー。
    assert_eq!(
        parse_note_text(&format!("o\n1\n{hash}")),
        Err(CheckpointError::Lines)
    );
    // 正しい 3 行は読める。
    let cp = parse_note_text(&format!("o\n0\n{hash}\n")).unwrap();
    assert_eq!(cp.size, 0);
}

#[test]
fn parse_note_text_rejects_bad_origin() {
    let hash = BASE64.encode([0u8; 32]);
    // origin は空・Unicode 空白・`+` を含まない（署名行の鍵名と同じ要件）。
    for origin in ["", "has space", "has\u{3000}space", "has+plus"] {
        assert_eq!(
            parse_note_text(&format!("{origin}\n1\n{hash}\n")),
            Err(CheckpointError::Lines),
            "origin {origin:?} must be rejected"
        );
    }
}

#[test]
fn parse_note_text_rejects_bad_size_and_hash() {
    let hash = BASE64.encode([0u8; 32]);
    // 行数行: 先頭の 0 は認めない、10 進のみ、u64 まで。
    for size in ["05", "abc", "1_000", "18446744073709551616"] {
        assert_eq!(
            parse_note_text(&format!("o\n{size}\n{hash}\n")),
            Err(CheckpointError::Size),
            "size {size:?} must be rejected"
        );
    }
    // ハッシュ行: base64 でなければならず、デコード後は 32 バイト。
    assert_eq!(parse_note_text("o\n1\n!!!\n"), Err(CheckpointError::Hash));
    assert_eq!(
        parse_note_text(&format!("o\n1\n{}\n", BASE64.encode([0u8; 31]))),
        Err(CheckpointError::Hash)
    );
}

/// 行数 0 のチェックポイントは鎖の起点（GENESIS）と照合する（空の鎖でも
/// 署名・検証が回る境界）。
#[test]
fn empty_chain_checkpoint_uses_genesis() {
    let signer = NoteSigner::from_bytes(ORIGIN, [0x42; 32]);
    let signed = checkpoint::sign_checkpoint(&[], ORIGIN, &signer).unwrap();
    let cp =
        checkpoint::verify_checkpoint(&[], &signed, &[signer.verifying_key()], ORIGIN).unwrap();
    assert_eq!(cp.size, 0);
    assert_eq!(cp.hash, GENESIS);
}

// --- 鍵名と鍵の素材 ---

#[test]
fn verifier_rejects_bad_key_names() {
    let pk = public_key_bytes();
    for name in ["", "has space", "has\ttab", "has+plus"] {
        assert!(
            matches!(
                NoteVerifier::from_public_key(name, &pk),
                Err(NoteError::Format)
            ),
            "key name {name:?} must be rejected"
        );
    }
    // 曲線上の点でない公開鍵も形式エラー（y = 2 は x の平方根を持たず
    // 圧縮形式が展開できない）。
    let mut bad_pk = [0u8; 32];
    bad_pk[0] = 2;
    assert!(matches!(
        NoteVerifier::from_public_key("pelway.test/k", &bad_pk),
        Err(NoteError::Format)
    ));
}

#[test]
fn generated_signer_signs_and_verifies() {
    let signer = NoteSigner::generate("pelway.test/audit");
    assert_eq!(signer.key_name(), "pelway.test/audit");
    assert_eq!(signer.key_id().len(), note::KEY_ID_LEN);
    let text = "body\n";
    let signed = signer.sign_note_text(text);
    assert_eq!(
        note::verify_note(&signed, &[signer.verifying_key()]).unwrap(),
        text
    );
}

// --- signed-note の形式 ---

#[test]
fn verify_note_rejects_control_characters() {
    // U+0020 未満の制御文字は改行だけを許す。
    assert_eq!(
        note::verify_note("t\x01ext\n\n— n Zg==\n", &[]),
        Err(NoteError::Format)
    );
}

#[test]
fn verify_note_rejects_missing_separator_or_newline() {
    // 本文と署名行を区切る最後の空行が無い。
    assert_eq!(
        note::verify_note("single line\n", &[]),
        Err(NoteError::Format)
    );
    // 本文が空。
    assert_eq!(
        note::verify_note("\n\n— n Zg==\n", &[]),
        Err(NoteError::Format)
    );
    // 署名行が改行で終わらない。
    assert_eq!(
        note::verify_note("t\n\n— n Zg==", &[]),
        Err(NoteError::Format)
    );
}

#[test]
fn verify_note_rejects_malformed_signature_lines() {
    let signer = NoteSigner::from_bytes("n", [0x42; 32]);
    let verifier = signer.verifying_key();
    // 署名行が「— 」で始まらない。
    assert_eq!(
        note::verify_note("t\n\nnope\n", std::slice::from_ref(&verifier)),
        Err(NoteError::Format)
    );
    // 鍵名と値の区切り（空白）が無い。
    assert_eq!(
        note::verify_note("t\n\n— onlykeyname\n", std::slice::from_ref(&verifier)),
        Err(NoteError::Format)
    );
    // 署名行の鍵名も要件を満たすこと（空白を含む鍵名は拒否）。
    assert_eq!(
        note::verify_note("t\n\n— bad name Zg==\n", std::slice::from_ref(&verifier)),
        Err(NoteError::Format)
    );
    // 値が base64 でない。
    assert_eq!(
        note::verify_note("t\n\n— n !!!\n", std::slice::from_ref(&verifier)),
        Err(NoteError::Format)
    );
    // デコード結果が鍵 ID 未満。
    assert_eq!(
        note::verify_note("t\n\n— n Zg==\n", std::slice::from_ref(&verifier)),
        Err(NoteError::Format)
    );
}

/// 鍵名と鍵 ID が既知の鍵に一致する署名行の署名部分が 64 バイトでなければ
/// 形式エラー（署名検証ではなく形式の失敗として拒否する）。
#[test]
fn verify_note_rejects_short_signature_for_known_key() {
    let signer = NoteSigner::from_bytes("n", [0x42; 32]);
    let verifier = signer.verifying_key();
    // 鍵 ID は検証鍵のものを流用し、署名部分だけ短くしたペイロードを作る。
    let mut payload = verifier.key_id().to_vec();
    payload.extend_from_slice(&[0xaa; 3]);
    let note_text = format!("t\n\n— n {}\n", BASE64.encode(payload));
    assert_eq!(
        note::verify_note(&note_text, &[verifier]),
        Err(NoteError::Format)
    );
}

/// 署名検証が失敗しても note 検証の Err として verify_checkpoint に伝わる
/// （VerifyError::Note の経路）。
#[test]
fn verify_checkpoint_propagates_note_errors() {
    let err = checkpoint::verify_checkpoint(&[], "not a note\n", &[], ORIGIN).unwrap_err();
    assert_eq!(err, VerifyError::Note(NoteError::Format));
}

// --- 保管先ログ ---

#[test]
fn entries_on_missing_file_is_empty() {
    // ファイルを作らないパスを指す（先に残っていても消す）。
    let path = tmp_path("missing");
    let _ = std::fs::remove_file(&path);
    let log = CheckpointLog::open(&path);
    assert_eq!(log.entries().unwrap(), Vec::<String>::new());
}

#[test]
fn entries_rejects_unreadable_file() {
    // ファイルが無い以外の open の失敗（権限拒否）は、空ではなく Err として返る。
    let path = tmp_path("unreadable");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, b"x").unwrap();
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o000);
    }
    std::fs::set_permissions(&path, perms).unwrap();
    let log = CheckpointLog::open(&path);
    assert!(log.entries().is_err());
    // 残骸を消せるように権限を戻す。
    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o600);
    }
    std::fs::set_permissions(&path, perms).unwrap();
}

#[test]
fn entries_rejects_truncated_payload() {
    // 長さ前置きだけ有効で本文が切れている保管先は読み取りエラー。
    let path = tmp_path("truncated");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, [0, 0, 0, 10, b'a', b'b']).unwrap();
    let log = CheckpointLog::open(&path);
    assert!(log.entries().is_err());
}
