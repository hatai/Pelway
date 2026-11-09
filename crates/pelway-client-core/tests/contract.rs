//! 契約写し（`src/contract.rs`）の公開 API のテスト。
//! 期待値は原本 4564e88 `pelway-contract` のテストと同じ値（コードポイント数・エラーコード）。

use pelway_client_core::contract::{
    ApproveError, COMMENT_MAX_CHARS, CardId, CommentError, IdempotencyKey, TaskState, comment_len,
    validate_comment,
};

// --- comment_len: コードポイント数で数える ---

#[test]
fn comment_len_counts_code_points_not_bytes_nor_graphemes() {
    assert_eq!(comment_len(""), 0);
    assert_eq!(comment_len("abc"), 3);
    // CJK は 1 文字 1 コードポイント（UTF-8 では 3 バイト）。
    assert_eq!(comment_len("合成データ"), 5);
    // 絵文字 1 つ（U+1F600、UTF-8 では 4 バイト）。
    assert_eq!(comment_len("\u{1F600}"), 1);
    // e + 結合用アキュート（見た目は 1 文字、コードポイントは 2）。
    assert_eq!(comment_len("e\u{0301}"), 2);
    // 改行も 1。
    assert_eq!(comment_len("1行目\n2行目"), 7);
}

#[test]
fn comment_len_counts_flag_and_family_emoji_by_code_points() {
    // 国旗（地域指示子 2 つ）は 2。
    assert_eq!(comment_len("\u{1F1EF}\u{1F1F5}"), 2);
    // ZWJ でつないだ家族の絵文字: 男・ZWJ・女・ZWJ・女の子 = 5。
    assert_eq!(
        comment_len("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"),
        5
    );
}

// --- validate_comment: 4000 と 4001 の境目（国旗の絵文字で） ---

#[test]
fn validate_comment_accepts_exactly_4000_code_points_of_flags() {
    // 国旗 2000 個 = 4000 コードポイント（地域指示子は各 4 バイトなので 16000 バイト）。
    let flags = "\u{1F1EF}\u{1F1F5}".repeat(COMMENT_MAX_CHARS / 2);
    assert_eq!(comment_len(&flags), COMMENT_MAX_CHARS);
    assert_eq!(flags.len(), COMMENT_MAX_CHARS * 4);
    assert_eq!(validate_comment(&flags), Ok(()));
}

#[test]
fn validate_comment_rejects_4001_code_points_of_flags() {
    // 国旗 2000 個 + 結合用アクセント 1 つ = 4001 コードポイント。
    let over = format!(
        "{}\u{0301}",
        "\u{1F1EF}\u{1F1F5}".repeat(COMMENT_MAX_CHARS / 2)
    );
    assert_eq!(comment_len(&over), COMMENT_MAX_CHARS + 1);
    assert_eq!(
        validate_comment(&over),
        Err(CommentError::TooLong {
            max_chars: COMMENT_MAX_CHARS
        })
    );
}

#[test]
fn validate_comment_boundary_with_4_byte_emoji() {
    // 絵文字（4 バイト / 1 cp）4000 個は通る。バイト数では上限の 4 倍。
    let emoji = "\u{1F600}".repeat(COMMENT_MAX_CHARS);
    assert_eq!(emoji.len(), COMMENT_MAX_CHARS * 4);
    assert_eq!(validate_comment(&emoji), Ok(()));
    let over = format!("{emoji}\u{1F600}");
    assert_eq!(
        validate_comment(&over),
        Err(CommentError::TooLong {
            max_chars: COMMENT_MAX_CHARS
        })
    );
}

#[test]
fn validate_comment_rejects_blank_body() {
    for blank in ["", " ", "\n", " \n\t ", "\u{3000}"] {
        assert_eq!(
            validate_comment(blank),
            Err(CommentError::EmptyBody),
            "{blank:?}"
        );
    }
    assert_eq!(validate_comment("\n了解\n"), Ok(()));
}

// --- 承認 API のエラーコード（原本の serde タグ snake_case と同じ値） ---

#[test]
fn approve_error_codes_match_wire_tags() {
    assert_eq!(
        ApproveError::VersionConflict { current_version: 3 }.code(),
        "version_conflict"
    );
    assert_eq!(ApproveError::SnapshotMismatch.code(), "snapshot_mismatch");
    assert_eq!(
        ApproveError::NotAwaitingApproval {
            state: TaskState::SentAwaitingReply
        }
        .code(),
        "not_awaiting_approval"
    );
    assert_eq!(
        ApproveError::ApprovalInProgress.code(),
        "approval_in_progress"
    );
    assert_eq!(
        ApproveError::IdempotencyKeyReused.code(),
        "idempotency_key_reused"
    );
    assert_eq!(ApproveError::CardIdMismatch.code(), "card_id_mismatch");
    assert_eq!(ApproveError::CardNotFound.code(), "card_not_found");
}

#[test]
fn approve_error_http_status_matches_contract() {
    let conflict = ApproveError::VersionConflict { current_version: 1 };
    assert_eq!(conflict.http_status(), 409);
    assert_eq!(ApproveError::SnapshotMismatch.http_status(), 409);
    assert_eq!(
        ApproveError::NotAwaitingApproval {
            state: TaskState::Done
        }
        .http_status(),
        409
    );
    assert_eq!(ApproveError::ApprovalInProgress.http_status(), 409);
    assert_eq!(ApproveError::IdempotencyKeyReused.http_status(), 422);
    assert_eq!(ApproveError::CardIdMismatch.http_status(), 422);
    assert_eq!(ApproveError::CardNotFound.http_status(), 404);
}

// --- コメント保存 API のエラー（99ee013 で追加された 409 を含む） ---

#[test]
fn comment_error_codes_match_wire_tags() {
    assert_eq!(
        CommentError::NotAwaitingApproval {
            state: TaskState::Sending
        }
        .code(),
        "not_awaiting_approval"
    );
    assert_eq!(CommentError::EmptyBody.code(), "empty_body");
    assert_eq!(CommentError::TooLong { max_chars: 4000 }.code(), "too_long");
    assert_eq!(CommentError::CardNotFound.code(), "card_not_found");
}

#[test]
fn comment_error_http_status_matches_contract() {
    assert_eq!(
        CommentError::NotAwaitingApproval {
            state: TaskState::Sending
        }
        .http_status(),
        409
    );
    assert_eq!(CommentError::CardNotFound.http_status(), 404);
    assert_eq!(CommentError::EmptyBody.http_status(), 422);
    assert_eq!(CommentError::TooLong { max_chars: 4000 }.http_status(), 422);
}

// --- ID 型（原本 ids.rs の写し） ---

#[test]
fn idempotency_key_accepts_uuid_like_value() {
    let key = IdempotencyKey::new("3f1c2a9e-7b4d-4e21-9a0f-1c2d3e4f5a6b");
    assert!(key.is_ok());
    assert_eq!(
        key.unwrap().as_str(),
        "3f1c2a9e-7b4d-4e21-9a0f-1c2d3e4f5a6b"
    );
}

#[test]
fn idempotency_key_rejects_short_long_and_odd_chars() {
    assert!(IdempotencyKey::new("short").is_err());
    assert!(IdempotencyKey::new("a".repeat(129)).is_err());
    assert!(IdempotencyKey::new("has space in the key!!").is_err());
    assert!(IdempotencyKey::new("").is_err());
}

#[test]
fn card_id_is_a_plain_string_newtype() {
    let id = CardId::from("card-1");
    assert_eq!(id.as_str(), "card-1");
    assert_eq!(id.to_string(), "card-1");
    assert_eq!(CardId::from("card-1"), id);
}

// --- TaskState（原本 task.rs の写し） ---

#[test]
fn task_state_send_side_matches_contract() {
    assert!(!TaskState::AwaitingSendApproval.is_send_side());
    assert!(TaskState::Sending.is_send_side());
    assert!(TaskState::SentAwaitingReply.is_send_side());
    assert!(TaskState::OutcomeUnknown.is_send_side());
    assert!(!TaskState::Done.is_send_side());
}
