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
    ApprovalCard, ApproveError, ApproveOutcome, ApproveRequest, CardComment, CommentError,
    CommentSaved, SaveCommentRequest, validate_comment,
};
use crate::ids::{
    CardId, ChannelId, CommentId, IdempotencyKey, MemberId, SimulatedSendId, WorkspaceId,
};
use crate::member::MemberKind;
use crate::snapshot::compute_snapshot_hash;
use crate::task::{ActionKind, TaskState};

/// 新しい承認カードの入力（合成データのみ）。
#[derive(Debug, Clone)]
pub struct NewCard {
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    pub card_id: CardId,
    /// 固定 payload の hash。`snapshot_hash` はこれとコメントから計算する。
    pub payload_hash: String,
    pub action_kind: ActionKind,
    pub recipient: String,
    pub body: String,
}

/// 疑似送信 1 件の記録。実外部への送信ではない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimulatedSendRecord {
    pub simulated_send_id: SimulatedSendId,
    pub card_id: CardId,
    pub card_version_approved: u64,
    /// 承認された `snapshot_hash`（承認時点のコメントを含む）。
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
    next_comment: u64,
    next_send: u64,
}

impl ApprovalLedger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// カードを「送信承認待ち」・版 1 で作る。同じ ID が既にあれば `None`。
    pub fn create_card(&mut self, new: NewCard) -> Option<&ApprovalCard> {
        if self.cards.contains_key(&new.card_id) {
            return None;
        }
        let snapshot_hash = compute_snapshot_hash(&new.payload_hash, &[]);
        let card = ApprovalCard {
            workspace_id: new.workspace_id,
            channel_id: new.channel_id,
            card_id: new.card_id.clone(),
            card_version: 1,
            payload_hash: new.payload_hash,
            snapshot_hash,
            action_kind: new.action_kind,
            recipient: new.recipient,
            body: new.body,
            state: TaskState::AwaitingSendApproval,
            comments: Vec::new(),
            simulated_send_id: None,
        };
        Some(self.cards.entry(new.card_id).or_insert(card))
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
        payload_hash: String,
        body: String,
    ) -> Result<u64, ApproveError> {
        let card = self
            .cards
            .get_mut(card_id)
            .ok_or(ApproveError::CardNotFound)?;
        if card.state != TaskState::AwaitingSendApproval {
            return Err(ApproveError::NotAwaitingApproval { state: card.state });
        }
        card.payload_hash = payload_hash;
        card.snapshot_hash = compute_snapshot_hash(&card.payload_hash, &card.comments);
        card.body = body;
        card.card_version += 1;
        Ok(card.card_version)
    }

    /// コメントを保存する。**状態も `card_version` も変えず、送信もしない。**
    /// コメントは `snapshot_hash` に含まれるので、保存後の新しい `snapshot_hash` を返す。
    ///
    /// 送信承認待ちでないカード（送信中・送信済み等）への保存は
    /// [`CommentError::NotAwaitingApproval`] で断る。コメントで `snapshot_hash` が変わると、
    /// 予約・疑似送信の記録が持つ予約時の hash と食い違うため。
    ///
    /// # Errors
    /// カードが無い、本文が空白だけ、または長すぎる場合（[`validate_comment`]）。
    /// カードが送信承認待ちでない場合（[`CommentError::NotAwaitingApproval`]、
    /// `snapshot_hash` は変えない）。
    pub fn save_comment(
        &mut self,
        author_id: MemberId,
        member_kind: MemberKind,
        req: SaveCommentRequest,
    ) -> Result<CommentSaved, CommentError> {
        validate_comment(&req.body)?;
        let card = self
            .cards
            .get_mut(&req.card_id)
            .ok_or(CommentError::CardNotFound)?;
        if card.state != TaskState::AwaitingSendApproval {
            return Err(CommentError::NotAwaitingApproval { state: card.state });
        }
        self.next_comment += 1;
        let comment_id = CommentId(format!("cmt-{:08}", self.next_comment));
        card.comments.push(CardComment {
            comment_id: comment_id.clone(),
            author_id,
            member_kind,
            body: req.body,
        });
        card.snapshot_hash = compute_snapshot_hash(&card.payload_hash, &card.comments);
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
    /// 検査順：冪等キーの既存記録 → カードの存在 → 版 → snapshot hash → 状態。
    ///
    /// # Errors
    /// [`ApproveError`] のいずれか。エラーのときは何も送らず、キーも記録しない
    /// （既存の記録は変えない）。
    pub fn begin_approve(&mut self, req: &ApproveRequest) -> Result<BeginApproval, ApproveError> {
        let binding = ApprovalBinding::of(req);
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
    pub fn abort_before_send(&mut self, ticket: SendTicket) {
        self.idempotency.remove(&ticket.key);
        if let Some(card) = self.cards.get_mut(&ticket.binding.card_id)
            && card.state == TaskState::Sending
        {
            card.state = TaskState::AwaitingSendApproval;
        }
    }

    /// 明示的な承認（[`Self::begin_approve`] → [`Self::perform_simulated_send`]）。
    ///
    /// # Errors
    /// [`ApproveError`] のいずれか。
    pub fn approve(&mut self, req: ApproveRequest) -> Result<ApproveOutcome, ApproveError> {
        match self.begin_approve(&req)? {
            BeginApproval::Replay(outcome) => Ok(outcome),
            BeginApproval::Reserved(ticket) => self.perform_simulated_send(ticket),
        }
    }

    /// 疑似送信の記録（テスト・監査表示用）。
    #[must_use]
    pub fn send_log(&self) -> &[SimulatedSendRecord] {
        &self.send_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::{COMMENT_MAX_CHARS, comment_len};

    const PAYLOAD_V1: &str = "sha256:synthetic-payload-v1";
    const PAYLOAD_V2: &str = "sha256:synthetic-payload-v2";

    fn key(s: &str) -> IdempotencyKey {
        IdempotencyKey::new(format!("test-key-{s}-000000")).expect("valid key")
    }

    fn new_card(id: &str) -> NewCard {
        NewCard {
            workspace_id: "ws-synthetic".into(),
            channel_id: "ch-synthetic".into(),
            card_id: id.into(),
            payload_hash: PAYLOAD_V1.to_owned(),
            action_kind: ActionKind::CustomerSend,
            recipient: "customer@example.invalid".to_owned(),
            body: "合成データの本文".to_owned(),
        }
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
        match ledger.begin_approve(req).expect("reserved") {
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
        assert_eq!(card.snapshot_hash, compute_snapshot_hash(PAYLOAD_V1, &[]));
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

    #[test]
    fn comment_changes_snapshot_hash_but_not_version() {
        let mut ledger = ledger_with_card("card-1");
        let before = hash_of(&ledger, "card-1");
        let saved = comment(&mut ledger, "card-1", "宛先を確認しました");
        assert_eq!(saved.card_version, 1);
        assert_ne!(saved.snapshot_hash, before);
        assert_eq!(saved.snapshot_hash, hash_of(&ledger, "card-1"));
        let saved2 = comment(&mut ledger, "card-1", "宛先を確認しました");
        // 同じ本文でも別のコメントなので hash は変わる。
        assert_ne!(saved2.snapshot_hash, saved.snapshot_hash);
        assert_eq!(saved2.card_version, 1);
    }

    #[test]
    fn approve_with_pre_comment_hash_is_rejected_and_returned_hash_succeeds() {
        let mut ledger = ledger_with_card("card-1");
        let pre = hash_of(&ledger, "card-1");
        let saved = comment(&mut ledger, "card-1", "本文に追記してから送って");
        // コメント前の画面のまま承認：拒否、送信なし、キーは記録しない。
        let stale = ledger.approve(approve_req("card-1", 1, &pre, "a"));
        assert_eq!(stale, Err(ApproveError::SnapshotMismatch));
        assert_eq!(stale.unwrap_err().http_status(), 409);
        assert!(ledger.send_log().is_empty());
        assert!(ledger.idempotency_state(&key("a")).is_none());
        // コメント保存の応答の値で承認：成功。
        let out = ledger
            .approve(approve_req(
                "card-1",
                saved.card_version,
                &saved.snapshot_hash,
                "b",
            ))
            .unwrap();
        assert_eq!(out.state, TaskState::SentAwaitingReply);
        assert_eq!(ledger.send_log().len(), 1);
        assert_eq!(ledger.send_log()[0].snapshot_hash, saved.snapshot_hash);
    }

    #[test]
    fn comment_after_send_is_rejected_and_does_not_resend() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        let out = ledger.approve(req.clone()).unwrap();
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
        assert_eq!(ledger.approve(req).unwrap(), out);
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
        let out = ledger.approve(current_req(&ledger, "card-1", "a")).unwrap();
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
            ledger.revise_snapshot(&"card-1".into(), PAYLOAD_V2.to_owned(), "改訂".to_owned()),
            Ok(2)
        );
        let h = hash_of(&ledger, "card-1");
        let stale = ledger.approve(approve_req("card-1", 1, &h, "a"));
        assert_eq!(
            stale,
            Err(ApproveError::VersionConflict { current_version: 2 })
        );
        let future = ledger.approve(approve_req("card-1", 9, &h, "b"));
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

    #[test]
    fn revise_keeps_comments_in_snapshot_hash() {
        let mut ledger = ledger_with_card("card-1");
        comment(&mut ledger, "card-1", "メモ");
        ledger
            .revise_snapshot(&"card-1".into(), PAYLOAD_V2.to_owned(), "改訂".to_owned())
            .unwrap();
        let card = ledger.card(&"card-1".into()).unwrap();
        assert_eq!(
            card.snapshot_hash,
            compute_snapshot_hash(PAYLOAD_V2, &card.comments)
        );
        assert_ne!(card.snapshot_hash, compute_snapshot_hash(PAYLOAD_V2, &[]));
    }

    #[test]
    fn snapshot_mismatch_is_rejected_and_sends_nothing() {
        let mut ledger = ledger_with_card("card-1");
        let res = ledger.approve(approve_req("card-1", 1, "sha256:wrong", "a"));
        assert_eq!(res, Err(ApproveError::SnapshotMismatch));
        assert!(ledger.send_log().is_empty());
    }

    // ---- 冪等キー ----

    #[test]
    fn conflict_409_before_send_releases_the_key() {
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        let res = ledger.approve(approve_req("card-1", 7, &h, "a"));
        assert_eq!(res.as_ref().unwrap_err().http_status(), 409);
        assert!(ledger.idempotency_state(&key("a")).is_none());
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().state,
            TaskState::AwaitingSendApproval
        );
        // クライアントは 409 のあと読み直し、新しいキーで承認する（クライアントの規則）。
        let out = ledger.approve(current_req(&ledger, "card-1", "b")).unwrap();
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
        assert!(ledger.approve(approve_req("card-1", 7, &h, "a")).is_err());
        assert!(ledger.approve(approve_req("card-1", 1, &h, "a")).is_ok());
        assert_eq!(ledger.send_log().len(), 1);
    }

    #[test]
    fn response_lost_after_successful_send_retry_does_not_resend() {
        let mut ledger = ledger_with_card("card-1");
        let req = current_req(&ledger, "card-1", "a");
        // 1 回目：送信は成功したが、応答がクライアントに届かなかった（結果を捨てる）。
        let first = ledger.approve(req.clone()).unwrap();
        assert_eq!(ledger.send_log().len(), 1);
        // クライアントは同じキーで送り直す。2 回目の送信は起きず、同じ結果が返る。
        for _ in 0..5 {
            assert_eq!(ledger.approve(req.clone()).unwrap(), first);
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
        let retry = ledger.approve(req.clone());
        assert_eq!(retry, Err(ApproveError::ApprovalInProgress));
        assert_eq!(retry.unwrap_err().http_status(), 409);
        assert!(ledger.send_log().is_empty());
        // 処理中に別のキーで承認：送信承認待ちではないので拒否、送信しない。
        let other = ledger.approve(req_with_key(&req, "b"));
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
        assert_eq!(ledger.approve(req).unwrap(), out);
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
            ledger.approve(other),
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
        let first = ledger.approve(req.clone()).unwrap();

        let mut other_card = current_req(&ledger, "card-2", "a");
        other_card.card_version = req.card_version;
        let mut other_version = req.clone();
        other_version.card_version += 1;
        let mut other_hash = req.clone();
        other_hash.snapshot_hash = "sha256:other".to_owned();

        for bad in [other_card, other_version, other_hash] {
            let res = ledger.approve(bad);
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
        assert!(ledger.approve(req).is_ok());
        assert_eq!(ledger.send_log().len(), 1);
    }

    #[test]
    fn second_approval_with_new_key_never_sends_twice() {
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        ledger.approve(approve_req("card-1", 1, &h, "a")).unwrap();
        // 同時に開いていた別画面（古い版）からの承認。
        let stale = ledger.approve(approve_req("card-1", 1, &h, "b"));
        assert_eq!(
            stale,
            Err(ApproveError::VersionConflict { current_version: 2 })
        );
        // 最新の版を読み直しても、送信済みなので受け付けない。
        let fresh = ledger.approve(current_req(&ledger, "card-1", "c"));
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
        let a = ledger.approve(current_req(&ledger, "card-1", "a")).unwrap();
        let b = ledger.approve(current_req(&ledger, "card-2", "b")).unwrap();
        assert_ne!(a.simulated_send_id, b.simulated_send_id);
    }

    #[test]
    fn unknown_card_and_path_mismatch_are_rejected() {
        let mut ledger = ledger_with_card("card-1");
        let h = hash_of(&ledger, "card-1");
        let res = ledger.approve(approve_req("card-x", 1, &h, "a"));
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
        dup.payload_hash = PAYLOAD_V2.to_owned();
        assert!(ledger.create_card(dup).is_none());
        assert_eq!(
            ledger.card(&"card-1".into()).unwrap().payload_hash,
            PAYLOAD_V1
        );
    }
}
