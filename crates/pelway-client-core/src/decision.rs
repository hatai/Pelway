//! 判断待ち（A/B 決定）の盤（#331）。UI とは独立し、通信をしない。
//!
//! 判断待ちの正本はメモリ上の [`ApprovalLedger`]（#328 の判断待ちテーブル）。
//! この層は「操作者（画面の向こうの人）」を束ね、一覧の読み・1件の読み・
//! 確定の入口だけを出す。確定要求は [`DecideRequest`] として組み立て、
//! 最後に組み立てたものを記録する（HTTP への結線はまだ無く、記録した要求が
//! 「これから送るもの」。`ApprovalMachine` の効果の持ち越しと同じ考え方）。

use crate::contract::{
    ApprovalLedger, ChannelId, CreateDecisionError, CreateDecisionRequest, DecideRequest,
    DecisionCard, DecisionError, DecisionId, DecisionListItem, DecisionOutcome, MemberId, OptionId,
    TsMs, WorkspaceId,
};

/// デモの判断待ちを束ねる盤。メモリ上の台帳を正本にする。
#[derive(Debug)]
pub struct DecisionBoard {
    ledger: ApprovalLedger,
    workspace: WorkspaceId,
    /// 操作者（「自分の判断待ち」の「自分」）。
    actor: MemberId,
    /// 最後に組み立てた確定要求（HTTP 結線の段階で送るもの）。
    last_decide_request: Option<DecideRequest>,
    /// 確定要求を組み立てた回数（成否を問わず）。
    decide_request_count: u32,
}

impl DecisionBoard {
    /// 操作者 `actor` の盤を作る（`workspace` が対象範囲）。
    #[must_use]
    pub fn new(workspace: WorkspaceId, actor: MemberId) -> Self {
        Self {
            ledger: ApprovalLedger::new(),
            workspace,
            actor,
            last_decide_request: None,
            decide_request_count: 0,
        }
    }

    /// 判断待ちを台帳に作る（合成データの仕込み。HTTP の結線では作成応答から入れる）。
    /// ID・版（1）・待ち開始時刻は台帳側が付ける。
    ///
    /// # Errors
    /// [`CreateDecisionError`] のいずれか（論点が空・選択肢不足・`option_id` 重複・本文なし）。
    pub fn seed(
        &mut self,
        req: CreateDecisionRequest,
        now: TsMs,
    ) -> Result<&DecisionCard, CreateDecisionError> {
        self.ledger
            .create_decision(self.workspace.clone(), req, now)
    }

    /// 自分の判断待ち一覧（待ち開始が早い順。決定済みは出ない）。
    /// 一覧の値は別に保存せず、台帳からその場で組み立てる。
    #[must_use]
    pub fn pending(&self, now: TsMs) -> Vec<DecisionListItem> {
        self.ledger
            .pending_decisions(&self.workspace, &self.actor, now)
    }

    /// 判断待ち 1 件を読む（チャット内カードの表示用。確定済みは `outcome` を持つ）。
    #[must_use]
    pub fn card(&self, decision_id: &DecisionId) -> Option<&DecisionCard> {
        self.ledger.decision(decision_id)
    }

    /// 項目が指すチャンネル（一覧から元のチャットへ戻る導線）。
    #[must_use]
    pub fn channel_of(&self, decision_id: &DecisionId) -> Option<ChannelId> {
        self.card(decision_id).map(|c| c.channel_id.clone())
    }

    /// 選択肢 `option_id` で確定する。`version` は操作者が画面で見ていた版。
    /// 確定要求を組み立てて記録してから、台帳に確定する。同じ選択の再送は
    /// 確定済みの結果を返す冪等リプレイ、別の選択は [`DecisionError::AlreadyDecided`]。
    ///
    /// # Errors
    /// [`DecisionError`] のいずれか。エラーのときは判断待ちを変えない
    /// （組み立てた要求は再送用に残る）。
    pub fn decide(
        &mut self,
        decision_id: &DecisionId,
        option_id: OptionId,
        version: u64,
        now: TsMs,
    ) -> Result<DecisionOutcome, DecisionError> {
        let req = DecideRequest {
            decision_id: decision_id.clone(),
            version,
            option_id,
        };
        self.last_decide_request = Some(req.clone());
        self.decide_request_count += 1;
        self.ledger.decide(req, &self.actor, now)
    }

    /// 最後に組み立てた確定要求（HTTP 結線の段階で送るもの。未操作なら `None`）。
    #[must_use]
    pub fn last_decide_request(&self) -> Option<&DecideRequest> {
        self.last_decide_request.as_ref()
    }

    /// 確定要求を組み立てた回数（テストと監視用）。
    #[must_use]
    pub fn decide_request_count(&self) -> u32 {
        self.decide_request_count
    }

    /// 正本の台帳への参照（他の読み取りの結線に使う）。
    #[must_use]
    pub fn ledger(&self) -> &ApprovalLedger {
        &self.ledger
    }
}

#[cfg(test)]
mod tests {
    use super::DecisionBoard;
    use crate::contract::{
        ChannelId, CreateDecisionRequest, DecisionError, DecisionOption, MemberId, MessageId,
        OptionId, WorkspaceId,
    };

    fn board() -> DecisionBoard {
        DecisionBoard::new(WorkspaceId::from("w-1"), MemberId::from("u-me"))
    }

    fn request(
        requester: &str,
        decider: &str,
        question: &str,
        options: &[(&str, &str)],
    ) -> CreateDecisionRequest {
        CreateDecisionRequest {
            channel_id: ChannelId::from("ch-1"),
            request_message_id: MessageId::from("m-1"),
            requester: MemberId::from(requester),
            decider: MemberId::from(decider),
            question: question.to_owned(),
            options: options
                .iter()
                .map(|(id, body)| DecisionOption {
                    option_id: OptionId::from(*id),
                    body: (*body).to_owned(),
                })
                .collect(),
        }
    }

    #[test]
    fn pending_lists_only_own_undecided_earliest_wait_first() {
        let mut board = board();
        let later = request(
            "u-ai",
            "u-me",
            "後に来た論点",
            &[("a", "案A"), ("b", "案B")],
        );
        let earlier = request(
            "u-ai",
            "u-me",
            "先に来た論点",
            &[("a", "案A"), ("b", "案B")],
        );
        board.seed(later, 10_000).unwrap();
        board.seed(earlier, 5_000).unwrap();
        // 別の決定者の判断待ちは自分の一覧に出ない。
        let others = request(
            "u-ai",
            "u-other",
            "他人の論点",
            &[("a", "案A"), ("b", "案B")],
        );
        board.seed(others, 1_000).unwrap();

        let items = board.pending(60_000);
        assert_eq!(items.len(), 2, "自分が決定者の未確定だけが並ぶ");
        assert_eq!(items[0].question, "先に来た論点", "待ち開始が早いものが先");
        assert_eq!(items[1].question, "後に来た論点");
        assert_eq!(items[0].wait_ms, 55_000, "wait_ms は now−waiting_since");
        assert_eq!(items[0].version, 1, "一覧の版は確定要求に書く値");
        assert_eq!(items[0].requester, MemberId::from("u-ai"));
    }

    #[test]
    fn decide_fixes_outcome_bumps_version_and_removes_item() {
        let mut board = board();
        let req = request(
            "u-ai",
            "u-me",
            "どちらにするか",
            &[("a", "案A"), ("b", "案B")],
        );
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        let outcome = board.decide(&id, OptionId::from("a"), 1, 2_000).unwrap();
        assert_eq!(outcome.selected.body, "案A", "選んだ選択肢の確定時点の本文");
        assert_eq!(outcome.decider, MemberId::from("u-me"));
        assert_eq!(outcome.version, 2, "確定で版が 1→2");
        assert_eq!(outcome.decided_at, 2_000);

        assert!(board.pending(3_000).is_empty(), "確定済みは一覧から消える");
        let card = board.card(&id).unwrap();
        assert_eq!(card.version, 2);
        assert_eq!(card.outcome.as_ref().unwrap().selected.body, "案A");
    }

    #[test]
    fn decide_records_request_with_seen_version() {
        let mut board = board();
        let req = request("u-ai", "u-me", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        board.decide(&id, OptionId::from("b"), 1, 2_000).unwrap();
        let sent = board.last_decide_request().unwrap();
        assert_eq!(sent.decision_id, id);
        assert_eq!(sent.version, 1, "画面で見ていた版を要求に書く");
        assert_eq!(sent.option_id, OptionId::from("b"));
        assert_eq!(board.decide_request_count(), 1);
    }

    #[test]
    fn decide_replay_same_option_is_idempotent() {
        let mut board = board();
        let req = request("u-ai", "u-me", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        let first = board.decide(&id, OptionId::from("a"), 1, 2_000).unwrap();
        // 同じ選択の再送（画面の版はまだ 1 のまま見えている想定）は冪等リプレイ。
        let replay = board.decide(&id, OptionId::from("a"), 1, 3_000).unwrap();
        assert_eq!(replay, first, "同じ選択の再送は確定済みの結果を返す");
        assert_eq!(board.card(&id).unwrap().version, 2, "版は上がり直さない");
    }

    #[test]
    fn decide_rejects_different_option_after_outcome() {
        let mut board = board();
        let req = request("u-ai", "u-me", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        board.decide(&id, OptionId::from("a"), 1, 2_000).unwrap();
        let err = board
            .decide(&id, OptionId::from("b"), 2, 3_000)
            .unwrap_err();
        assert_eq!(err, DecisionError::AlreadyDecided);
    }

    #[test]
    fn decide_rejects_stale_version() {
        let mut board = board();
        let req = request("u-ai", "u-me", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        let err = board
            .decide(&id, OptionId::from("a"), 99, 2_000)
            .unwrap_err();
        assert_eq!(
            err,
            DecisionError::VersionConflict { current_version: 1 },
            "画面で見ていた版と違えば 409"
        );
        assert_eq!(board.pending(3_000).len(), 1, "失敗時は一覧に残る");
    }

    #[test]
    fn decide_rejects_non_decider() {
        let mut board = board();
        // 決定者が自分（u-me）ではない判断待ちを確定しようとする。
        let req = request("u-ai", "u-other", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        let err = board
            .decide(&id, OptionId::from("a"), 1, 2_000)
            .unwrap_err();
        assert_eq!(err, DecisionError::NotDecider);
    }

    #[test]
    fn decide_rejects_unknown_option() {
        let mut board = board();
        let req = request("u-ai", "u-me", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();

        let err = board
            .decide(&id, OptionId::from("c"), 1, 2_000)
            .unwrap_err();
        assert_eq!(
            err,
            DecisionError::UnknownOption {
                option_id: OptionId::from("c")
            }
        );
    }

    #[test]
    fn channel_of_points_back_to_the_chat() {
        let mut board = board();
        let req = request("u-ai", "u-me", "論点", &[("a", "案A"), ("b", "案B")]);
        let id = board.seed(req, 1_000).unwrap().decision_id.clone();
        assert_eq!(board.channel_of(&id), Some(ChannelId::from("ch-1")));
    }
}
