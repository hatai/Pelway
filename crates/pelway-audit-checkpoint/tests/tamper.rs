//! 改ざん検出の受け入れ条件（Issue #102）。合成データだけを使う。
//!
//! 確かめること: DB の管理者が鎖全体を書き換えて再計算しても、保管済みチェック
//! ポイントとの照合で見つかる。鎖の歩査だけでは検出できないケース（全書き換え）を
//! チェックポイントが担う。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use pelway_audit_checkpoint::chain::{self, AuditEvent, StoredRow};
use pelway_audit_checkpoint::checkpoint::{self, VerifyError};
use pelway_audit_checkpoint::note::{self, NoteSigner};
use pelway_audit_checkpoint::store::CheckpointLog;

const ORIGIN: &str = "pelway.example/w/ws-1";

/// 偶数行を bot-1、奇数行を human-1 が実行した合成イベント。
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

/// k 行時点の署名済みチェックポイントとその検証鍵。
fn signed_at(k: usize) -> (String, note::NoteVerifier) {
    let signer = NoteSigner::from_bytes(ORIGIN, [0x42; 32]);
    let signed = checkpoint::sign_checkpoint(&rows_of(k), ORIGIN, &signer).unwrap();
    (signed, signer.verifying_key())
}

/// テストごとに一意な一時ファイル（cargo が用意するテスト用ディレクトリの下。
/// 実行ごとに残骸が残らないよう、名前は PID と通し番号で一意にして先に消す）。
fn tmp_log(tag: &str) -> (CheckpointLog, PathBuf) {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::var("CARGO_TARGET_TMPDIR")
        .map_or_else(|_| std::env::temp_dir().join("opencode"), PathBuf::from);
    let path = dir.join(format!(
        "checkpoint-{tag}-{}-{}.log",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    // 前回の実行が同じ PID で残した場合に備えて、先に消しておく（無くてもよい）。
    let _ = std::fs::remove_file(&path);
    (CheckpointLog::open(&path), path)
}

/// 改ざんがなければ検証が通り、行数と最新ハッシュが読める。
#[test]
fn intact_chain_verifies() {
    let (signed, vkey) = signed_at(5);
    let rows = rows_of(5);
    let cp = checkpoint::verify_checkpoint(&rows, &signed, &[vkey], ORIGIN).unwrap();
    assert_eq!(cp.size, 5);
    assert_eq!(cp.hash, rows[4].hash);
    assert_eq!(cp.origin, ORIGIN);
}

/// DB の管理者が行を書き換えて鎖全体を再計算しても、チェックポイントとの照合で
/// 見つかる（鎖の歩査だけでは検出できないケース）。
#[test]
fn full_rewrite_with_rechained_hashes_is_detected() {
    let (signed, vkey) = signed_at(5);
    let mut evs = events(5);
    evs[2].kind = "tool_read".to_owned();
    let rewritten = chain::build_chain(evs);
    assert_eq!(
        chain::walk(&rewritten).unwrap().len(),
        5,
        "書き換え後の鎖は内部で一貫している（歩査だけでは検出できない）"
    );
    let err = checkpoint::verify_checkpoint(&rewritten, &signed, &[vkey], ORIGIN).unwrap_err();
    assert!(matches!(
        err,
        VerifyError::HashMismatch {
            checkpoint_size: 5,
            ..
        }
    ));
}

/// 行の切り詰め（削除）は行数の照合で見つかる。
#[test]
fn truncation_is_detected() {
    let (signed, vkey) = signed_at(5);
    let rows = rows_of(5);
    let err = checkpoint::verify_checkpoint(&rows[..3], &signed, &[vkey], ORIGIN).unwrap_err();
    assert_eq!(
        err,
        VerifyError::Truncated {
            have: 3,
            checkpoint_size: 5
        }
    );
}

/// チェックポイントより後の追記は検証を壊さない（成長は許す）。ただし追記分の
/// 書き換えは次のチェックポイントまで担保外という境界も確かめる（定期発行が前提）。
#[test]
fn growth_after_checkpoint_still_verifies() {
    let (signed, vkey) = signed_at(5);
    // 5 行時点のチェックポイントで 7 行の鎖を照合しても通る。
    checkpoint::verify_checkpoint(&rows_of(7), &signed, std::slice::from_ref(&vkey), ORIGIN)
        .unwrap();
    // 6 行目以降を書き換えても、5 行目までしか担保していないため検証は通る。
    let mut evs = events(7);
    evs[6].result = "tampered".to_owned();
    let cp =
        checkpoint::verify_checkpoint(&chain::build_chain(evs), &signed, &[vkey], ORIGIN).unwrap();
    assert_eq!(cp.size, 5);
}

/// 内容を書き換えて保存済みハッシュをそのまま残せば、鎖の歩査で見つかる。
#[test]
fn content_change_without_rechaining_breaks_walk() {
    let (signed, vkey) = signed_at(5);
    let mut rows = rows_of(5);
    rows[2].event.result = "tampered".to_owned();
    let err = checkpoint::verify_checkpoint(&rows, &signed, &[vkey], ORIGIN).unwrap_err();
    assert_eq!(
        err,
        VerifyError::Chain(chain::ChainError::HashMismatch { row: 2 })
    );
}

/// 行の削除で `prev_hash` の付き合わせが壊れる場合も歩査で見つかる。
#[test]
fn middle_row_removal_breaks_walk() {
    let (signed, vkey) = signed_at(5);
    let mut rows = rows_of(5);
    rows.remove(2);
    let err = checkpoint::verify_checkpoint(&rows, &signed, &[vkey], ORIGIN).unwrap_err();
    assert_eq!(
        err,
        VerifyError::Chain(chain::ChainError::BrokenLink { row: 2 })
    );
}

/// 本文の改変（署名をそのまま流用）と、攻撃者の鍵による偽造を拒否する。
#[test]
fn forged_or_tampered_notes_are_rejected() {
    let (signed, vkey) = signed_at(5);
    let rows = rows_of(5);
    // 本文の行数を書き換えて署名をそのまま使う → 署名検証に失敗する。
    let tampered = signed.replacen("\n5\n", "\n4\n", 1);
    assert_ne!(&tampered, &signed);
    let err = checkpoint::verify_checkpoint(&rows, &tampered, std::slice::from_ref(&vkey), ORIGIN)
        .unwrap_err();
    assert_eq!(err, VerifyError::Note(note::NoteError::InvalidSignature));
    // 攻撃者の鍵（鍵名が同じでも鍵 ID が違う）で署いても既知の鍵に一致しない。
    let attacker = NoteSigner::from_bytes(ORIGIN, [0x99; 32]);
    let text = note::verify_note(&signed, std::slice::from_ref(&vkey)).unwrap();
    let forged = attacker.sign_note_text(&text);
    let err = checkpoint::verify_checkpoint(&rows, &forged, &[vkey], ORIGIN).unwrap_err();
    assert_eq!(err, VerifyError::Note(note::NoteError::NoKnownKey));
}

/// 別 workspace 向けのチェックポイントは origin の照合で拒否する。
#[test]
fn other_origin_is_rejected() {
    let rows = rows_of(5);
    let other_signer = NoteSigner::from_bytes("pelway.example/w/ws-2", [0x42; 32]);
    let other = checkpoint::sign_checkpoint(&rows, "pelway.example/w/ws-2", &other_signer).unwrap();
    let err = checkpoint::verify_checkpoint(&rows, &other, &[other_signer.verifying_key()], ORIGIN)
        .unwrap_err();
    assert_eq!(err, VerifyError::OriginMismatch);
}

/// 保管先（DB とは別の場所）に追記したチェックポイントで、DB を全て書き換えた
/// 改ざんが見つかる。書き換え前は同じ保管先のチェックポイントで検証が通ることも
/// 確かめる。保管先の長さ前置きが切れていたら、黙って空にせずエラーにする。
#[test]
fn stored_checkpoint_detects_full_db_rewrite() {
    let (log, path) = tmp_log("rewrite");
    let (signed, vkey) = signed_at(5);
    log.append(&signed).unwrap();
    let entries = log.entries().unwrap();
    assert_eq!(entries.len(), 1);
    // 保管したチェックポイントで、そのままの DB は検証が通る。
    checkpoint::verify_checkpoint(
        &rows_of(5),
        &entries[0],
        std::slice::from_ref(&vkey),
        ORIGIN,
    )
    .unwrap();
    // DB を全て書き換えて鎖も再計算すると、保管済みチェックポイントとの照合で見つかる。
    let mut evs = events(5);
    evs[2].actor = "attacker".to_owned();
    let err = checkpoint::verify_checkpoint(&chain::build_chain(evs), &entries[0], &[vkey], ORIGIN)
        .unwrap_err();
    assert!(matches!(err, VerifyError::HashMismatch { .. }));
    // 長さ前置きが切れた保管先は読み取りエラー（検証を黙って通さない）。
    std::fs::write(&path, [0x00, 0x00, 0x00]).unwrap();
    assert!(log.entries().is_err());
}
