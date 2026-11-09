//! 承認カードの状態機械（`src/approval.rs`）のテスト。
//! 通信は書かず「次に何をするか」（Effect）を検証する。キー生成と時計は Fake を注入する。

use pelway_client_core::approval::{
    ApprovalMachine, BugReason, CardView, Clock, CommentAttempt, Effect, KeySource, Notice,
    ReloadReason,
};
use pelway_client_core::contract::{
    ApproveError, ApproveOutcome, ApproveRequest, CardId, CommentId, CommentSaved, IdempotencyKey,
    SimulatedSendId, TaskState,
};

// ---- テスト用のキー源と時計 ----

struct FakeKeys {
    issued: u32,
}

impl FakeKeys {
    fn new() -> Self {
        Self { issued: 0 }
    }
}

impl KeySource for FakeKeys {
    fn fresh_key(&mut self) -> IdempotencyKey {
        self.issued += 1;
        // 16 文字以上の [A-Za-z0-9_-]。
        IdempotencyKey::new(format!("test-key-{:07}", self.issued)).unwrap()
    }
}

struct FakeClock {
    now_ms: std::cell::Cell<u64>,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            now_ms: std::cell::Cell::new(0),
        }
    }

    fn advance(&self, ms: u64) {
        self.now_ms.set(self.now_ms.get() + ms);
    }
}

impl Clock for FakeClock {
    fn now_ms(&self) -> u64 {
        self.now_ms.get()
    }
}

fn card_v3() -> CardView {
    CardView {
        card_id: CardId::from("card-1"),
        card_version: 3,
        snapshot_hash: "snap-a".to_string(),
    }
}

fn machine_with(clock: &FakeClock) -> ApprovalMachine<FakeKeys, &FakeClock> {
    ApprovalMachine::new(card_v3(), FakeKeys::new(), clock)
}

fn approve_req_of(effect: Option<Effect>) -> ApproveRequest {
    match effect {
        Some(Effect::Approve(req)) => req,
        other => panic!("expected Effect::Approve, got {other:?}"),
    }
}

fn in_progress() -> Result<ApproveOutcome, ApproveError> {
    Err(ApproveError::ApprovalInProgress)
}

fn outcome() -> ApproveOutcome {
    ApproveOutcome {
        card_id: CardId::from("card-1"),
        card_version: 4,
        state: TaskState::SentAwaitingReply,
        simulated_send_id: SimulatedSendId::from("send-1"),
    }
}

// ---- 新規押下 ----

#[test]
fn pressing_approve_uses_a_fresh_key_and_the_shown_version_and_snapshot() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    assert!(m.approve_button_enabled());
    assert!(m.comment_editable());

    let req = approve_req_of(m.press_approve());
    assert_eq!(req.card_id, CardId::from("card-1"));
    assert_eq!(req.card_version, 3);
    assert_eq!(req.snapshot_hash, "snap-a");
    assert_eq!(req.idempotency_key.as_str(), "test-key-0000001");

    // 承認の通信中はボタンもコメント欄も無効。
    assert!(!m.approve_button_enabled());
    assert!(!m.comment_editable());
}

#[test]
fn pressing_approve_again_while_approving_is_ignored() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(m.press_approve(), None);
}

#[test]
fn approve_success_disables_further_approval_and_scheduling() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(m.on_approve_response(Ok(outcome())), None);
    assert!(!m.approve_button_enabled());
    assert!(!m.comment_editable());
    assert_eq!(m.press_approve(), None);
    assert_eq!(m.poll(), None);
}

// ---- approval_in_progress の確認（同じキー・1s/2s/4s・最大 3 回） ----

#[test]
fn in_progress_is_rechecked_with_the_same_key_at_1s_2s_4s_then_gives_up() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let first = approve_req_of(m.press_approve());

    // 最初の応答が approval_in_progress → 1 秒後の確認を予約。
    assert_eq!(m.on_approve_response(in_progress()), None);
    assert_eq!(m.poll(), None);
    clock.advance(999);
    assert_eq!(m.poll(), None);
    clock.advance(1);
    let check1 = approve_req_of(m.poll());
    assert_eq!(check1.idempotency_key, first.idempotency_key);
    assert_eq!(m.on_approve_response(in_progress()), None);

    // 2 回目はさらに 2 秒後。
    clock.advance(1999);
    assert_eq!(m.poll(), None);
    clock.advance(1);
    let check2 = approve_req_of(m.poll());
    assert_eq!(check2.idempotency_key, first.idempotency_key);
    assert_eq!(m.on_approve_response(in_progress()), None);

    // 3 回目はさらに 4 秒後。
    clock.advance(3999);
    assert_eq!(m.poll(), None);
    clock.advance(1);
    let check3 = approve_req_of(m.poll());
    assert_eq!(check3.idempotency_key, first.idempotency_key);

    // 3 回目の確認でも決まらない → 処理中のため確認できません。
    assert_eq!(
        m.on_approve_response(in_progress()),
        Some(Effect::Notice(Notice::CannotCheckApproval))
    );
    // 自動の確認はもうない。ボタンは押せる。コメント欄は承認未確定のため読み取り専用のまま。
    assert_eq!(m.poll(), None);
    assert!(m.approve_button_enabled());
    assert!(!m.comment_editable());
}

#[test]
fn in_progress_resolves_on_a_later_recheck_and_clears_the_schedule() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    m.on_approve_response(in_progress());
    clock.advance(1000);
    let _ = m.poll();
    assert_eq!(m.on_approve_response(Ok(outcome())), None);
    assert!(!m.approve_button_enabled());
    assert_eq!(m.poll(), None);
    assert_eq!(m.press_approve(), None);
}

#[test]
fn pressing_again_after_giving_up_rechecks_with_the_same_key_and_no_new_send() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let first = approve_req_of(m.press_approve());
    for delay_ms in [1000, 2000, 4000] {
        m.on_approve_response(in_progress());
        clock.advance(delay_ms);
        let _ = approve_req_of(m.poll());
    }
    assert_eq!(
        m.on_approve_response(in_progress()),
        Some(Effect::Notice(Notice::CannotCheckApproval))
    );

    // もう一度押す → 同じキーで確認（新しいキーを作らない＝二重送信しない）。
    let again = approve_req_of(m.press_approve());
    assert_eq!(again.idempotency_key, first.idempotency_key);
    assert_eq!(again.card_version, 3);
    assert_eq!(again.snapshot_hash, "snap-a");

    // 再押下後の確認も同じキーのまま 1 秒後に始まる。
    m.on_approve_response(in_progress());
    clock.advance(1000);
    let recheck = approve_req_of(m.poll());
    assert_eq!(recheck.idempotency_key, first.idempotency_key);
}

// ---- 409: 読み直しが必要な失敗（自動再送しない） ----

#[test]
fn version_conflict_requires_reload_without_auto_retry() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.on_approve_response(Err(ApproveError::VersionConflict { current_version: 5 })),
        Some(Effect::Notice(Notice::NeedsReload {
            reason: ReloadReason::VersionConflict { current_version: 5 }
        }))
    );
    assert!(!m.approve_button_enabled());
    assert!(!m.comment_editable());
    assert_eq!(m.press_approve(), None);
    assert_eq!(m.poll(), None);
}

#[test]
fn snapshot_mismatch_requires_reload() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.on_approve_response(Err(ApproveError::SnapshotMismatch)),
        Some(Effect::Notice(Notice::NeedsReload {
            reason: ReloadReason::SnapshotMismatch
        }))
    );
    assert_eq!(m.press_approve(), None);
    assert_eq!(m.poll(), None);
}

#[test]
fn not_awaiting_approval_requires_reload() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.on_approve_response(Err(ApproveError::NotAwaitingApproval {
            state: TaskState::Done
        })),
        Some(Effect::Notice(Notice::NeedsReload {
            reason: ReloadReason::NotAwaitingApproval {
                state: TaskState::Done
            }
        }))
    );
}

#[test]
fn card_not_found_requires_reload() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.on_approve_response(Err(ApproveError::CardNotFound)),
        Some(Effect::Notice(Notice::NeedsReload {
            reason: ReloadReason::CardNotFound
        }))
    );
}

#[test]
fn reload_restores_idle_and_the_next_approval_uses_a_new_key_and_new_snapshot() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let first = approve_req_of(m.press_approve());
    m.on_approve_response(Err(ApproveError::VersionConflict { current_version: 5 }));

    let reloaded = CardView {
        card_id: CardId::from("card-1"),
        card_version: 5,
        snapshot_hash: "snap-b".to_string(),
    };
    m.reload(reloaded);
    assert!(m.approve_button_enabled());
    assert!(m.comment_editable());

    let second = approve_req_of(m.press_approve());
    assert_eq!(second.card_version, 5);
    assert_eq!(second.snapshot_hash, "snap-b");
    assert_ne!(second.idempotency_key, first.idempotency_key);
}

// ---- 422: クライアントの不具合（再送しない） ----

#[test]
fn idempotency_key_reused_is_a_client_bug_and_is_never_retried() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.on_approve_response(Err(ApproveError::IdempotencyKeyReused)),
        Some(Effect::Notice(Notice::ClientBug {
            reason: BugReason::IdempotencyKeyReused
        }))
    );
    assert!(!m.approve_button_enabled());
    assert_eq!(m.press_approve(), None);
    assert_eq!(m.poll(), None);
    // 復帰は読み直しのみ。
    m.reload(card_v3());
    assert!(m.approve_button_enabled());
}

#[test]
fn card_id_mismatch_is_a_client_bug_and_is_never_retried() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.on_approve_response(Err(ApproveError::CardIdMismatch)),
        Some(Effect::Notice(Notice::ClientBug {
            reason: BugReason::CardIdMismatch
        }))
    );
    assert_eq!(m.press_approve(), None);
    assert_eq!(m.poll(), None);
}

// ---- コメント保存（コメント欄はコメント API だけを呼ぶ） ----

#[test]
fn blank_comment_is_rejected_without_calling_the_api() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    for blank in ["", " ", "\n", "\u{3000}"] {
        assert_eq!(
            m.request_save_comment(blank.to_string()),
            CommentAttempt::Reject(Notice::CommentBlank),
            "{blank:?}"
        );
    }
}

#[test]
fn comment_of_exactly_4000_code_points_is_sent_and_4001_is_rejected() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);

    // 国旗 2000 個 = 4000 コードポイントは送る。
    let flags = "\u{1F1EF}\u{1F1F5}".repeat(2000);
    assert!(matches!(
        m.request_save_comment(flags),
        CommentAttempt::Send(_)
    ));

    // 国旗 2000 個 + 結合アクセント = 4001 コードポイントは拒否。
    let over = format!("{}\u{0301}", "\u{1F1EF}\u{1F1F5}".repeat(2000));
    assert_eq!(
        m.request_save_comment(over),
        CommentAttempt::Reject(Notice::CommentTooLong { max_chars: 4000 })
    );
}

#[test]
fn saving_a_comment_swaps_snapshot_hash_and_keeps_the_shown_version() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);

    match m.request_save_comment("了解".to_string()) {
        CommentAttempt::Send(req) => {
            assert_eq!(req.card_id, CardId::from("card-1"));
            assert_eq!(req.body, "了解");
        }
        other => panic!("expected Send, got {other:?}"),
    }

    m.on_comment_saved(CommentSaved {
        card_id: CardId::from("card-1"),
        comment_id: CommentId::from("cmt-1"),
        card_version: 3,
        snapshot_hash: "snap-b".to_string(),
        state: TaskState::AwaitingSendApproval,
    });
    assert_eq!(m.card().snapshot_hash, "snap-b");
    assert_eq!(m.card().card_version, 3);

    // 次の承認は差し替え後の snapshot と表示中の版で行う。
    let req = approve_req_of(m.press_approve());
    assert_eq!(req.snapshot_hash, "snap-b");
    assert_eq!(req.card_version, 3);
}

#[test]
fn comment_cannot_be_saved_while_approving() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    assert_eq!(
        m.request_save_comment("了解".to_string()),
        CommentAttempt::Reject(Notice::CommentLocked)
    );
}

#[test]
fn late_approve_response_arriving_out_of_approval_is_ignored() {
    let clock = FakeClock::new();
    let mut m = machine_with(&clock);
    let _ = m.press_approve();
    m.on_approve_response(Ok(outcome()));
    // 承認済みのあとに届いた古い応答は無視する。
    assert_eq!(m.on_approve_response(in_progress()), None);
    assert!(!m.approve_button_enabled());
}
