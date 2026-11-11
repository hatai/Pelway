//! 契約モジュール（`src/contract.rs` の再エクスポート）の公開 API のテスト。
//! 契約型は `pelway-contract` をそのまま再エクスポートしており、写しではない。
//! 期待値は原本 `pelway-contract` の契約と同じ値（コードポイント数・ワイヤのエラータグ）。

use pelway_client_core::contract::{
    ApproveError, COMMENT_MAX_CHARS, CardId, CommentError, IdempotencyKey, TaskState, comment_len,
    validate_comment,
};

// --- 再エクスポートが pelway-contract と同一型であること（写しでないこと） ---
// 写し（別型の複製）のままだとこれらの代入はコンパイルが通らない。

#[test]
fn contract_reexports_pelway_contract_types_verbatim() {
    // ids: CardId（ 他の ID 型も同じ newtype マクロ由来 ）。
    let card = pelway_client_core::contract::CardId::from("card-1");
    let original: pelway_contract::ids::CardId = card;
    assert_eq!(original.as_str(), "card-1");
    let back: pelway_client_core::contract::CardId = original;
    assert_eq!(back.as_str(), "card-1");
    // approval: CommentSaved。
    let saved = pelway_client_core::contract::CommentSaved {
        card_id: CardId::from("c1"),
        comment_id: pelway_client_core::contract::CommentId::from("cmt-1"),
        card_version: 1,
        snapshot_hash: "sha256:abc".to_owned(),
        state: TaskState::AwaitingSendApproval,
    };
    let original: pelway_contract::approval::CommentSaved = saved;
    assert_eq!(original.card_version, 1);
    // task: TaskState。
    let state = pelway_client_core::contract::TaskState::Sending;
    let original: pelway_contract::task::TaskState = state;
    assert!(original.is_send_side());
}

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

// --- 承認 API のエラーのワイヤ形式（原本 pelway-contract の serde タグと同じ値） ---
// 契約を再エクスポートしたため、タグは serde derive が作る。直列化で確かめる。
// http_status() の対応は同一型の pelway-contract 側（ledger のテスト）が検証する。

#[test]
fn approve_error_codes_match_wire_tags() {
    assert_eq!(
        serde_json::to_string(&ApproveError::VersionConflict { current_version: 3 }).unwrap(),
        r#"{"code":"version_conflict","current_version":3}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::SnapshotMismatch).unwrap(),
        r#"{"code":"snapshot_mismatch"}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::NotAwaitingApproval {
            state: TaskState::SentAwaitingReply
        })
        .unwrap(),
        r#"{"code":"not_awaiting_approval","state":"sent_awaiting_reply"}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::ApprovalInProgress).unwrap(),
        r#"{"code":"approval_in_progress"}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::IdempotencyKeyReused).unwrap(),
        r#"{"code":"idempotency_key_reused"}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::CardIdMismatch).unwrap(),
        r#"{"code":"card_id_mismatch"}"#
    );
    assert_eq!(
        serde_json::to_string(&ApproveError::CardNotFound).unwrap(),
        r#"{"code":"card_not_found"}"#
    );
}

// --- コメント保存 API のエラー ---

#[test]
fn comment_error_codes_match_wire_tags() {
    assert_eq!(
        serde_json::to_string(&CommentError::NotAwaitingApproval {
            state: TaskState::Sending
        })
        .unwrap(),
        r#"{"code":"not_awaiting_approval","state":"sending"}"#
    );
    assert_eq!(
        serde_json::to_string(&CommentError::EmptyBody).unwrap(),
        r#"{"code":"empty_body"}"#
    );
    assert_eq!(
        serde_json::to_string(&CommentError::TooLong { max_chars: 4000 }).unwrap(),
        r#"{"code":"too_long","max_chars":4000}"#
    );
    assert_eq!(
        serde_json::to_string(&CommentError::CardNotFound).unwrap(),
        r#"{"code":"card_not_found"}"#
    );
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
