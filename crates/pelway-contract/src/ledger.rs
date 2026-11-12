//! 承認カードのメモリ内参照実装（デモ用、データはメモリだけ）。
//!
//! HTTP 層はこの型をそのまま呼ぶ想定。外部 I/O は一切しない。疑似送信は
//! メモリ上の記録 [`SimulatedSendRecord`] を 1 件追加するだけ。
//!
//! # 承認の流れと冪等キー
//!
//! 1. [`ApprovalLedger::begin_approve`]：冪等キーの既存記録を見て、無ければカードを検査する。
//!    検査に通ったら、送信前にキーを「処理中」として (`card_id`, `card_version`,
//!    `snapshot_hash`) に結び付けて記録し、カードを「送信中」にする。
//!    検査で失敗したら何も記録しない（キーは解放されたまま）。
//! 2. [`ApprovalLedger::perform_simulated_send`]：疑似送信を 1 回行い、キーを「完了」にして
//!    結果を保存する。同じキー・同じ内容の再送には、この結果をそのまま返す。
//! 3. [`ApprovalLedger::abort_before_send`]：何も送らないまま中止するとき（送信前の失敗）だけ、
//!    キーを解放してカードを「送信承認待ち」に戻す。
//!
//! [`ApprovalLedger::approve`] は 1→2 をまとめて行う。送信が成功して応答だけが失われた場合も、
//! キーは「完了」で残るので、同じキーの再送は 2 回目の送信をしない。

use std::collections::HashMap;

use crate::approval::{
    ApprovalCard, ApprovalPolicy, ApproveError, ApproveOutcome, ApproveRequest, CardComment,
    CommentError, CommentSaved, CreateCardError, Decider, SaveCommentRequest, validate_comment,
};
use crate::handoff::{
    HandoffDecision, HandoffError, HandoffNotification, HandoffRecord, HandoffRefusal,
    HandoffRefusalReason, HandoffRequest, MAX_HANDOFF_DEPTH,
};
use crate::ids::{
    CardId, ChannelId, CommentId, HandoffId, IdempotencyKey, MemberId, SimulatedSendId, WorkspaceId,
};
use crate::member::MemberKind;
use crate::snapshot::{ActionPayload, compute_snapshot_hash};
use crate::store::TsMs;
use crate::task::TaskState;

/// 新しい承認カードの入力（合成データのみ）。
#[derive(Debug, Clone)]
pub struct NewCard {
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    pub card_id: CardId,
    /// 正規化した操作の中身。`snapshot_hash` はこれだけから計算する。
    pub payload: ActionPayload,
    /// 依頼した人（「自分の依頼」タブの判定）。
    pub requester: MemberId,
    /// 誰の判断待ちか（承認者。指名した1人または役割。`AgentHandoff` では引き継ぎ先の担当）。
    pub decider: Decider,
    /// 判断の期限（並べ替えのキー。期限切れの状態は作らない）。
    pub deadline: Option<TsMs>,
}

/// 疑似送信 1 件の記録。実外部への送信ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedSendRecord {
    pub simulated_send_id: SimulatedSendId,
    pub card_id: CardId,
    pub card_version_approved: u64,
    /// 承認された `snapshot_hash`（正規化した payload の hash。コメントは含まない）。
    pub snapshot_hash: String,
}

/// 冪等キーが結び付く承認の内容。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalBinding {
    pub card_id: CardId,
    pub card_version: u64,
    pub snapshot_hash: String,
}

impl ApprovalBinding {
    fn of(req: &ApproveRequest) -> Self {
        Self {
            card_id: req.card_id.clone(),
            card_version: req.card_version,
            snapshot_hash: req.snapshot_hash.clone(),
        }
    }
}

/// 冪等キーの状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdempotencyState {
    /// 送信前に記録した。送信はまだ終わっていない。
    InProgress,
    /// 送信が終わった。再送にはこの結果を返す。
    Completed(ApproveOutcome),
}

#[derive(Debug)]
struct IdempotencyEntry {
    binding: ApprovalBinding,
    state: IdempotencyState,
}

/// 送信の予約。`Clone` できないので、1 つの予約から送信できるのは 1 回だけ。
#[derive(Debug)]
pub struct SendTicket {
    key: IdempotencyKey,
    binding: ApprovalBinding,
}

impl SendTicket {
    #[must_use]
    pub fn binding(&self) -> &ApprovalBinding {
        &self.binding
    }
}

/// [`ApprovalLedger::begin_approve`] の結果。
#[derive(Debug)]
pub enum BeginApproval {
    /// キーを「処理中」で記録した。続けて送信するか、送信前に中止する。
    Reserved(SendTicket),
    /// 同じキー・同じ内容の承認は完了済み。最初の結果を返す（送信しない）。
    Replay(ApproveOutcome),
}

/// 承認カードの台帳。
#[derive(Debug, Default)]
pub struct ApprovalLedger {
    cards: HashMap<CardId, ApprovalCard>,
    idempotency: HashMap<IdempotencyKey, IdempotencyEntry>,
    send_log: Vec<SimulatedSendRecord>,
    /// カードごとの受け渡し記録（#89）。古い順に並ぶ。
    handoff_log: HashMap<CardId, Vec<HandoffRecord>>,
    /// ワークスペースの承認の設定（ADR 0002）。承認者の解決と
    /// 「依頼者が承認者を兼ねてよいか」の検査に使う（#86）。
    policy: ApprovalPolicy,
    next_comment: u64,
    next_send: u64,
    next_handoff: u64,
}

impl ApprovalLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 承認の設定（ADR 0002）を指定して作る。
    #[must_use]
    pub fn with_policy(policy: ApprovalPolicy) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }

    /// 承認の設定（ADR 0002）を差し替える。**既存カードの承認者には影響しない**
    /// （承認者はカード作成時に解決して固定する。ADR 0002）。この差し替えは
    /// 以降のカード作成の解決（役割→代表）にだけ効く。
    pub fn set_policy(&mut self, policy: ApprovalPolicy) {
        self.policy = policy;
    }

    /// 現在の承認の設定。
    #[must_use]
    pub fn policy(&self) -> &ApprovalPolicy {
        &self.policy
    }

    /// カードを「送信承認待ち」・版 1 で作る。同じ ID が既にあれば
    /// [`CreateCardError::DuplicateCardId`]。
    /// 承認者（`decider`）はここで解決してカードに固定する（ADR 0002）:
    /// 指名した1人ならその人、役割なら作成時点の代表。役割が解決できなければ
    /// [`CreateCardError::UnresolvedDeciderRole`]。
    /// 依頼者が承認者に解決され、設定（[`ApprovalPolicy::allow_requester_as_decider`]）が
    /// 兼ねることを許さなければ [`CreateCardError::RequesterAsDeciderNotAllowed`]（#86）。
    ///
    /// # Errors
    /// 上のとおり。エラーのときは台帳の状態を変えない。
    pub fn create_card(&mut self, new: NewCard) -> Result<&ApprovalCard, CreateCardError> {
        if self.cards.contains_key(&new.card_id) {
            return Err(CreateCardError::DuplicateCardId);
        }
        // 承認者を作成時に解決して固定する（以後、設定を変えても動かない）。
        let decider = crate::approval::resolve_decider(&new.decider, &self.policy)
            .ok_or(CreateCardError::UnresolvedDeciderRole)?;
        // 依頼者が承認者を兼ねてよいかはワークスペースの設定で決める（ADR 0002・#86）。
        // 役割が依頼者に解決される場合も兼ねているのと同じ扱い。
        if !self.policy.allow_requester_as_decider && decider == new.requester {
            return Err(CreateCardError::RequesterAsDeciderNotAllowed);
        }
        let snapshot_hash = compute_snapshot_hash(&new.payload);
        let card = ApprovalCard {
            workspace_id: new.workspace_id,
            channel_id: new.channel_id,
            card_id: new.card_id.clone(),
            card_version: 1,
            payload: new.payload,
            snapshot_hash,
            state: TaskState::AwaitingSendApproval,
            requester: new.requester,
            decider,
            deadline: new.deadline,
            comments: Vec::new(),
            simulated_send_id: None,
        };
        Ok(self.cards.entry(new.card_id).or_insert(card))
    }

    #[must_use]
    pub fn card(&self, card_id: &CardId) -> Option<&ApprovalCard> {
        self.cards.get(card_id)
    }

    /// 冪等キーの現在の状態（監査表示・テスト用）。解放済み・未使用なら `None`。
    #[must_use]
    pub fn idempotency_state(&self, key: &IdempotencyKey) -> Option<&IdempotencyState> {
        self.idempotency.get(key).map(|e| &e.state)
    }

    /// 送信前の payload 改訂。版を上げ、前の版への承認を無効にする
    /// （03 §3：予約前の編集は前承認を失効させ、送信承認待ちのまま）。
    ///
    /// # Errors
    /// カードが無い、または送信承認待ちでなければエラー。
    pub fn revise_snapshot(
        &mut self,
        card_id: &CardId,
        payload: ActionPayload,
    ) -> Result<u64, ApproveError> {
        let card = self
            .cards
            .get_mut(card_id)
            .ok_or(ApproveError::CardNotFound)?;
        if card.state != TaskState::AwaitingSendApproval {
            return Err(ApproveError::NotAwaitingApproval { state: card.state });
        }
        card.payload = payload;
        card.snapshot_hash = compute_snapshot_hash(&card.payload);
        card.card_version += 1;
        Ok(card.card_version)
    }

    /// コメントを保存する。**状態も `card_version` も `snapshot_hash` も変えず、
    /// 送信もしない。**コメントは `snapshot_hash` に含まれないため（#87）、
    /// 保存前の hash での承認はそのまま有効。
    ///
    /// 送信承認待ちでないカード（送信中・送信済み等）への保存は
    /// [`CommentError::NotAwaitingApproval`] で断る。承認が確定したカードの
    /// 議論欄を後から変えないため。本文の内容（空白・上限超過）より先に状態を
    /// 確かめ、承認待ちでなければすべて 409 にする（リード決定 16:11）。
    ///
    /// # Errors
    /// カードが無い場合。カードが送信承認待ちでない場合
    /// （[`CommentError::NotAwaitingApproval`]、`snapshot_hash` は変えない）。
    /// 本文が空白だけ、または長すぎる場合（[`validate_comment`]）。
    pub fn save_comment(
        &mut self,
        author_id: MemberId,
        member_kind: MemberKind,
        req: SaveCommentRequest,
    ) -> Result<CommentSaved, CommentError> {
        let card = self
            .cards
            .get_mut(&req.card_id)
            .ok_or(CommentError::CardNotFound)?;
        if card.state != TaskState::AwaitingSendApproval {
            return Err(CommentError::NotAwaitingApproval { state: card.state });
        }
        validate_comment(&req.body)?;
        self.next_comment += 1;
        let comment_id = CommentId(format!("cmt-{:08}", self.next_comment));
        card.comments.push(CardComment {
            comment_id: comment_id.clone(),
            author_id,
            member_kind,
            body: req.body,
        });
        Ok(CommentSaved {
            card_id: card.card_id.clone(),
            comment_id,
            card_version: card.card_version,
            snapshot_hash: card.snapshot_hash.clone(),
            state: card.state,
        })
    }

    /// 承認の前半。冪等キーを確かめ、検査に通ったら送信前にキーを「処理中」で記録する。
    ///
    /// 検査順：カードの存在 → 操作者の身元（承認者か。ADR 0002・#86）→ 冪等キーの
    /// 既存記録 → 版 → snapshot hash → 状態。操作者が承認者でなければ
    /// [`ApproveError::NotDecider`]（403）で、キーも記録しない。
    ///
    /// # Errors
    /// [`ApproveError`] のいずれか。エラーのときは何も送らず、キーも記録しない
    /// （既存の記録は変えない）。
    pub fn begin_approve(
        &mut self,
        req: &ApproveRequest,
        operator: &MemberId,
    ) -> Result<BeginApproval, ApproveError> {
        let binding = ApprovalBinding::of(req);
        // カードの存在を先に確かめる（身元・キー・版より前。ADR 0002・#86）。
        if !self.cards.contains_key(&req.card_id) {
            return Err(ApproveError::CardNotFound);
        }
        // 操作した人がカードの承認者か（ADR 0002・#86）。承認者はカード作成時に
        // 解決して固定した1人なので、比較はそれだけでよい。冪等キーの検査より前で
        // 断るので、承認者でない人はキーのリプレイ（完了済み結果の再取得）もできない。
        if self.cards[&req.card_id].decider != *operator {
            return Err(ApproveError::NotDecider);
        }
        if let Some(entry) = self.idempotency.get(&req.idempotency_key) {
            if entry.binding != binding {
                return Err(ApproveError::IdempotencyKeyReused);
            }
            return match &entry.state {
                IdempotencyState::Completed(outcome) => Ok(BeginApproval::Replay(outcome.clone())),
                IdempotencyState::InProgress => Err(ApproveError::ApprovalInProgress),
            };
        }

        let card = self
            .cards
            .get_mut(&req.card_id)
            .ok_or(ApproveError::CardNotFound)?;
        if card.card_version != req.card_version {
            return Err(ApproveError::VersionConflict {
                current_version: card.card_version,
            });
        }
        if card.snapshot_hash != req.snapshot_hash {
            return Err(ApproveError::SnapshotMismatch);
        }
        if card.state != TaskState::AwaitingSendApproval {
            return Err(ApproveError::NotAwaitingApproval { state: card.state });
        }

        // 送信前にキーを記録する。ここから先で応答が失われても、再送は二重送信にならない。
        card.state = TaskState::Sending;
        self.idempotency.insert(
            req.idempotency_key.clone(),
            IdempotencyEntry {
                binding: binding.clone(),
                state: IdempotencyState::InProgress,
            },
        );
        Ok(BeginApproval::Reserved(SendTicket {
            key: req.idempotency_key.clone(),
            binding,
        }))
    }

    /// 承認の後半。予約に対して疑似送信を 1 回行い、キーを「完了」にする。
    ///
    /// 送信中 → 疑似送信 → 送信済み・顧客回答待ち。実システムでは予約・開始受理・
    /// 外部 I/O が別段階（03 §5、TODO(U01)）。
    ///
    /// # Errors
    /// 予約のカードが消えている場合（台帳の外から壊されない限り起きない）。
    pub fn perform_simulated_send(
        &mut self,
        ticket: SendTicket,
    ) -> Result<ApproveOutcome, ApproveError> {
        let card = self
            .cards
            .get_mut(&ticket.binding.card_id)
            .ok_or(ApproveError::CardNotFound)?;
        self.next_send += 1;
        let simulated_send_id = SimulatedSendId(format!("sim-{:08}", self.next_send));
        self.send_log.push(SimulatedSendRecord {
            simulated_send_id: simulated_send_id.clone(),
            card_id: card.card_id.clone(),
            card_version_approved: ticket.binding.card_version,
            snapshot_hash: ticket.binding.snapshot_hash.clone(),
        });
        card.state = TaskState::SentAwaitingReply;
        card.card_version += 1;
        card.simulated_send_id = Some(simulated_send_id.clone());

        let outcome = ApproveOutcome {
            card_id: card.card_id.clone(),
            card_version: card.card_version,
            state: card.state,
            simulated_send_id,
        };
        self.idempotency.insert(
            ticket.key,
            IdempotencyEntry {
                binding: ticket.binding,
                state: IdempotencyState::Completed(outcome.clone()),
            },
        );
        Ok(outcome)
    }

    /// 何も送らないまま中止する（送信前の失敗）。キーを解放し、カードを
    /// 「送信承認待ち」に戻す。送信が始まったかもしれない場合には使わないこと。
    ///
    /// クレート内（単体テスト）でのみ使う。HTTP 層は `approve` だけを呼び、
    /// 予約を持ち出さない（依頼 2・作業2 の方針）。
    /// ライブラリ部では呼ばれないため、テスト専用として許可する。
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn abort_before_send(&mut self, ticket: SendTicket) {
        self.idempotency.remove(&ticket.key);
        if let Some(card) = self.cards.get_mut(&ticket.binding.card_id)
            && card.state == TaskState::Sending
        {
            card.state = TaskState::AwaitingSendApproval;
        }
    }

    /// 明示的な承認（[`Self::begin_approve`] → [`Self::perform_simulated_send`]）。
    /// `operator` は操作した人の身元（#86）。カードの承認者でなければ
    /// [`ApproveError::NotDecider`] で断る。
    ///
    /// # Errors
    /// [`ApproveError`] のいずれか。
    pub fn approve(
        &mut self,
        req: ApproveRequest,
        operator: &MemberId,
    ) -> Result<ApproveOutcome, ApproveError> {
        match self.begin_approve(&req, operator)? {
            BeginApproval::Replay(outcome) => Ok(outcome),
            BeginApproval::Reserved(ticket) => self.perform_simulated_send(ticket),
        }
    }

    /// 疑似送信の記録（テスト・監査表示用）。
    #[must_use]
    pub fn send_log(&self) -> &[SimulatedSendRecord] {
        &self.send_log
    }

    /// workspace 内のメンバーの判断待ち一覧（[`crate::inbox::Inbox`]）を作る。
    /// 一覧用のデータは別に保存せず、この台帳のカード（状態のテーブル）を
    /// 検索してその場で組み立てる。詳細は [`crate::inbox`] のモジュール説明。
    #[must_use]
    pub fn inbox(&self, workspace_id: &WorkspaceId, member: &MemberId) -> crate::inbox::Inbox {
        crate::inbox::build_inbox(
            member,
            self.cards
                .values()
                .filter(|c| &c.workspace_id == workspace_id),
        )
    }

    /// カードの受け渡し記録（#89。古い順）。
    #[must_use]
    pub fn handoff_log(&self, card_id: &CardId) -> &[HandoffRecord] {
        self.handoff_log
            .get(card_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// #89: Bot → Bot の受け渡し。引き継ぎの 5 項目を持つ記録として、サーバーを
    /// 通して行う。詳細な不変条件は [`crate::handoff`] のモジュール説明。
    ///
    /// # Errors
    /// [`HandoffError`] のいずれか（4xx。カードも記録も変えない）。
    /// 段数の上限・同じ 2 者の行き来はエラーではなく [`HandoffDecision::Refused`]
    /// （担当を依頼者に戻し、チャンネルへの通知を返す）。
    pub fn hand_off(
        &mut self,
        req: HandoffRequest,
        now: TsMs,
    ) -> Result<HandoffDecision, HandoffError> {
        // 1. 検査（4xx）。ここで断った要求はカードも記録も変えない。
        //    承認者はカード作成時に固定した1人（ADR 0002）なので素直に比較する。
        let card = self
            .cards
            .get_mut(&req.card_id)
            .ok_or(HandoffError::CardNotFound)?;
        if card.decider != req.from {
            return Err(HandoffError::NotCurrentDecider {
                current: card.decider.clone(),
            });
        }
        if req.from == req.to {
            return Err(HandoffError::SelfHandoff);
        }

        // 2. 段数と行き来の検査（拒否）。どちらも担当を依頼者に戻し、通知を返す。
        let chain = self.handoff_log.entry(req.card_id.clone()).or_default();
        let depth = u32::try_from(chain.len())
            .unwrap_or(u32::MAX)
            .saturating_add(1);
        if depth > MAX_HANDOFF_DEPTH {
            return Ok(refuse_handoff(
                card,
                &req,
                depth,
                HandoffRefusalReason::DepthLimitExceeded {
                    depth,
                    max: MAX_HANDOFF_DEPTH,
                },
            ));
        }
        if chain.iter().any(|r| {
            (r.from == req.from && r.to == req.to) || (r.from == req.to && r.to == req.from)
        }) {
            // 「同じ 2 者の間」は向きによらず同じ組として表す。
            let mut between = [req.from.clone(), req.to.clone()];
            between.sort();
            return Ok(refuse_handoff(
                card,
                &req,
                depth,
                HandoffRefusalReason::Bouncing { between },
            ));
        }

        // 3. 受理。担当（誰の判断待ちか）を依頼先へ移し、記録を台帳に残す。
        //    段数・記録 ID・時刻はここでサーバーが決める。
        self.next_handoff += 1;
        let record = HandoffRecord {
            handoff_id: HandoffId(format!("hnd-{:08}", self.next_handoff)),
            card_id: req.card_id.clone(),
            from: req.from,
            to: req.to,
            context: req.context,
            depth,
            created_at: now,
        };
        chain.push(record.clone());
        card.decider = record.to.clone();
        Ok(HandoffDecision::Accepted { record })
    }
}

/// 受け渡しの拒否（段数の上限・同じ 2 者の行き来）。カードの担当を依頼者に戻し、
/// チャンネルへの通知を作る。
fn refuse_handoff(
    card: &mut ApprovalCard,
    req: &HandoffRequest,
    depth: u32,
    reason: HandoffRefusalReason,
) -> HandoffDecision {
    card.decider = card.requester.clone();
    HandoffDecision::Refused {
        refusal: HandoffRefusal {
            reason: reason.clone(),
            notification: HandoffNotification {
                channel_id: card.channel_id.clone(),
                reason_key: reason.notification_key().to_owned(),
                from: req.from.clone(),
                to: req.to.clone(),
                depth,
            },
            returned_to: card.requester.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::{COMMENT_MAX_CHARS, comment_len};
    use crate::task::ActionKind;

    fn payload() -> ActionPayload {
        ActionPayload {
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: "合成データの本文\n".to_owned(),
            attachments: vec!["sha256:att-1".to_owned()],
            account: "acct-cw-1".to_owned(),
            scope: vec!["tickets:write".to_owned()],
        }
    }

    /// `payload()` と内容が違う改訂版（宛先と本文を変える）。
    fn payload_v2() -> ActionPayload {
        ActionPayload {
            recipient: "other@example.invalid".to_owned(),
            body: "改訂後の本文".to_owned(),
            ..payload()
        }
    }

    fn key(s: &str) -> IdempotencyKey {
        IdempotencyKey::new(format!("test-key-{s}-000000")).expect("valid key")
    }

    fn new_card(id: &str) -> NewCard {
        NewCard {
            workspace_id: "ws-synthetic".into(),
            channel_id: "ch-synthetic".into(),
            card_id: id.into(),
            payload: payload(),
            requester: "m-human-1".into(),
            decider: Decider::Member("m-human-1".into()),
            deadline: None,
        }
    }

    /// テストのカードの承認者（`new_card` が指名した人）。
    fn decider() -> MemberId {
        "m-human-1".into()
    }

    fn ledger_with_card(id: &str) -> ApprovalLedger {
        let mut ledger = ApprovalLedger::new();
        ledger.create_card(new_card(id)).expect("new card");
        ledger
    }

    fn hash_of(ledger: &ApprovalLedger, card: &str) -> String {
        ledger.card(&card.into()).unwrap().snapshot_hash.clone()
    }

    fn approve_req(card: &str, version: u64, hash: &str, k: &str) -> ApproveRequest {
        ApproveRequest {
            card_id: card.into(),
            card_version: version,
            snapshot_hash: hash.to_owned(),
            idempotency_key: key(k),
        }
    }

    /// 画面に出ている現在の版と hash で承認要求を作る。
    fn current_req(ledger: &ApprovalLedger, card: &str, k: &str) -> ApproveRequest {
        let c = ledger.card(&card.into()).unwrap();
        approve_req(card, c.card_version, &c.snapshot_hash, k)
    }

    fn comment(ledger: &mut ApprovalLedger, card: &str, body: &str) -> CommentSaved {
        ledger
            .save_comment(
                "m-human-1".into(),
                MemberKind::Human,
                SaveCommentRequest {
                    card_id: card.into(),
                    body: body.to_owned(),
                },
            )
            .expect("comment saved")
    }

    fn reserve(ledger: &mut ApprovalLedger, req: &ApproveRequest) -> SendTicket {
        match ledger.begin_approve(req, &decider()).expect("reserved") {
            BeginApproval::Reserved(t) => t,
            BeginApproval::Replay(_) => panic!("expected a reservation"),
        }
    }

    // ---- 送信は approve だけ ----

    #[test]
    fn nothing_is_sent_without_approve() {
        let ledger = ledger_with_card("card-1");
        let card = ledger.card(&"card-1".into()).unwrap();
        assert_eq!(card.state, TaskState::AwaitingSendApproval);
        assert!(card.simulated_send_id.is_none());
        assert!(ledger.send_log().is_empty());
        assert_eq!(card.snapshot_hash, compute_snapshot_hash(&payload()));
    }

    #[test]
    fn comment_save_never_transitions_to_sent() {
        let mut ledger = ledger_with_card("card-1");
        // Enter で入った改行を含むコメントも、ただのコメント。
        for body in [
            "了解です",
            "1行目\n2行目",
            "\n先頭に改行",
            "@bot 確認して\n",
        ] {
            let saved = comment(&mut ledger, "card-1", body);
            assert_eq!(saved.state, TaskState::AwaitingSendApproval);
            assert_eq!(saved.card_version, 1);
            assert!(!saved.state.is_send_side());
        }
        let card = ledger.card(&"card-1".into()).unwrap();
        assert_eq!(card.comments.len(), 4);
        assert_eq!(card.comments[1].body, "1行目\n2行目");
        assert_eq!(card.state, TaskState::AwaitingSendApproval);
        assert!(card.simulated_send_id.is_none());
        assert!(ledger.send_log().is_empty());
    }

    // ---- コメントと snapshot_hash ----

    // #87 の受け入れ条件: コメントは snapshot_hash に含まれない。
    #[test]
    fn comment_keeps_snapshot_hash_and_version() {
        let mut ledger = ledger_with_card("card-1");
        let before = hash_of(&ledger, "card-1");
        let saved = comment(&mut ledger, "card-1", "宛先を確認しました");
        assert_eq!(saved.card_version, 1);
        assert_eq!(saved.snapshot_hash, before);
        assert_eq!(saved.snapshot_hash, hash_of(&ledger, "card-1"));
        let saved2 = comment(&mut ledger, "card-1", "宛先を確認しました");
        // 同じ本文の別コメントでも hash は同じまま。
        assert_eq!(saved2.snapshot_hash, before);
        assert_eq!(saved2.card_version, 1);
    }

    // #87 の主訴求: コメントを書くだけで承認が無効にならない。
    #[test]
    fn approve_with_pre_comment_hash_succeeds_after_comments() {
        let mut ledger = ledger_with_card("card-1");
        let pre = hash_of(&ledger, "card-1");
        comment(&mut ledger, "card-1", "本文に追記してから送って");
        comment(&mut ledger, "card-1", "宛先も確認済み");
        // コメント前の画面の hash のまま承認できる。
        let out = ledger
            .approve(approve_req("card-1", 1, &pre, "a"), &decider())
            .unwrap();
        assert_eq!(out.state, TaskState::SentAwaitingReply);
        assert_eq!(ledger.send_log().len(), 1);
        assert_eq!(ledger.send_log()[0].snapshot_hash, pre);
    }

    #[test]
    fn comment_after_send_is_rejected_and_does_not_resend() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        let out = ledger.approve(req.clone(), &decider()).unwrap();
        let before = hash_of(&ledger, "card-1");
        let res = ledger.save_comment(
            "m-human-1".into(),
            MemberKind::Human,
            SaveCommentRequest {
                card_id: "card-1".into(),
                body: "送ったあとのメモ".to_owned(),
            },
        );
        assert_eq!(
            res,
            Err(CommentError::NotAwaitingApproval {
                state: TaskState::SentAwaitingReply
            })
        );
        assert_eq!(res.unwrap_err().http_status(), 409);
        // 拒否されたコメントで hash は変わらない。
        assert_eq!(hash_of(&ledger, "card-1"), before);
        assert!(ledger.card(&"card-1".into()).unwrap().comments.is_empty());
        assert_eq!(ledger.send_log().len(), 1);
        // 同じキーの再送は最初の結果のまま。
        assert_eq!(ledger.approve(req, &decider()).unwrap(), out);
        assert_eq!(ledger.send_log().len(), 1);
    }

    #[test]
    fn comment_on_sending_card_is_rejected_and_hash_stays() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        let ticket = reserve(&mut ledger, &req);
        let before = hash_of(&ledger, "card-1");
        let res = ledger.save_comment(
            "m-human-1".into(),
            MemberKind::Human,
            SaveCommentRequest {
                card_id: "card-1".into(),
                body: "送信中のメモ".to_owned(),
            },
        );
        // 送信中へのコメントは 409。予約・送信記録が持つ古い hash を壊さない。
        assert_eq!(
            res,
            Err(CommentError::NotAwaitingApproval {
                state: TaskState::Sending
            })
        );
        assert_eq!(res.unwrap_err().http_status(), 409);
        assert_eq!(hash_of(&ledger, "card-1"), before);
        let card = ledger.card(&"card-1".into()).unwrap();
        assert!(card.comments.is_empty());
        assert_eq!(card.state, TaskState::Sending);
        // 送信前に中止して承認待ちに戻れば、コメントは保存できる。
        ledger.abort_before_send(ticket);
        assert_eq!(
            comment(&mut ledger, "card-1", "中止後のメモ").state,
            TaskState::AwaitingSendApproval
        );
    }

    #[test]
    fn invalid_body_on_non_awaiting_card_returns_409_not_422() {
        // リード決定（16:11）: AwaitingSendApproval 以外のすべての状態で 409 not_awaiting_approval。
        // 本文の検証（422）より状態の検査（409）が先。
        let blank = " \n ".to_owned();
        let over = "あ".repeat(COMMENT_MAX_CHARS + 1);
        let mut sent = ledger_with_card("card-1");
        sent.approve(current_req(&sent, "card-1", "a"), &decider())
            .unwrap();
        let mut sending = ledger_with_card("card-2");
        let sending_req = current_req(&sending, "card-2", "a");
        let _ticket = reserve(&mut sending, &sending_req);
        let sent_before = card_summary(&sent, "card-1");
        let sending_before = card_summary(&sending, "card-2");
        for (ledger, card, state) in [
            (&mut sent, "card-1", TaskState::SentAwaitingReply),
            (&mut sending, "card-2", TaskState::Sending),
        ] {
            for body in [&blank, &over] {
                let res = ledger.save_comment(
                    "m-human-1".into(),
                    MemberKind::Human,
                    SaveCommentRequest {
                        card_id: card.into(),
                        body: body.clone(),
                    },
                );
                assert_eq!(res, Err(CommentError::NotAwaitingApproval { state }));
                assert_eq!(res.unwrap_err().http_status(), 409);
            }
        }
        // 拒否のあと、snapshot_hash・コメント数・版は呼び出し前と変わらない。
        assert_eq!(card_summary(&sent, "card-1"), sent_before);
        assert_eq!(card_summary(&sending, "card-2"), sending_before);
    }

    /// カードの (snapshot_hash, コメント数, card_version)。拒否で何も変わっていないことの確認用。
    fn card_summary(ledger: &ApprovalLedger, card: &str) -> (String, usize, u64) {
        let c = ledger.card(&card.into()).unwrap();
        (c.snapshot_hash.clone(), c.comments.len(), c.card_version)
    }

    #[test]
    fn empty_or_oversized_comment_is_rejected_without_side_effects() {
        let mut ledger = ledger_with_card("card-1");
        let before = hash_of(&ledger, "card-1");
        let empty = ledger.save_comment(
            "m-human-1".into(),
            MemberKind::Human,
            SaveCommentRequest {
                card_id: "card-1".into(),
                body: " \n ".to_owned(),
            },
        );
        assert_eq!(empty, Err(CommentError::EmptyBody));
        let long = ledger.save_comment(
            "m-human-1".into(),
            MemberKind::Human,
            SaveCommentRequest {
                card_id: "card-1".into(),
                body: "あ".repeat(COMMENT_MAX_CHARS + 1),
            },
        );
        assert!(matches!(long, Err(CommentError::TooLong { .. })));
        let card = ledger.card(&"card-1".into()).unwrap();
        assert!(card.comments.is_empty());
        assert_eq!(card.snapshot_hash, before);
    }

    #[test]
    fn comment_at_exact_limit_is_saved() {
        let mut ledger = ledger_with_card("card-1");
        let body = "\u{1F600}".repeat(COMMENT_MAX_CHARS);
        let saved = comment(&mut ledger, "card-1", &body);
        assert_eq!(saved.card_version, 1);
    }

    #[test]
    fn comment_limit_counts_code_points_with_flags() {
        // 国旗は地域指示子 2 つ = 2 コードポイント。上限はコードポイント数で数える。
        let flag = "\u{1F1EF}\u{1F1F5}";
        assert_eq!(comment_len(flag), 2);
        let mut ledger = ledger_with_card("card-1");
        // 2000 個でちょうど 4000 コードポイント：受理。
        let exact = flag.repeat(COMMENT_MAX_CHARS / 2);
        assert_eq!(comment_len(&exact), COMMENT_MAX_CHARS);
        let saved = comment(&mut ledger, "card-1", &exact);
        assert_eq!(saved.card_version, 1);
        assert_eq!(saved.snapshot_hash, hash_of(&ledger, "card-1"));
        // 地域指示子をもう 1 つ（+1 コードポイント）で 4001：too_long で拒否、hash は変わらない。
        let before = hash_of(&ledger, "card-1");
        let over = format!("{exact}\u{1F1EF}");
        assert_eq!(comment_len(&over), COMMENT_MAX_CHARS + 1);
        assert_eq!(
            ledger.save_comment(
                "m-human-1".into(),
                MemberKind::Human,
                SaveCommentRequest {
                    card_id: "card-1".into(),
                    body: over,
                },
            ),
            Err(CommentError::TooLong {
                max_chars: COMMENT_MAX_CHARS
            })
        );
        assert_eq!(hash_of(&ledger, "card-1"), before);
    }

    // ---- 承認と疑似送信 ----

    #[test]
    fn approve_sends_once_with_stable_id_and_sent_awaiting_reply() {
        let mut ledger = ledger_with_card("card-1");
        let out = ledger
            .approve(current_req(&ledger, "card-1", "a"), &decider())
            .unwrap();
        assert_eq!(out.state, TaskState::SentAwaitingReply);
        assert_eq!(out.card_version, 2);
        assert_eq!(out.simulated_send_id.as_str(), "sim-00000001");
        assert_eq!(ledger.send_log().len(), 1);
        let card = ledger.card(&"card-1".into()).unwrap();
        assert_eq!(
            card.simulated_send_id.as_ref(),
            Some(&out.simulated_send_id)
        );
        assert_eq!(ledger.send_log()[0].card_version_approved, 1);
        assert_eq!(
            ledger.idempotency_state(&key("a")),
            Some(&IdempotencyState::Completed(out))
        );
    }

    #[test]
    fn version_mismatch_is_rejected_and_sends_nothing() {
        let mut ledger = ledger_with_card("card-1");
        assert_eq!(
            ledger.revise_snapshot(&"card-1".into(), payload_v2()),
            Ok(2)
        );
        let h = hash_of(&ledger, "card-1");
        let stale = ledger.approve(approve_req("card-1", 1, &h, "a"), &decider());
        assert_eq!(
            stale,
            Err(ApproveError::VersionConflict { current_version: 2 })
        );
        let future = ledger.approve(approve_req("card-1", 9, &h, "b"), &decider());
        assert_eq!(
            future,
            Err(ApproveError::VersionConflict { current_version: 2 })
        );
        assert!(ledger.send_log().is_empty());
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().state,
            TaskState::AwaitingSendApproval
        );
    }

    // #87 の受け入れ条件: hash の対象は正規化した payload だけ。
    #[test]
    fn revise_changes_snapshot_hash_only_when_payload_changes() {
        let mut ledger = ledger_with_card("card-1");
        comment(&mut ledger, "card-1", "メモ");
        let before = hash_of(&ledger, "card-1");
        // 内容の違う payload への改訂: hash が変わり、版も上がる。
        ledger
            .revise_snapshot(&"card-1".into(), payload_v2())
            .unwrap();
        let card = ledger.card(&"card-1".into()).unwrap();
        assert_eq!(card.snapshot_hash, compute_snapshot_hash(&card.payload));
        assert_ne!(card.snapshot_hash, before);
        assert_eq!(card.card_version, 2);
    }

    // #87 の受け入れ条件: 同じ意味の payload（正規化で同一）は同じ hash。
    #[test]
    fn revise_to_equivalent_payload_keeps_hash_but_bumps_version() {
        let mut ledger = ledger_with_card("card-1");
        let before = hash_of(&ledger, "card-1");
        // 本文の改行コードだけが違う（同じ意味の）payload への改訂。
        let mut equivalent = payload();
        equivalent.body = "合成データの本文\r\n".to_owned();
        ledger
            .revise_snapshot(&"card-1".into(), equivalent)
            .unwrap();
        let card = ledger.card(&"card-1".into()).unwrap();
        assert_eq!(card.snapshot_hash, before);
        assert_eq!(card.card_version, 2);
    }

    #[test]
    fn snapshot_mismatch_is_rejected_and_sends_nothing() {
        let mut ledger = ledger_with_card("card-1");
        let res = ledger.approve(approve_req("card-1", 1, "sha256:wrong", "a"), &decider());
        assert_eq!(res, Err(ApproveError::SnapshotMismatch));
        assert!(ledger.send_log().is_empty());
    }

    // ---- 冪等キー ----

    #[test]
    fn conflict_409_before_send_releases_the_key() {
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        let res = ledger.approve(approve_req("card-1", 7, &h, "a"), &decider());
        assert_eq!(res.as_ref().unwrap_err().http_status(), 409);
        assert!(ledger.idempotency_state(&key("a")).is_none());
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().state,
            TaskState::AwaitingSendApproval
        );
        // クライアントは 409 のあと読み直し、新しいキーで承認する（クライアントの規則）。
        let out = ledger
            .approve(current_req(&ledger, "card-1", "b"), &decider())
            .unwrap();
        assert_eq!(ledger.send_log().len(), 1);
        // 解放されたキー a は何にも結び付いていない。
        assert!(ledger.idempotency_state(&key("a")).is_none());
        assert_eq!(
            ledger.idempotency_state(&key("b")),
            Some(&IdempotencyState::Completed(out))
        );
    }

    #[test]
    fn released_key_is_not_burned_on_the_server() {
        // サーバーは 409 のキーを記録しないので、同じキーで正しい内容に直しても通る。
        // （クライアントは新しいキーを作る規則。これはサーバー側の性質の確認。）
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        assert!(
            ledger
                .approve(approve_req("card-1", 7, &h, "a"), &decider())
                .is_err()
        );
        assert!(
            ledger
                .approve(approve_req("card-1", 1, &h, "a"), &decider())
                .is_ok()
        );
        assert_eq!(ledger.send_log().len(), 1);
    }

    #[test]
    fn response_lost_after_successful_send_retry_does_not_resend() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        // 1 回目：送信は成功したが、応答がクライアントに届かなかった（結果を捨てる）。
        let first = ledger.approve(req.clone(), &decider()).unwrap();
        assert_eq!(ledger.send_log().len(), 1);
        // クライアントは同じキーで送り直す。2 回目の送信は起きず、同じ結果が返る。
        for _ in 0..5 {
            assert_eq!(ledger.approve(req.clone(), &decider()).unwrap(), first);
        }
        assert_eq!(ledger.send_log().len(), 1);
        assert_eq!(first.simulated_send_id.as_str(), "sim-00000001");
    }

    #[test]
    fn retry_while_in_progress_does_not_send_again() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        let ticket = reserve(&mut ledger, &req);
        assert_eq!(
            ledger.idempotency_state(&key("a")),
            Some(&IdempotencyState::InProgress)
        );
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().state,
            TaskState::Sending
        );
        // 処理中に同じキー・同じ内容で再送：409 の処理中、送信しない。
        let retry = ledger.approve(req.clone(), &decider());
        assert_eq!(retry, Err(ApproveError::ApprovalInProgress));
        assert_eq!(retry.unwrap_err().http_status(), 409);
        assert!(ledger.send_log().is_empty());
        // 処理中に別のキーで承認：送信承認待ちではないので拒否、送信しない。
        let other = ledger.approve(req_with_key(&req, "b"), &decider());
        assert_eq!(
            other,
            Err(ApproveError::NotAwaitingApproval {
                state: TaskState::Sending
            })
        );
        assert!(ledger.idempotency_state(&key("b")).is_none());
        assert!(ledger.send_log().is_empty());
        // 最初の処理が終わると、同じキーの再送は最初の結果を返す。
        let out = ledger.perform_simulated_send(ticket).unwrap();
        assert_eq!(ledger.approve(req, &decider()).unwrap(), out);
        assert_eq!(ledger.send_log().len(), 1);
    }

    fn req_with_key(req: &ApproveRequest, k: &str) -> ApproveRequest {
        ApproveRequest {
            idempotency_key: key(k),
            ..req.clone()
        }
    }

    #[test]
    fn same_key_different_binding_while_in_progress_is_rejected() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        let _ticket = reserve(&mut ledger, &req);
        let mut other = req.clone();
        other.snapshot_hash = "sha256:other".to_owned();
        assert_eq!(
            ledger.approve(other, &decider()),
            Err(ApproveError::IdempotencyKeyReused)
        );
        assert_eq!(
            ledger.idempotency_state(&key("a")),
            Some(&IdempotencyState::InProgress)
        );
        assert!(ledger.send_log().is_empty());
    }

    #[test]
    fn same_key_with_different_card_version_or_hash_after_completion_is_rejected() {
        let mut ledger = ledger_with_card("card-1");
        ledger.create_card(new_card("card-2")).unwrap();
        let req = current_req(&ledger, "card-1", "a");
        let first = ledger.approve(req.clone(), &decider()).unwrap();

        let mut other_card = current_req(&ledger, "card-2", "a");
        other_card.card_version = req.card_version;
        let mut other_version = req.clone();
        other_version.card_version += 1;
        let mut other_hash = req.clone();
        other_hash.snapshot_hash = "sha256:other".to_owned();

        for bad in [other_card, other_version, other_hash] {
            let res = ledger.approve(bad, &decider());
            // 前の結果を返さずに断る。
            assert_eq!(res, Err(ApproveError::IdempotencyKeyReused));
            assert_eq!(res.unwrap_err().http_status(), 422);
        }
        assert_eq!(ledger.send_log().len(), 1);
        assert_eq!(
            ledger.idempotency_state(&key("a")),
            Some(&IdempotencyState::Completed(first))
        );
        // card-2 は送られていない。
        assert_eq!(
            ledger.card(&"card-2".into()).unwrap().state,
            TaskState::AwaitingSendApproval
        );
    }

    #[test]
    fn abort_before_send_releases_key_and_restores_card() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        let ticket = reserve(&mut ledger, &req);
        ledger.abort_before_send(ticket);
        assert!(ledger.idempotency_state(&key("a")).is_none());
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().state,
            TaskState::AwaitingSendApproval
        );
        assert!(ledger.send_log().is_empty());
        // 何も送っていないので、同じ内容でもう一度承認できる。
        assert!(ledger.approve(req, &decider()).is_ok());
        assert_eq!(ledger.send_log().len(), 1);
    }

    #[test]
    fn second_approval_with_new_key_never_sends_twice() {
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        ledger
            .approve(approve_req("card-1", 1, &h, "a"), &decider())
            .unwrap();
        // 同時に開いていた別画面（古い版）からの承認。
        let stale = ledger.approve(approve_req("card-1", 1, &h, "b"), &decider());
        assert_eq!(
            stale,
            Err(ApproveError::VersionConflict { current_version: 2 })
        );
        // 最新の版を読み直しても、送信済みなので受け付けない。
        let fresh = ledger.approve(current_req(&ledger, "card-1", "c"), &decider());
        assert_eq!(
            fresh,
            Err(ApproveError::NotAwaitingApproval {
                state: TaskState::SentAwaitingReply
            })
        );
        assert_eq!(ledger.send_log().len(), 1);
    }

    #[test]
    fn simulated_send_ids_are_unique_per_card() {
        let mut ledger = ledger_with_card("card-1");
        ledger.create_card(new_card("card-2")).unwrap();
        let a = ledger
            .approve(current_req(&ledger, "card-1", "a"), &decider())
            .unwrap();
        let b = ledger
            .approve(current_req(&ledger, "card-2", "b"), &decider())
            .unwrap();
        assert_ne!(a.simulated_send_id, b.simulated_send_id);
    }

    #[test]
    fn unknown_card_and_path_mismatch_are_rejected() {
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        let res = ledger.approve(approve_req("card-x", 1, &h, "a"), &decider());
        assert_eq!(res, Err(ApproveError::CardNotFound));
        assert!(ledger.idempotency_state(&key("a")).is_none());
        let req = approve_req("card-1", 1, &h, "b");
        assert_eq!(
            req.check_path(&"card-2".into()),
            Err(ApproveError::CardIdMismatch)
        );
        assert_eq!(req.check_path(&"card-1".into()), Ok(()));
        assert!(ledger.send_log().is_empty());
    }

    #[test]
    fn http_status_mapping() {
        assert_eq!(ApproveError::CardNotFound.http_status(), 404);
        assert_eq!(
            ApproveError::VersionConflict { current_version: 1 }.http_status(),
            409
        );
        assert_eq!(ApproveError::SnapshotMismatch.http_status(), 409);
        assert_eq!(ApproveError::ApprovalInProgress.http_status(), 409);
        assert_eq!(ApproveError::IdempotencyKeyReused.http_status(), 422);
        assert_eq!(CommentError::EmptyBody.http_status(), 422);
        assert_eq!(
            CommentError::NotAwaitingApproval {
                state: TaskState::Sending
            }
            .http_status(),
            409
        );
    }

    #[test]
    fn duplicate_card_id_is_refused() {
        let mut ledger = ledger_with_card("card-1");
        let mut dup = new_card("card-1");
        dup.payload = payload_v2();
        assert_eq!(
            ledger.create_card(dup).unwrap_err(),
            CreateCardError::DuplicateCardId
        );
        assert_eq!(ledger.card(&"card-1".into()).unwrap().payload, payload());
    }

    // ---- #86: 承認者の身元と権限 ----

    /// 承認者を指名した1人にしたカードと、役割（代表）にしたカード。
    fn card_with_decider(id: &str, decider: Decider) -> NewCard {
        NewCard {
            decider,
            ..new_card(id)
        }
    }

    /// 代表を m-human-rep にした設定。
    fn policy_with_representative() -> ApprovalPolicy {
        ApprovalPolicy {
            allow_requester_as_decider: true,
            representative: Some("m-human-rep".into()),
        }
    }

    // #86 のやること1 + レビュー2 (PR #119): 承認者は作成時に解決してカードに
    // 固定する (ADR 0002「承認カードを作るときに承認者を決める」)。
    // 指名した1人ならその人、役割なら作成時点の代表。カードは解決後の1人を持ち、
    // 作成後の代表の変更 (set_policy) でも差し替わらない。承認者の変更は明示的な
    // 再割当て (handoff) だけが行う。
    #[test]
    fn create_card_resolves_the_decider_at_creation() {
        let mut ledger = ApprovalLedger::with_policy(policy_with_representative());
        ledger
            .create_card(card_with_decider(
                "card-member",
                Decider::Member("m-human-2".into()),
            ))
            .expect("new card");
        ledger
            .create_card(card_with_decider(
                "card-role",
                Decider::Role(crate::approval::ApproverRole::Representative),
            ))
            .expect("new card");
        assert_eq!(
            ledger.card(&"card-member".into()).unwrap().decider,
            "m-human-2".into(),
            "指名した1人はそのまま承認者になる"
        );
        assert_eq!(
            ledger.card(&"card-role".into()).unwrap().decider,
            "m-human-rep".into(),
            "役割は作成時の代表に解決される"
        );
        // 作成後に代表を変えても、既存カードの承認者は作成時のまま動かない。
        ledger.set_policy(ApprovalPolicy {
            allow_requester_as_decider: true,
            representative: Some("m-human-new-rep".into()),
        });
        assert_eq!(
            ledger.card(&"card-role".into()).unwrap().decider,
            "m-human-rep".into(),
            "代表の変更が既存カードの承認者を差し替えない"
        );
    }

    // #86 のやること2・4: 承認者でない人の承認（依頼者を含む）は拒否され、何も送らない。
    #[test]
    fn approve_by_non_decider_is_rejected_and_sends_nothing() {
        let mut ledger = ApprovalLedger::new();
        ledger
            .create_card(card_with_decider(
                "card-1",
                Decider::Member("m-human-2".into()),
            ))
            .expect("new card");
        let req = current_req(&ledger, "card-1", "a");
        let before = card_summary(&ledger, "card-1");
        // m-human-1 は依頼者（new_card の requester）だが承認者ではない。
        let res = ledger.approve(req, &"m-human-1".into());
        assert_eq!(res, Err(ApproveError::NotDecider));
        assert_eq!(res.unwrap_err().http_status(), 403);
        // 状態は変わらない。送信記録も冪等キーの記録も無い。
        assert_eq!(card_summary(&ledger, "card-1"), before);
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().state,
            TaskState::AwaitingSendApproval
        );
        assert!(ledger.send_log().is_empty());
        assert!(ledger.idempotency_state(&key("a")).is_none());
        // 指名された人なら承認できる。
        ledger
            .approve(current_req(&ledger, "card-1", "b"), &"m-human-2".into())
            .expect("decider can approve");
        assert_eq!(ledger.send_log().len(), 1);
    }

    // #86: 役割（代表）を承認者にしたカードは、代表だけが承認できる。
    #[test]
    fn role_decider_allows_only_the_representative() {
        let mut ledger = ApprovalLedger::with_policy(policy_with_representative());
        ledger
            .create_card(card_with_decider(
                "card-role",
                Decider::Role(crate::approval::ApproverRole::Representative),
            ))
            .expect("new card");
        let req = current_req(&ledger, "card-role", "a");
        // 代表でない人は拒否。
        assert_eq!(
            ledger.approve(req.clone(), &"m-human-1".into()),
            Err(ApproveError::NotDecider)
        );
        assert!(ledger.send_log().is_empty());
        // レビュー2 (PR #119): 作成後に代表が変わっても、承認できるのは
        // カード作成時の代表だけ（承認者は作成時に固定。ADR 0002）。
        ledger.set_policy(ApprovalPolicy {
            allow_requester_as_decider: true,
            representative: Some("m-human-new-rep".into()),
        });
        assert_eq!(
            ledger.approve(req.clone(), &"m-human-new-rep".into()),
            Err(ApproveError::NotDecider),
            "新しい代表では承認できない"
        );
        // 作成時の代表なら承認できる。
        ledger
            .approve(req, &"m-human-rep".into())
            .expect("the representative at card creation can approve");
        assert_eq!(ledger.send_log().len(), 1);
    }

    // #86: 承認済みキーのリプレイも、承認者でなければ 403（キーの検査より前）。
    #[test]
    fn non_decider_cannot_replay_completed_key() {
        let mut ledger = ApprovalLedger::new();
        ledger
            .create_card(card_with_decider(
                "card-1",
                Decider::Member("m-human-2".into()),
            ))
            .expect("new card");
        let req = current_req(&ledger, "card-1", "a");
        let out = ledger.approve(req.clone(), &"m-human-2".into()).unwrap();
        // 他人が同じキー・同じ内容で送り直しても最初の結果を返さない。
        assert_eq!(
            ledger.approve(req.clone(), &"m-human-1".into()),
            Err(ApproveError::NotDecider)
        );
        assert_eq!(ledger.send_log().len(), 1);
        // 承認済みキー「a」の再送はリプレイとして同じ結果を返す。
        let replay = ledger.approve(req, &"m-human-2".into()).unwrap();
        assert_eq!(replay, out);
        assert_eq!(ledger.send_log().len(), 1);
    }

    // #86 のやること3: 依頼者が承認者を兼ねてよいかはワークスペースの設定で切り替える。
    // 指名・役割のどちらでも、承認者が依頼者に解決されるカードが対象。
    #[test]
    fn requester_as_decider_policy_toggles_card_creation() {
        // 兼ねてよい（既定）: 依頼者を承認者にしたカードを作れる。
        let mut allowed = ApprovalLedger::new();
        assert!(allowed.policy().allow_requester_as_decider);
        allowed.create_card(new_card("card-ok")).expect("new card");

        // 兼ねてはならない: 依頼者に解決されるカードは作れない。
        let mut ledger = ApprovalLedger::with_policy(ApprovalPolicy {
            allow_requester_as_decider: false,
            representative: Some("m-human-rep".into()),
        });
        assert_eq!(
            ledger.create_card(new_card("card-self")).unwrap_err(),
            CreateCardError::RequesterAsDeciderNotAllowed
        );
        assert!(ledger.card(&"card-self".into()).is_none());
        // 依頼者と承認者が別なら作れる。
        let mut other = new_card("card-other");
        other.decider = Decider::Member("m-human-2".into());
        ledger.create_card(other).expect("new card");
        // 役割が依頼者に解決される場合も兼ねているのと同じ。
        let mut rep_self = new_card("card-rep-self");
        rep_self.requester = "m-human-rep".into();
        rep_self.decider = Decider::Role(crate::approval::ApproverRole::Representative);
        assert_eq!(
            ledger.create_card(rep_self).unwrap_err(),
            CreateCardError::RequesterAsDeciderNotAllowed
        );
    }

    // #86: 役割が解決できない（代表が未設定）カードは作れない。
    #[test]
    fn unresolved_role_cannot_create_card() {
        let mut ledger = ApprovalLedger::new();
        assert!(ledger.policy().representative.is_none());
        assert_eq!(
            ledger
                .create_card(card_with_decider(
                    "card-role",
                    Decider::Role(crate::approval::ApproverRole::Representative),
                ))
                .unwrap_err(),
            CreateCardError::UnresolvedDeciderRole
        );
        assert!(ledger.card(&"card-role".into()).is_none());
    }
}
