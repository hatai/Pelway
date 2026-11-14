//! 管理操作の監査で追加した `reason` と `ref_id` フィールドの符号化（#149）。
//! `None` は従来の符号化（タグ 0x00）のまま（`compat.rs` の固定値が保存の証拠）、
//! `Some` は拡張符号化で、理由・参照の改ざんは歩査で検出できる。

use pelway_audit_checkpoint::chain::{self, AuditEvent, ChainError};

fn event(reason: Option<String>) -> AuditEvent {
    AuditEvent {
        workspace_id: "admin".to_owned(),
        at_ms: 1_700_000_000_000,
        actor: "ops-1".to_owned(),
        subject: "channels".to_owned(),
        reason,
        ref_id: None,
        kind: "admin-op-intent".to_owned(),
        result: "pending".to_owned(),
        snapshot_hash: [7; 32],
        policy_version: 0,
    }
}

/// 理由の有無でタグが変わり、同じ行でもハッシュが変わる（None と Some("") は
/// 別の行として区別される）。理由と result 行の参照（ref_id）の改ざんは歩査の
/// 再計算で検出できる。
#[test]
fn reason_changes_hash_and_tampering_is_detected() {
    let prev = [1; 32];
    let without = chain::row_hash(&event(None), &prev);
    let empty = chain::row_hash(&event(Some(String::new())), &prev);
    let with = chain::row_hash(&event(Some("issue #149 の確認".to_owned())), &prev);
    assert_ne!(
        without, empty,
        "None（タグ 0x00）と Some 空（タグ 0x01）は別"
    );
    assert_ne!(empty, with, "理由の中身がハッシュに入る");

    let mut rows = chain::build_chain(vec![
        event(None),
        event(Some("夜間メンテナンス".to_owned())),
        event(Some("誤登録の訂正".to_owned())),
    ]);
    rows[1].event.reason = Some("書き換え後の理由".to_owned());
    assert_eq!(
        chain::walk(&rows),
        Err(ChainError::HashMismatch { row: 1 }),
        "保存済み hash と内容から計算した hash が一致しない"
    );
    // result 行の参照（ref_id）の付け替えも同様に検出する。
    let mut rows = chain::build_chain(vec![event(None), event(Some("夜間".to_owned()))]);
    rows[1].event.ref_id = Some(1);
    assert_eq!(
        chain::walk(&rows),
        Err(ChainError::HashMismatch { row: 1 }),
        "参照の付け替え"
    );
}
