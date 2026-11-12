//! 判断待ちの一覧（#88）。チャンネルをまたいで、待っている項目を 1 か所に集める。
//!
//! 競合の「Needs you」（CircleChat・Context）、「To handle」（Rerun）に相当する
//! （`docs/competitors/` を参照）。PRD の MVP 7-4「判断待ちを『今日』画面にまとめる。
//! 誰の判断か、何を決めるかを表示する」のうち、一覧を作る部分。
//!
//! # 守ること（このモジュールの不変条件）
//!
//! - **データを別に保存しない**。一覧は呼び出しのたびに状態のテーブル
//!   （デモでは [`crate::ledger::ApprovalLedger`] の承認カード）を検索して
//!   組み立てる。台帳の状態が変われば、次の [`build_inbox`] の結果も変わる
//!   （承認済みの項目は「自分の判断待ち」から消え、「自分の依頼」の進み具合に
//!   反映される）。
//! - 2 つのタブ（[`Inbox::needs_my_decision`]・[`Inbox::my_requests`]）の内容は
//!   #88 の定義による:
//!   - 「自分の判断待ち」: 自分が承認する項目、自分が担当する引き継ぎ、
//!     結果が分からない（`outcome_unknown`）項目。
//!   - 「自分の依頼」: 自分が依頼した項目と、その現在の状態（進み具合）。
//! - 並びは **期限の近いものから**。期限の無い項目は最後。同期限・期限なしの
//!   間は `card_id` の辞書順で決定的に並べる。
//! - 一覧の項目はカードへのポインタ（ID・宛先・状態・期限・理由）で、本文は
//!   含めない。内容は元のチャンネルのカードで確認する。
//!
//! # まだ決まらないこと（このモジュールでは扱わない）
//!
//! - 承認期限が切れたときの状態と通知は別作業。ここは一覧の判定と並べ替えだけを
//!   行う。承認 API での承認者の身元検査は #86（`ApprovalLedger::approve`）。

use crate::approval::ApprovalCard;
use crate::ids::{CardId, ChannelId, MemberId};
use crate::store::TsMs;
use crate::task::{ActionKind, TaskState};

/// 一覧の項目がそのタブに入っている理由（タブの中での分類）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum InboxReason {
    /// 自分が承認する（承認者・判断者が自分で、判断を待っている）。
    Approval,
    /// 自分が担当する引き継ぎ（`action_kind` が `AgentHandoff` で、受け取る担当が自分）。
    Handoff,
    /// 結果が分からない（`outcome_unknown`）。人が照合して判断する必要がある。
    OutcomeUnknown,
    /// 自分が依頼したもの（進み具合として現在の状態を表示する）。
    Requested,
}

/// 判断待ちの一覧の 1 項目。カードのコピーではなく、カードを指す表示用の値。
/// 本文は含めない（内容は元のチャンネルのカードで確認する）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InboxItem {
    pub card_id: CardId,
    pub channel_id: ChannelId,
    pub action_kind: ActionKind,
    /// 表示用の宛先（合成データのみ）。
    pub recipient: String,
    /// カードの現在の状態（「自分の依頼」タブでは進み具合として表示する）。
    pub state: TaskState,
    /// 依頼した人。
    pub requester: MemberId,
    /// 誰の判断待ちか（承認者。作成時に決めた1人。引き継ぎでは受け取る担当）。
    pub decider: MemberId,
    /// 判断の期限（なければ `None`）。
    pub deadline: Option<TsMs>,
    /// このタブに入っている理由。
    pub reason: InboxReason,
}

impl InboxItem {
    /// 承認カードから一覧項目を作る（理由は呼び出し側が決める）。
    fn from_card(card: &ApprovalCard, reason: InboxReason) -> Self {
        Self {
            card_id: card.card_id.clone(),
            channel_id: card.channel_id.clone(),
            action_kind: card.payload.action_kind,
            recipient: card.payload.recipient.clone(),
            state: card.state,
            requester: card.requester.clone(),
            decider: card.decider.clone(),
            deadline: card.deadline,
            reason,
        }
    }
}

/// メンバー 1 人ぶんの判断待ちの一覧（2 つのタブ）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Inbox {
    /// 「自分の判断待ち」タブ。自分が承認する項目・自分が担当する引き継ぎ・
    /// 結果が分からない項目を、期限の近い順に並べる。
    pub needs_my_decision: Vec<InboxItem>,
    /// 「自分の依頼」タブ。自分が依頼した項目を、期限の近い順に並べる。
    /// 項目の `state` が進み具合。
    pub my_requests: Vec<InboxItem>,
}

/// 判断を待っている（人の判断で動ける）状態か。
const fn is_awaiting_person(state: TaskState) -> bool {
    matches!(
        state,
        TaskState::AwaitingDecision | TaskState::AwaitingSendApproval
    )
}

/// カードの一覧から、メンバー 1 人ぶんの判断待ち一覧を作る。
///
/// `cards` は呼び出し側が範囲（workspace 等）を絞ったものを渡す
/// （[`crate::ledger::ApprovalLedger::inbox`] が workspace で絞って渡す）。
/// 一覧用のデータは保存せず、渡されたカードからその場で組み立てる。
/// カードの承認者は作成時に決まった1人（ADR 0002・#86）なので、判定は比較だけ。
#[must_use]
pub fn build_inbox<'a, I>(member: &MemberId, cards: I) -> Inbox
where
    I: IntoIterator<Item = &'a ApprovalCard>,
{
    let mut inbox = Inbox::default();
    for card in cards {
        if card.decider == *member && is_awaiting_person(card.state) {
            let reason = if card.payload.action_kind == ActionKind::AgentHandoff {
                InboxReason::Handoff
            } else {
                InboxReason::Approval
            };
            inbox
                .needs_my_decision
                .push(InboxItem::from_card(card, reason));
        } else if card.state == TaskState::OutcomeUnknown
            && (card.requester == *member || card.decider == *member)
        {
            inbox
                .needs_my_decision
                .push(InboxItem::from_card(card, InboxReason::OutcomeUnknown));
        }
        if card.requester == *member {
            inbox
                .my_requests
                .push(InboxItem::from_card(card, InboxReason::Requested));
        }
    }
    sort_by_deadline(&mut inbox.needs_my_decision);
    sort_by_deadline(&mut inbox.my_requests);
    inbox
}

/// 期限の近い順に並べ替える。期限の無い項目は最後。
/// 同じ期限（または両方期限なし）の間は `card_id` の辞書順（決定的な並び）。
fn sort_by_deadline(items: &mut [InboxItem]) {
    items.sort_by(|a, b| match (a.deadline, b.deadline) {
        (Some(x), Some(y)) => x.cmp(&y).then_with(|| a.card_id.cmp(&b.card_id)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.card_id.cmp(&b.card_id),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::ActionPayload;

    const ME: &str = "m-human-inbox";
    const OTHER: &str = "m-human-other";

    /// テスト用のカードを組み立てる（台帳を介さず、状態を自由に作る）。
    fn card(id: &str, state: TaskState) -> ApprovalCard {
        ApprovalCard {
            workspace_id: "ws-synthetic-inbox".into(),
            channel_id: "ch-synthetic-inbox".into(),
            card_id: id.into(),
            card_version: 1,
            payload: ActionPayload {
                action_kind: ActionKind::CustomerSend,
                recipient: "customer@example.invalid".to_owned(),
                body: "合成データの本文".to_owned(),
                attachments: Vec::new(),
                account: "acct-cw-1".to_owned(),
                scope: Vec::new(),
            },
            snapshot_hash: "sha256:synthetic-snapshot-inbox".to_owned(),
            state,
            requester: ME.into(),
            decider: ME.into(),
            deadline: None,
            comments: Vec::new(),
            simulated_send_id: None,
        }
    }

    fn ids(items: &[InboxItem]) -> Vec<&str> {
        items.iter().map(|i| i.card_id.as_str()).collect()
    }

    // 受け入れ条件1: 「自分の判断待ち」タブは、自分が承認する項目・
    // 自分が担当する引き継ぎ・結果不明の項目を含む（それ以外は含まない）。
    #[test]
    fn needs_my_decision_lists_what_awaits_me_and_nothing_else() {
        let mine = card("card-approve-me", TaskState::AwaitingSendApproval);
        let mut handoff_card = card("card-handoff-me", TaskState::AwaitingDecision);
        handoff_card.payload.action_kind = ActionKind::AgentHandoff;
        let handoff = handoff_card;
        let unknown = card("card-outcome-unknown", TaskState::OutcomeUnknown);
        let not_mine = ApprovalCard {
            decider: OTHER.into(),
            ..card("card-other-decider", TaskState::AwaitingSendApproval)
        };
        let already_sent = card("card-sent", TaskState::SentAwaitingReply);
        let unknown_of_other = ApprovalCard {
            requester: OTHER.into(),
            decider: OTHER.into(),
            ..card("card-unknown-other", TaskState::OutcomeUnknown)
        };
        let inbox = build_inbox(
            &ME.into(),
            [
                &mine,
                &handoff,
                &unknown,
                &not_mine,
                &already_sent,
                &unknown_of_other,
            ],
        );
        assert_eq!(
            ids(&inbox.needs_my_decision),
            ["card-approve-me", "card-handoff-me", "card-outcome-unknown"]
        );
        assert_eq!(inbox.needs_my_decision[0].reason, InboxReason::Approval);
        assert_eq!(inbox.needs_my_decision[1].reason, InboxReason::Handoff);
        assert_eq!(
            inbox.needs_my_decision[2].reason,
            InboxReason::OutcomeUnknown
        );
    }

    // 受け入れ条件1（補足）: 結果不明は依頼者にも表示する。
    #[test]
    fn outcome_unknown_appears_for_requester_too() {
        let requested_by_me = ApprovalCard {
            decider: OTHER.into(),
            ..card("card-unknown-mine", TaskState::OutcomeUnknown)
        };
        let inbox = build_inbox(&ME.into(), [&requested_by_me]);
        assert_eq!(ids(&inbox.needs_my_decision), ["card-unknown-mine"]);
        assert_eq!(
            inbox.needs_my_decision[0].reason,
            InboxReason::OutcomeUnknown
        );
    }

    // 受け入れ条件2: 「自分の依頼」タブは自分が依頼した項目とその状態（進み具合）を返す。
    #[test]
    fn my_requests_lists_my_requests_with_current_state() {
        let working = card("card-working", TaskState::Working);
        let awaiting = card("card-awaiting", TaskState::AwaitingSendApproval);
        let sent = card("card-sent", TaskState::SentAwaitingReply);
        let done = card("card-done", TaskState::Done);
        let others = ApprovalCard {
            requester: OTHER.into(),
            ..card("card-not-mine", TaskState::Working)
        };
        let inbox = build_inbox(&ME.into(), [&working, &awaiting, &sent, &done, &others]);
        assert_eq!(
            ids(&inbox.my_requests),
            ["card-awaiting", "card-done", "card-sent", "card-working"]
        );
        assert!(
            inbox
                .my_requests
                .iter()
                .all(|i| i.reason == InboxReason::Requested)
        );
        let states: Vec<TaskState> = inbox.my_requests.iter().map(|i| i.state).collect();
        assert_eq!(
            states,
            [
                TaskState::AwaitingSendApproval,
                TaskState::Done,
                TaskState::SentAwaitingReply,
                TaskState::Working
            ]
        );
    }

    // 受け入れ条件3: 期限の近いものから順に並べる。期限なしは最後。
    #[test]
    fn items_are_sorted_by_nearest_deadline_with_none_last() {
        let far = ApprovalCard {
            deadline: Some(30_000),
            ..card("card-far", TaskState::AwaitingSendApproval)
        };
        let near = ApprovalCard {
            deadline: Some(10_000),
            ..card("card-near", TaskState::AwaitingSendApproval)
        };
        let none_a = card("card-none-a", TaskState::AwaitingSendApproval);
        let none_z = card("card-none-z", TaskState::AwaitingSendApproval);
        let tie = ApprovalCard {
            deadline: Some(10_000),
            ..card("card-tie", TaskState::AwaitingSendApproval)
        };
        let inbox = build_inbox(&ME.into(), [&far, &none_z, &tie, &none_a, &near]);
        assert_eq!(
            ids(&inbox.needs_my_decision),
            [
                "card-near",
                "card-tie",
                "card-far",
                "card-none-a",
                "card-none-z"
            ]
        );
    }

    // 受け入れ条件4: 一覧は台帳の状態を検索して作る（別保存しない）。
    // 承認でカードの状態が変われば、次の一覧も変わる。
    #[test]
    fn inbox_follows_ledger_state_changes() {
        use crate::ids::IdempotencyKey;
        use crate::ledger::ApprovalLedger;

        let mut ledger = ApprovalLedger::new();
        ledger
            .create_card(crate::ledger::NewCard {
                workspace_id: "ws-synthetic-inbox".into(),
                channel_id: "ch-synthetic-inbox".into(),
                card_id: "card-live".into(),
                payload: card("card-live", TaskState::AwaitingSendApproval).payload,
                requester: ME.into(),
                decider: crate::approval::Decider::Member(ME.into()),
                deadline: Some(10_000),
            })
            .expect("new card");

        let before = ledger.inbox(&"ws-synthetic-inbox".into(), &ME.into());
        assert_eq!(ids(&before.needs_my_decision), ["card-live"]);
        assert_eq!(before.my_requests[0].state, TaskState::AwaitingSendApproval);

        // 承認すると「自分の判断待ち」から消え、「自分の依頼」の進み具合が変わる。
        let c = ledger.card(&"card-live".into()).unwrap();
        ledger
            .approve(crate::approval::ApproveRequest {
                card_id: "card-live".into(),
                card_version: c.card_version,
                snapshot_hash: c.snapshot_hash.clone(),
                idempotency_key: IdempotencyKey::new("test-key-inbox-000001").expect("valid key"),
            })
            .expect("approve");

        let after = ledger.inbox(&"ws-synthetic-inbox".into(), &ME.into());
        assert!(
            after.needs_my_decision.is_empty(),
            "承認済みは判断待ちに残らない"
        );
        assert_eq!(ids(&after.my_requests), ["card-live"]);
        assert_eq!(after.my_requests[0].state, TaskState::SentAwaitingReply);
    }

    // 受け入れ条件4（補足）: 台帳の一覧は workspace で絞る。
    #[test]
    fn ledger_inbox_is_scoped_to_the_workspace() {
        use crate::ledger::ApprovalLedger;

        let mut ledger = ApprovalLedger::new();
        for (ws, id) in [("ws-a", "card-a"), ("ws-b", "card-b")] {
            ledger
                .create_card(crate::ledger::NewCard {
                    workspace_id: ws.into(),
                    channel_id: "ch-synthetic-inbox".into(),
                    card_id: id.into(),
                    payload: card(id, TaskState::AwaitingSendApproval).payload,
                    requester: ME.into(),
                    decider: crate::approval::Decider::Member(ME.into()),
                    deadline: None,
                })
                .expect("new card");
        }
        let inbox = ledger.inbox(&"ws-a".into(), &ME.into());
        assert_eq!(ids(&inbox.needs_my_decision), ["card-a"]);
        assert_eq!(ids(&inbox.my_requests), ["card-a"]);
    }
}
