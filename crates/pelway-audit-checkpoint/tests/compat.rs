//! 依存更新（ed25519-dalek 2 → 3）の互換の受け入れ条件（Issue #226）。
//! 固定値だけを使う。
//!
//! 確かめること: 更新前の ed25519-dalek 2.2.0 で作った鍵と署名付きチェック
//! ポイントが、更新後の版でも同じように検証できる。署名・検証の形式は
//! RFC 8032 Ed25519 のままで、2.x から 3.x の変更履歴に形式の変更は無いため、
//! バイト列を保存したチェックポイントはそのまま検証できるはずである。

use pelway_audit_checkpoint::chain::{self, AuditEvent, StoredRow};
use pelway_audit_checkpoint::checkpoint::{self, VerifyError};
use pelway_audit_checkpoint::note::{self, NoteSigner};

const ORIGIN: &str = "pelway.example/w/ws-1";

/// 2.2.0 で `NoteSigner::from_bytes(ORIGIN, [0x42; 32])` が署した 5 行時点の
/// チェックポイント（`tamper.rs` の `events(5)` と同じ合成データ）。署名行の
/// em dash は U+2014。更新後もこのバイト列のまま検証できなければならない。
const V2_SIGNED: &str = concat!(
    "pelway.example/w/ws-1\n",
    "5\n",
    "0ofTo62ToUJSNJDabBr9CFn400L5HReb+rL+WNuEma0=\n",
    "\n",
    "\u{2014} pelway.example/w/ws-1 ",
    "CnfJO+4BPLoqKAWNNKyX8Mjs/09K3vDIgb+HwbA6zAYN3uxVN4ne+qfMs6BCRzDlCnWqsbr5ur1ThfY0jddWZSPhDQA=\n",
);

/// `tamper.rs` と同じ合成イベント（固定値の鎖と同じ行を作る）。
fn events(n: usize) -> Vec<AuditEvent> {
    (0..n)
        .map(|i| AuditEvent {
            workspace_id: "ws-1".to_owned(),
            at_ms: 1_700_000_000_000 + i as u64,
            actor: if i % 2 == 0 { "bot-1" } else { "human-1" }.to_owned(),
            subject: format!("task-{i}"),
            kind: "tool_write".to_owned(),
            result: "ok".to_owned(),
            snapshot_hash: [i as u8; 32],
            policy_version: 3,
        })
        .collect()
}

fn rows_of(n: usize) -> Vec<StoredRow> {
    chain::build_chain(events(n))
}

/// 2.x の固定値の署名入りチェックポイントが、現行の版でも検証を通る。
#[test]
fn v2_signed_checkpoint_verifies() {
    let verifier = NoteSigner::from_bytes(ORIGIN, [0x42; 32]).verifying_key();
    let rows = rows_of(5);
    let cp =
        checkpoint::verify_checkpoint(&rows, V2_SIGNED, std::slice::from_ref(&verifier), ORIGIN)
            .unwrap();
    assert_eq!(cp.size, 5);
    assert_eq!(cp.hash, rows[4].hash);
    assert_eq!(cp.origin, ORIGIN);
}

/// 同じ鍵・同じ本文への再署名が 2.x の固定値とバイト列まで一致する
/// （Ed25519 の署名は決定的なので、版が変わっても署名バイトは変わらない）。
#[test]
fn v2_signature_bytes_are_reproduced() {
    let signer = NoteSigner::from_bytes(ORIGIN, [0x42; 32]);
    let signed = checkpoint::sign_checkpoint(&rows_of(5), ORIGIN, &signer).unwrap();
    assert_eq!(&signed, V2_SIGNED);
}

/// 2.x の固定値も、改ざんに対しては現行の版と同じに拒否される。
#[test]
fn v2_fixed_value_rejects_tampering() {
    let verifier = NoteSigner::from_bytes(ORIGIN, [0x42; 32]).verifying_key();
    let rows = rows_of(5);
    let tampered = V2_SIGNED.replacen("\n5\n", "\n4\n", 1);
    assert_ne!(tampered, V2_SIGNED);
    let err =
        checkpoint::verify_checkpoint(&rows, &tampered, std::slice::from_ref(&verifier), ORIGIN)
            .unwrap_err();
    assert_eq!(err, VerifyError::Note(note::NoteError::InvalidSignature));
}
