//! AI 同士（Bot → Bot）のタスクの受け渡し（#89）。
//!
//! OpenBot の `message_bot`（`docs/competitors/openbot.md`）に相当するが、Pelway では
//! 受け渡しを **引き継ぎの 5 項目（PRD §7 A-1）を持つ記録** としてサーバーを通して
//! 行う。5 項目は「決まったこと（決定の中身と、決めた人・時刻）」「送った内容
//! （宛先・本文・添付・送信時刻）」「今の状態」「残っている作業」「誰の判断待ちか」。
//!
//! # 守ること（このモジュールの不変条件）
//!
//! - 受け渡しはこの API（[`crate::ledger::ApprovalLedger::hand_off`]）だけで行う。
//!   メンションを含むメッセージの保存はタスクの担当を移さない
//!   （[`crate::store::DataStore::add_message`] はカードの `decider` に触らない）。
//! - 受け渡しの段数（`depth`）と記録 ID は **サーバーが決める**。要求
//!   （[`HandoffRequest`]）には含めない（OpenBot と同じ: 連鎖の深さはモデルが
//!   決めない）。`deny_unknown_fields` で `depth` 等の未知の欄は拒否する。
//! - 受け渡しの段数は [`MAX_HANDOFF_DEPTH`]（3 段）まで。4 段目は受理しない。
//! - **同じ 2 者の間を行き来する受け渡し**（A→B のあとの B→A、または再度 A→B）は
//!   受理しない。行き来の検出は同じカードの受け渡し記録の中だけで行う。
//! - 段数の上限を超えたとき・行き来を見つけたとき、サーバーは (1) チャンネルへの
//!   通知（安定 ID [`HandoffRefusalReason::notification_key`]）を結果として返し、
//!   (2) タスクの担当（カードの `decider`）を依頼者（`requester`）に戻す。
//!   拒否は「受け渡しの続行を断る」という業務結果であり、HTTP の失敗（4xx）ではない。
//! - **現在の担当（`decider`）だけが受け渡せる**。依頼先が依頼元と同じなら受理しない。
//!   これらの検査に失敗した要求は 4xx で、カードも記録も変えない。
//!
//! # まだ決まらないこと（このモジュールでは扱わない）
//!
//! - PRD §9「AI から AI へ仕事を渡すときに、人の確認を挟むか」は未決。この実装は
//!   確認を挟まない。人の確認は別の作業で足す。
//! - 受け渡し先の権限・承認の複製は行わない（03 §6「Bot が Bot へ依頼する子タスクは
//!   親 task へ関連付ける。初期案は読み取りと内部下書きだけで、親の権限・承認を
//!   複製しない」）。子タスクの権限設計はこの crate の外。

use crate::ids::{CardId, ChannelId, HandoffId, MemberId};
use crate::store::TsMs;
use crate::task::TaskState;

/// 受け渡し API のパス。`{card_id}` は対象の承認カード。
pub const HANDOFFS_PATH: &str = "/v1/approvals/{card_id}/handoffs";

/// 受け渡しの段数の上限（3 段）。
pub const MAX_HANDOFF_DEPTH: u32 = 3;

/// [`HANDOFFS_PATH`] の `{card_id}` を差し替えて、要求に使う実際の経路を作る。
#[must_use]
pub fn handoffs_path(card_id: &CardId) -> String {
    HANDOFFS_PATH.replace("{card_id}", card_id.as_str())
}

/// 引き継ぎの 5 項目のうち「1. 決まったこと」の 1 件。
/// 決定の中身と、決めた人・時刻の組。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecisionEntry {
    /// 決定の中身。
    pub body: String,
    /// 決めた人。
    pub decider: MemberId,
    /// 決めた時刻。
    pub decided_at: TsMs,
}

/// 引き継ぎの 5 項目のうち「2. 送った内容」の 1 件。
/// 宛先・本文・添付・送信時刻の組。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SentEntry {
    /// 宛先（表示用。合成データのみ）。
    pub recipient: String,
    /// 本文（表示用。合成データのみ）。
    pub body: String,
    /// 添付（名前の列。中身は持たない）。
    pub attachments: Vec<String>,
    /// 送信時刻。
    pub sent_at: TsMs,
}

/// 引き継ぎの 5 項目（PRD §7 A-1）。受け渡しの記録が必ずこの 5 つを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct HandoffContext {
    /// 1. 決まったこと（決定の中身と、決めた人・時刻）。0 件でもよい
    ///    （まだ何も決まっていないタスクの受け渡しはある）。
    pub decisions: Vec<DecisionEntry>,
    /// 2. 送った内容（宛先・本文・添付・送信時刻）。まだ何も送っていなければ 0 件。
    pub sent: Vec<SentEntry>,
    /// 3. 今の状態。
    pub current_state: TaskState,
    /// 4. 残っている作業。残っていなければ空の列。
    pub remaining_work: Vec<String>,
    /// 5. 誰の判断待ちか。
    pub awaiting: MemberId,
}

/// `POST /v1/approvals/{card_id}/handoffs` の入力。
/// 段数・記録 ID・時刻はサーバー付けのため入力に無い（`deny_unknown_fields` で
/// `depth` 等の未知の欄も拒否する）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct HandoffRequest {
    pub card_id: CardId,
    /// 依頼元。カードの現在の担当（`decider`）と一致しなければ
    /// [`HandoffError::NotCurrentDecider`]。
    pub from: MemberId,
    /// 依頼先（次の担当）。
    pub to: MemberId,
    /// 引き継ぎの 5 項目。
    pub context: HandoffContext,
}

impl HandoffRequest {
    /// パスの `{card_id}` と本文の `card_id` が一致するか検査する。
    ///
    /// # Errors
    /// 一致しなければ [`HandoffError::CardIdMismatch`]。
    pub fn check_path(&self, path_card_id: &CardId) -> Result<(), HandoffError> {
        if &self.card_id == path_card_id {
            Ok(())
        } else {
            Err(HandoffError::CardIdMismatch)
        }
    }
}

/// 受け渡し 1 件の記録。段数・ID・時刻はサーバーが決める。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct HandoffRecord {
    pub handoff_id: HandoffId,
    pub card_id: CardId,
    pub from: MemberId,
    pub to: MemberId,
    /// 引き継ぎの 5 項目。
    pub context: HandoffContext,
    /// 何段目か（1 始まり）。サーバーが数える。
    pub depth: u32,
    pub created_at: TsMs,
}

/// 受け渡し API のエラー（4xx）。どれもカードも受け渡し記録も変えない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "code", rename_all = "snake_case"))]
pub enum HandoffError {
    CardNotFound,
    CardIdMismatch,
    /// 依頼元がカードの現在の担当ではない（既にほかのメンバーへ渡っている等）。
    /// 現在の担当を読み直してから受け渡す。
    NotCurrentDecider {
        current: MemberId,
    },
    /// 依頼先が依頼元と同じ。受け渡しにならない。
    SelfHandoff,
}

impl HandoffError {
    /// HTTP ステータスへの対応。
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::CardNotFound => 404,
            Self::CardIdMismatch | Self::SelfHandoff => 422,
            Self::NotCurrentDecider { .. } => 409,
        }
    }
}

/// 受け渡しの拒否理由。チャンネルへの通知の安定 ID
/// （[`HandoffRefusalReason::notification_key`]）と 1:1 に対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "code", rename_all = "snake_case"))]
pub enum HandoffRefusalReason {
    /// 受け渡しの段数が上限を超えた（4 段目以降）。
    DepthLimitExceeded { depth: u32, max: u32 },
    /// 同じ 2 者の間を行き来している（過去に同じ組の受け渡しがある）。
    Bouncing { between: [MemberId; 2] },
}

impl HandoffRefusalReason {
    /// チャンネルへの通知の安定 ID（03 §10 と同じ考え方。表示側はこの ID で翻訳する）。
    #[must_use]
    pub fn notification_key(&self) -> &'static str {
        match self {
            Self::DepthLimitExceeded { .. } => "handoff-depth-exceeded",
            Self::Bouncing { .. } => "handoff-bouncing",
        }
    }
}

/// 拒否時にチャンネルへ知らせる通知。表示用の引数（`from`・`to`・`depth`）は
/// すべて合成データの ID・数値だけを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct HandoffNotification {
    /// 知らせる先のチャンネル（カードのチャンネル）。
    pub channel_id: ChannelId,
    /// 通知の安定 ID（[`HandoffRefusalReason::notification_key`] と同じ値）。
    pub reason_key: String,
    /// 受け渡しを依頼したメンバー（依頼元）。
    pub from: MemberId,
    /// 受け渡しの依頼先。
    pub to: MemberId,
    /// この受け渡しが何段目か（`between` の行き来・上限の表示に使う）。
    pub depth: u32,
}

/// 受け渡しの拒否。サーバーが (1) チャンネルへの通知を作り、
/// (2) タスクの担当を依頼者に戻したあとの結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct HandoffRefusal {
    pub reason: HandoffRefusalReason,
    pub notification: HandoffNotification,
    /// タスクを依頼者に戻したあとの担当（カードの `requester`）。
    pub returned_to: MemberId,
}

/// [`crate::ledger::ApprovalLedger::hand_off`] の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "result", rename_all = "snake_case"))]
pub enum HandoffDecision {
    /// 受け入れた。タスクの担当を依頼先（`to`）へ移した。
    Accepted { record: HandoffRecord },
    /// 拒否した（段数の上限・同じ 2 者の行き来）。タスクを依頼者に戻した。
    Refused { refusal: HandoffRefusal },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::Decider;
    use crate::ids::{CardId, MemberId};
    use crate::ledger::{ApprovalLedger, NewCard};
    use crate::member::MemberKind;
    use crate::snapshot::ActionPayload;
    use crate::store::{DataStore, NewMessage, PurposeId};
    use crate::task::{ActionKind, TaskState};

    const NOW: TsMs = 1_000;

    fn member(s: &str) -> MemberId {
        MemberId::from(s)
    }

    /// 5 項目すべてに 1 件以上の値を入れた引き継ぎ文脈。
    fn context() -> HandoffContext {
        HandoffContext {
            decisions: vec![DecisionEntry {
                body: "案Aで送る".to_owned(),
                decider: member("m-human-1"),
                decided_at: 900,
            }],
            sent: vec![SentEntry {
                recipient: "customer@example.invalid".to_owned(),
                body: "合成データの本文".to_owned(),
                attachments: vec!["引合書.png".to_owned()],
                sent_at: 950,
            }],
            current_state: TaskState::Working,
            remaining_work: vec!["返信を確認する".to_owned()],
            awaiting: member("m-bot-2"),
        }
    }

    /// 担当 `decider` の AgentHandoff カード（依頼者は m-human-1）。
    /// payload は PR #111 からの `ActionPayload`（NewCard が持つのはこれ 1 つ）。
    fn handoff_card(id: &str, decider: &str) -> NewCard {
        NewCard {
            workspace_id: "ws-synthetic".into(),
            channel_id: "ch-synthetic".into(),
            card_id: id.into(),
            payload: ActionPayload {
                action_kind: ActionKind::AgentHandoff,
                recipient: "m-bot-2".to_owned(),
                body: "合成データの本文".to_owned(),
                attachments: vec!["sha256:att-1".to_owned()],
                account: "acct-cw-1".to_owned(),
                scope: vec!["tickets:write".to_owned()],
            },
            requester: member("m-human-1"),
            decider: Decider::Member(member(decider)),
            deadline: None,
        }
    }

    fn ledger_with_card() -> ApprovalLedger {
        let mut ledger = ApprovalLedger::new();
        ledger
            .create_card(handoff_card("card-1", "m-bot-1"))
            .expect("new card");
        ledger
    }

    fn handoff_req(from: &str, to: &str) -> HandoffRequest {
        HandoffRequest {
            card_id: CardId::from("card-1"),
            from: member(from),
            to: member(to),
            context: context(),
        }
    }

    fn accept(ledger: &mut ApprovalLedger, from: &str, to: &str) -> HandoffRecord {
        match ledger
            .hand_off(handoff_req(from, to), NOW)
            .expect("hand_off itself must not error")
        {
            HandoffDecision::Accepted { record } => record,
            HandoffDecision::Refused { refusal } => {
                panic!("expected acceptance, refused: {:?}", refusal.reason)
            }
        }
    }

    fn refuse(ledger: &mut ApprovalLedger, from: &str, to: &str) -> HandoffRefusal {
        match ledger
            .hand_off(handoff_req(from, to), NOW)
            .expect("hand_off itself must not error")
        {
            HandoffDecision::Accepted { record } => {
                panic!("expected refusal, accepted: {:?}", record)
            }
            HandoffDecision::Refused { refusal } => refusal,
        }
    }

    fn decider(ledger: &ApprovalLedger, card: &str) -> MemberId {
        ledger.card(&card.into()).unwrap().decider.clone()
    }

    // ---- 受け入れ条件 1: 受け渡しは 5 項目を持つ記録としてサーバーを通る ----

    #[test]
    fn handoff_record_carries_all_five_context_fields() {
        let mut ledger = ledger_with_card();
        let record = accept(&mut ledger, "m-bot-1", "m-bot-2");
        // 記録は 5 項目をそのまま持つ（1. 決まったこと、2. 送った内容、3. 今の状態、
        // 4. 残っている作業、5. 誰の判断待ちか）。
        assert_eq!(record.context, context());
        // 段数・ID・時刻はサーバーが決めた値（要求には含めていない）。
        assert_eq!(record.depth, 1);
        assert_eq!(record.handoff_id.as_str(), "hnd-00000001");
        assert_eq!(record.created_at, NOW);
        assert_eq!(record.card_id, CardId::from("card-1"));
        assert_eq!(record.from, member("m-bot-1"));
        assert_eq!(record.to, member("m-bot-2"));
        // カードの担当（誰の判断待ちか）が依頼先へ移る。
        assert_eq!(decider(&ledger, "card-1"), member("m-bot-2"));
        // 記録は台帳に残る（サーバーを通した記録）。
        assert_eq!(ledger.handoff_log(&"card-1".into()).len(), 1);
    }

    #[test]
    fn depth_and_ids_are_not_accepted_from_the_client() {
        // 段数・記録 ID・時刻はサーバー付け。要求の欄として受け付けない。
        let full = serde_json::to_string(&handoff_req("m-bot-1", "m-bot-2")).unwrap();
        assert!(serde_json::from_str::<HandoffRequest>(&full).is_ok());
        for extra in [
            r#","depth":1"#,
            r#","handoff_id":"hnd-00000001""#,
            r#","created_at":1000"#,
        ] {
            let tampered = full.replace('}', &format!("{extra}}}"));
            assert!(
                serde_json::from_str::<HandoffRequest>(&tampered).is_err(),
                "must reject extra field {extra}"
            );
        }
    }

    // ---- 受け入れ条件 2: メンションしただけではタスクの担当を移さない ----

    #[test]
    fn mention_only_message_does_not_move_decider() {
        let ledger = ledger_with_card();
        let mut store = DataStore::new();
        // 本文に @m-bot-2 へのメンションを含むメッセージを保存する。
        store.add_message(
            NewMessage {
                workspace_id: "ws-synthetic".into(),
                channel_id: "ch-synthetic".into(),
                author_id: member("m-human-1"),
                member_kind: MemberKind::Human,
                purpose_id: PurposeId::P01,
                body: "@m-bot-2 このタスクをお願いします".to_owned(),
                thread_root: None,
            },
            NOW,
        );
        assert_eq!(store.messages(&"ws-synthetic".into()).len(), 1);
        // カードの担当と受け渡し記録は変わらない。
        assert_eq!(decider(&ledger, "card-1"), member("m-bot-1"));
        assert!(ledger.handoff_log(&"card-1".into()).is_empty());
    }

    // ---- 受け入れ条件 3: 受け渡しの段数を 3 段までにする ----

    #[test]
    fn handoff_depth_is_capped_at_three() {
        let mut ledger = ledger_with_card();
        // A→B→C→D は同じ 2 者の組を繰り返さない（行き来ではない）ので
        // 段数の上限だけで判断される。
        assert_eq!(accept(&mut ledger, "m-bot-1", "m-bot-2").depth, 1);
        assert_eq!(accept(&mut ledger, "m-bot-2", "m-bot-3").depth, 2);
        assert_eq!(accept(&mut ledger, "m-bot-3", "m-bot-4").depth, 3);
        // 4 段目は受理しない（拒否の中身は次のテストで確かめる）。
        assert!(matches!(
            ledger.hand_off(handoff_req("m-bot-4", "m-bot-5"), NOW),
            Ok(HandoffDecision::Refused { .. })
        ));
        // 記録は 3 件のまま。
        assert_eq!(ledger.handoff_log(&"card-1".into()).len(), 3);
    }

    // ---- 受け入れ条件 4: 同じ相手の間を行き来しているのを見つけたら止める ----

    #[test]
    fn bouncing_between_the_same_pair_is_refused() {
        let mut ledger = ledger_with_card();
        accept(&mut ledger, "m-bot-1", "m-bot-2");
        // B → A は同じ 2 者の間の行き来（`between` は向きによらず同じ組）。
        // 拒否はチャンネルへの通知を返し、タスクを依頼者に戻す（上限超過と同じ）。
        let refusal = refuse(&mut ledger, "m-bot-2", "m-bot-1");
        assert_eq!(
            refusal.reason,
            HandoffRefusalReason::Bouncing {
                between: [member("m-bot-1"), member("m-bot-2")],
            }
        );
        assert_eq!(refusal.notification.reason_key, "handoff-bouncing");
        assert_eq!(refusal.notification.channel_id, "ch-synthetic".into());
        assert_eq!(refusal.returned_to, member("m-human-1"));
        assert_eq!(decider(&ledger, "card-1"), member("m-human-1"));
        // 同一方向（A → B）の再要求は、担当が既に B へ移っているため
        // NotCurrentDecider で断る（行き以前の問題として 409）。
        let mut ledger2 = ledger_with_card();
        accept(&mut ledger2, "m-bot-1", "m-bot-2");
        assert_eq!(
            ledger2.hand_off(handoff_req("m-bot-1", "m-bot-2"), NOW),
            Err(HandoffError::NotCurrentDecider {
                current: member("m-bot-2")
            })
        );
    }

    // ---- 受け入れ条件 5: 上限超過や行き来のときはチャンネルに知らせ、
    //      タスクを依頼者に戻す ----

    #[test]
    fn depth_refusal_notifies_channel_and_returns_task_to_requester() {
        let mut ledger = ledger_with_card();
        for (from, to) in [
            ("m-bot-1", "m-bot-2"),
            ("m-bot-2", "m-bot-3"),
            ("m-bot-3", "m-bot-4"),
        ] {
            accept(&mut ledger, from, to);
        }
        let refusal = refuse(&mut ledger, "m-bot-4", "m-bot-5");
        assert_eq!(
            refusal.reason,
            HandoffRefusalReason::DepthLimitExceeded {
                depth: 4,
                max: MAX_HANDOFF_DEPTH,
            }
        );
        // チャンネルへの通知を持つ（知らせる先はカードのチャンネル）。
        assert_eq!(refusal.notification.channel_id, "ch-synthetic".into());
        assert_eq!(refusal.notification.reason_key, "handoff-depth-exceeded");
        assert_eq!(refusal.notification.from, member("m-bot-4"));
        assert_eq!(refusal.notification.to, member("m-bot-5"));
        assert_eq!(refusal.notification.depth, 4);
        // タスクを依頼者（m-human-1）に戻す。
        assert_eq!(refusal.returned_to, member("m-human-1"));
        assert_eq!(decider(&ledger, "card-1"), member("m-human-1"));
    }

    // ---- 検査（4xx。状態を変えない） ----

    #[test]
    fn only_the_current_decider_can_hand_off() {
        let mut ledger = ledger_with_card();
        // 現在の担当は m-bot-1。m-bot-2 からは渡せない。
        assert_eq!(
            ledger.hand_off(handoff_req("m-bot-2", "m-bot-3"), NOW),
            Err(HandoffError::NotCurrentDecider {
                current: member("m-bot-1")
            })
        );
        assert_eq!(
            HandoffError::NotCurrentDecider {
                current: member("m-bot-1")
            }
            .http_status(),
            409
        );
        // カードも記録も変わらない。
        assert_eq!(decider(&ledger, "card-1"), member("m-bot-1"));
        assert!(ledger.handoff_log(&"card-1".into()).is_empty());
    }

    #[test]
    fn self_handoff_and_unknown_card_are_rejected() {
        let mut ledger = ledger_with_card();
        assert_eq!(
            ledger.hand_off(handoff_req("m-bot-1", "m-bot-1"), NOW),
            Err(HandoffError::SelfHandoff)
        );
        // カードが無ければ 404（依頼先の MemberId ではなく card_id で決まる）。
        let unknown_card = HandoffRequest {
            card_id: CardId::from("card-x"),
            ..handoff_req("m-bot-1", "m-bot-2")
        };
        assert_eq!(
            ledger.hand_off(unknown_card, NOW),
            Err(HandoffError::CardNotFound)
        );
        assert_eq!(HandoffError::SelfHandoff.http_status(), 422);
        assert_eq!(HandoffError::CardNotFound.http_status(), 404);
        let req = handoff_req("m-bot-1", "m-bot-2");
        assert_eq!(
            req.check_path(&"card-2".into()),
            Err(HandoffError::CardIdMismatch)
        );
        assert_eq!(req.check_path(&"card-1".into()), Ok(()));
        assert_eq!(HandoffError::CardIdMismatch.http_status(), 422);
        assert!(ledger.handoff_log(&"card-1".into()).is_empty());
    }

    // ---- 経路 ----

    #[test]
    fn path_builder_replaces_the_card_id_segment() {
        assert_eq!(
            handoffs_path(&CardId::from("card-1")),
            "/v1/approvals/card-1/handoffs"
        );
    }
}
