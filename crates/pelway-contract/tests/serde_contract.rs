//! ワイヤ形式（JSON）の契約テスト。`serde` 機能が必要。
#![cfg(feature = "serde")]

use pelway_contract::approval::{
    ApproveError, ApproveRequest, COMMENT_MAX_CHARS, CommentError, CommentSaved, comment_len,
    validate_comment,
};
use pelway_contract::member::{ChannelMember, MemberKind};
use pelway_contract::task::TaskState;

#[test]
fn member_kind_is_required() {
    let missing = r#"{"member_id":"m1","display_name":"合成ユーザー"}"#;
    assert!(serde_json::from_str::<ChannelMember>(missing).is_err());

    let ok = r#"{"member_id":"m1","display_name":"合成Bot","member_kind":"ai"}"#;
    let member: ChannelMember = serde_json::from_str(ok).unwrap();
    assert_eq!(member.member_kind, MemberKind::Ai);
}

#[test]
fn unknown_member_kind_is_rejected() {
    for bad in ["robot", "bot", "external_agent", "Human"] {
        let json = format!(r#"{{"member_id":"m1","display_name":"x","member_kind":"{bad}"}}"#);
        assert!(
            serde_json::from_str::<ChannelMember>(&json).is_err(),
            "{bad}"
        );
    }
}

#[test]
fn approve_request_requires_all_four_fields() {
    let full = r#"{"card_id":"c1","card_version":1,"snapshot_hash":"sha256:s","idempotency_key":"3f1c2a9e-7b4d-4e21"}"#;
    assert!(serde_json::from_str::<ApproveRequest>(full).is_ok());
    for missing in [
        r#"{"card_version":1,"snapshot_hash":"sha256:s","idempotency_key":"3f1c2a9e-7b4d-4e21"}"#,
        r#"{"card_id":"c1","snapshot_hash":"sha256:s","idempotency_key":"3f1c2a9e-7b4d-4e21"}"#,
        r#"{"card_id":"c1","card_version":1,"idempotency_key":"3f1c2a9e-7b4d-4e21"}"#,
        r#"{"card_id":"c1","card_version":1,"snapshot_hash":"sha256:s"}"#,
    ] {
        assert!(
            serde_json::from_str::<ApproveRequest>(missing).is_err(),
            "{missing}"
        );
    }
}

#[test]
fn approve_request_rejects_unknown_fields() {
    let extra = r#"{"card_id":"c1","card_version":1,"snapshot_hash":"sha256:s","idempotency_key":"3f1c2a9e-7b4d-4e21","via":"enter"}"#;
    assert!(serde_json::from_str::<ApproveRequest>(extra).is_err());
}

#[test]
fn approve_request_rejects_malformed_idempotency_key() {
    let bad =
        r#"{"card_id":"c1","card_version":1,"snapshot_hash":"sha256:s","idempotency_key":"short"}"#;
    assert!(serde_json::from_str::<ApproveRequest>(bad).is_err());
}

#[test]
fn comment_saved_carries_snapshot_hash_and_version() {
    let saved = CommentSaved {
        card_id: "c1".into(),
        comment_id: "cmt-00000001".into(),
        card_version: 1,
        snapshot_hash: "sha256:abc".to_owned(),
        state: TaskState::AwaitingSendApproval,
    };
    let json = serde_json::to_string(&saved).unwrap();
    assert_eq!(
        json,
        r#"{"card_id":"c1","comment_id":"cmt-00000001","card_version":1,"snapshot_hash":"sha256:abc","state":"awaiting_send_approval"}"#
    );
    let missing_hash = r#"{"card_id":"c1","comment_id":"cmt-00000001","card_version":1,"state":"awaiting_send_approval"}"#;
    assert!(serde_json::from_str::<CommentSaved>(missing_hash).is_err());
}

#[test]
fn task_state_wire_names_match_spec() {
    assert_eq!(
        serde_json::to_string(&TaskState::SentAwaitingReply).unwrap(),
        r#""sent_awaiting_reply""#
    );
    assert_eq!(
        serde_json::to_string(&TaskState::AwaitingSendApproval).unwrap(),
        r#""awaiting_send_approval""#
    );
}

#[test]
fn approve_error_is_tagged_by_code() {
    let json =
        serde_json::to_string(&ApproveError::VersionConflict { current_version: 3 }).unwrap();
    assert_eq!(json, r#"{"code":"version_conflict","current_version":3}"#);
    assert_eq!(
        serde_json::to_string(&ApproveError::ApprovalInProgress).unwrap(),
        r#"{"code":"approval_in_progress"}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::IdempotencyKeyReused).unwrap(),
        r#"{"code":"idempotency_key_reused"}"#
    );
}

#[test]
fn comment_error_not_awaiting_approval_is_tagged_with_state() {
    let json = serde_json::to_string(&CommentError::NotAwaitingApproval {
        state: TaskState::Sending,
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"code":"not_awaiting_approval","state":"sending"}"#
    );
    let round: CommentError = serde_json::from_str(&json).unwrap();
    assert_eq!(
        round,
        CommentError::NotAwaitingApproval {
            state: TaskState::Sending
        }
    );
}

#[test]
fn comment_limit_is_importable_and_counts_code_points() {
    // クライアントが取り込む公開の定数と関数。
    assert_eq!(COMMENT_MAX_CHARS, 4000);
    assert_eq!(comment_len("\u{1F600}あe\u{0301}"), 4);
    assert_eq!(
        validate_comment(&"あ".repeat(COMMENT_MAX_CHARS + 1)),
        Err(CommentError::TooLong { max_chars: 4000 })
    );
    let json = serde_json::to_string(&CommentError::TooLong { max_chars: 4000 }).unwrap();
    assert_eq!(json, r#"{"code":"too_long","max_chars":4000}"#);
}
