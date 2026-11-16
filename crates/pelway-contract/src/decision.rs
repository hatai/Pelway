//! A/B 決定の契約（#328）。判断待ち（選択肢つきの決定待ち）の作成・確定・
//! 一覧の型と遷移。03 §3「作業中 → 判断待ち → 作業中」で、Bot の質問で開いた
//! 判断待ちを、権限ある人の決定で閉じる部分。送信承認（approve）とは別の API
//! で、疑似送信も `snapshot_hash` も関わらない。
//!
//! # 守ること（このモジュールの不変条件）
//!
//! - **決定の正本はサーバーに置く**。判断待ちは台帳
//!   （[`crate::ledger::ApprovalLedger`]）が持ち、確定した決定は
//!   選択内容・決定者・時刻・版を固定する。確定後の変更は拒否する
//!   （同じ選択の再送だけは確定済みの結果を返す冪等リプレイ）。
//! - 判断待ちは「誰の（`decider`）・何の（`question`・`options`）判断か」と、
//!   依頼メッセージへの参照（`channel_id`・`request_message_id`）・待ち開始時刻
//!   （`waiting_since`）を持つ。ID・待ち開始時刻・版・決定時刻はサーバーが付け、
//!   要求に偽の値があっても正本にしない（`deny_unknown_fields`）。
//! - **待ち時間は待ち開始時刻から出す**。一覧の `wait_ms` は、一覧を組み立てた
//!   時刻から `waiting_since` を引いた値（`InboxItem` の `deadline` で代用しない）。
//! - 決定できるのは判断待ちの `decider` だけ。作成時に固定した1人で、
//!   `decider` でない操作は [`DecisionError::NotDecider`]（403）で断る。
//! - 一覧用のデータは別に保存しない。[`build_pending_decisions`] は呼び出しの
//!   たびに判断待ちのテーブルを検索して組み立てる。決定済みは出さない。
//!
//! # まだ決まらないこと（このモジュールでは扱わない）
//!
//! - 多段階の決定、決定の期限切れの状態（#328 の対象外。ADR 0002 の承認期限は
//!   承認カードの話で、判断待ちには期限を持たせない）。
//! - 判断待ちの改訂（論点・選択肢の編集）。作成後の内容は不変とする。
//! - 判断待ちの取り下げ・再割当て。

use crate::ids::{ChannelId, DecisionId, MemberId, MessageId, OptionId, WorkspaceId};
use crate::store::TsMs;

/// 判断待ちの作成・一覧 API のパス。`{workspace_id}` は対象のワークスペース。
/// 作成は `POST`、一覧は `GET`（表示対象のメンバーはクエリ `actor`）。
pub const DECISIONS_PATH: &str = "/v1/workspaces/{workspace_id}/decisions";

/// 確定 API のパス。`{decision_id}` は対象の判断待ち。
pub const DECIDE_PATH: &str = "/v1/decisions/{decision_id}/decide";

/// [`DECISIONS_PATH`] の `{workspace_id}` を差し替えて、要求に使う実際の経路を作る。
/// サーバー（`pelway-http`）の経路判定もテストもこの関数で経路を作り、
/// 文字列リテラルで書き写さない（定数と経路の一致を構造的に保つ。Issue #91）。
#[must_use]
pub fn decisions_path(workspace_id: &WorkspaceId) -> String {
    DECISIONS_PATH.replace("{workspace_id}", workspace_id.as_str())
}

/// [`DECIDE_PATH`] の `{decision_id}` を差し替えて、要求に使う実際の経路を作る。
#[must_use]
pub fn decide_path(decision_id: &DecisionId) -> String {
    DECIDE_PATH.replace("{decision_id}", decision_id.as_str())
}

/// 選択肢の 1 つ（案A・案B など）。`option_id` は要求側が付ける安定 ID で、
/// 決定要求はこれで選択を指す。`body` は選択肢の中身（合成データ）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecisionOption {
    pub option_id: OptionId,
    pub body: String,
}

/// 判断待ち（決定を待っている項目）。決定の正本としてサーバー（台帳）が持つ。
/// `version` は確定のときだけ上がる（作成時 1 → 確定で 2）。論点・選択肢は
/// 作成後は不変（改訂はこのモジュールでは扱わない）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecisionCard {
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    pub decision_id: DecisionId,
    /// 版（確定要求が確かめる「見ていた版」。作成時 1、確定で 2）。
    pub version: u64,
    /// 何の判断か（論点。03 §3「判断者と論点を必須保存」）。
    pub question: String,
    /// 選択肢（A/B。不変版）。`option_id` はこの中で一意。
    pub options: Vec<DecisionOption>,
    /// 依頼メッセージへの参照（対象チャンネルのメッセージ）。
    pub request_message_id: MessageId,
    /// 依頼した人（一覧の「依頼者」表示に使う）。
    pub requester: MemberId,
    /// 誰の判断待ちか（決定者。作成時に固定した1人）。
    pub decider: MemberId,
    /// 待ち開始時刻（サーバー付け）。一覧の待ち時間はここから出す。
    pub waiting_since: TsMs,
    /// 確定した決定。`None` は判断待ち中。
    pub outcome: Option<DecisionOutcome>,
}

/// 確定した決定（選択内容・決定者・時刻・版）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecisionOutcome {
    pub decision_id: DecisionId,
    /// 確定後の版（判断待ちの `version` と同じ値）。
    pub version: u64,
    /// 選んだ選択肢（`option_id` と確定時点の `body` の複製）。
    pub selected: DecisionOption,
    /// 決めた人（判断待ちの `decider` と同じ）。
    pub decider: MemberId,
    /// 決定時刻（サーバー付け）。
    pub decided_at: TsMs,
}

/// `POST /v1/workspaces/{workspace_id}/decisions` の入力。
/// `decision_id`・`version`・`waiting_since` はサーバーが付けるため入力に無い
/// （`deny_unknown_fields` で未知の欄は拒否する）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CreateDecisionRequest {
    /// 対象チャンネル（依頼メッセージのある・判断待ちカードを出す先）。
    pub channel_id: ChannelId,
    /// 依頼メッセージへの参照。
    pub request_message_id: MessageId,
    /// 依頼した人（一覧の「依頼者」表示）。
    pub requester: MemberId,
    /// 誰の判断待ちか（決定者。指名した1人をそのまま固定する。
    /// 承認カードの役割解決・承認者設定は持ち込まない）。
    pub decider: MemberId,
    /// 何の判断か（論点）。
    pub question: String,
    /// 選択肢（A/B。2 つ以上）。
    pub options: Vec<DecisionOption>,
}

/// `POST /v1/decisions/{decision_id}/decide` の入力。3 項目すべて必須。
/// 確定要求を出す人（操作者）は、経路ではなく合成ヘッダーで受ける。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecideRequest {
    pub decision_id: DecisionId,
    /// 決定者が画面で見ていた版。現在の版と違えば 409。
    pub version: u64,
    /// 選んだ選択肢。
    pub option_id: OptionId,
}

/// 判断待ち一覧の 1 項目。判断待ちのコピーではなく、表示用に組み立てた値。
/// 本文は含めない（依頼メッセージへの参照から元のチャットで確認する）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecisionListItem {
    pub decision_id: DecisionId,
    /// 対象チャンネル（元のチャットへ戻る導線）。
    pub channel_id: ChannelId,
    /// 依頼メッセージへの参照。
    pub request_message_id: MessageId,
    /// 何の判断か（論点）。
    pub question: String,
    /// 選択肢（不変版。一覧の判断内容と、決定ボタンの `option_id` に使う）。
    pub options: Vec<DecisionOption>,
    /// 依頼した人。
    pub requester: MemberId,
    /// 誰の判断待ちか。
    pub decider: MemberId,
    /// 待ち開始時刻（サーバー付け）。
    pub waiting_since: TsMs,
    /// 待ち時間（一覧を組み立てた時刻 − `waiting_since`）。
    pub wait_ms: u64,
    /// 版（このあとの確定要求が `version` に書く値）。
    pub version: u64,
}

/// メンバー 1 人ぶんの判断待ち一覧を作る。一覧用のデータは保存せず、
/// 渡された判断待ちからその場で組み立てる（[`crate::inbox::build_inbox`] と
/// 同じ考え方）。入るのは「自分が決定者で、まだ決まっていない」項目だけ
/// （決定済みは出さない）。並びは待ち開始が早いものから、同じなら
/// `decision_id` の辞書順で決定的に並べる。
///
/// `decisions` は呼び出し側が範囲（workspace 等）を絞ったものを渡す
/// （[`crate::ledger::ApprovalLedger::pending_decisions`] が workspace で絞って渡す）。
/// `wait_ms` は `now` − `waiting_since`（`InboxItem` の `deadline` で代用しない）。
#[must_use]
pub fn build_pending_decisions<'a, I>(
    member: &MemberId,
    decisions: I,
    now: TsMs,
) -> Vec<DecisionListItem>
where
    I: IntoIterator<Item = &'a DecisionCard>,
{
    let mut items: Vec<DecisionListItem> = decisions
        .into_iter()
        .filter(|d| d.decider == *member && d.outcome.is_none())
        .map(|d| DecisionListItem {
            decision_id: d.decision_id.clone(),
            channel_id: d.channel_id.clone(),
            request_message_id: d.request_message_id.clone(),
            question: d.question.clone(),
            options: d.options.clone(),
            requester: d.requester.clone(),
            decider: d.decider.clone(),
            waiting_since: d.waiting_since,
            wait_ms: now.saturating_sub(d.waiting_since),
            version: d.version,
        })
        .collect();
    items.sort_by(|a, b| {
        a.waiting_since
            .cmp(&b.waiting_since)
            .then_with(|| a.decision_id.cmp(&b.decision_id))
    });
    items
}

/// 確定 API のエラー。どれも判断待ちも決定記録も変えない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "code", rename_all = "snake_case"))]
pub enum DecisionError {
    DecisionNotFound,
    DecisionIdMismatch,
    /// 操作した人はこの判断待ちの決定者ではない。作成時に固定した1人だけが
    /// 決定できる。
    NotDecider,
    /// 画面の版が古い。最新の版を読み直してから確定し直す。
    VersionConflict {
        current_version: u64,
    },
    /// 選択肢に無い `option_id`。
    UnknownOption {
        option_id: OptionId,
    },
    /// すでに確定済み（確定後の変更は受け付けない）。同じ選択の再送は
    /// 確定済みの結果を返す冪等リプレイで、このエラーにはならない。
    AlreadyDecided,
}

impl DecisionError {
    /// HTTP ステータスへの対応。
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::DecisionNotFound => 404,
            Self::NotDecider => 403,
            Self::DecisionIdMismatch | Self::UnknownOption { .. } => 422,
            Self::VersionConflict { .. } | Self::AlreadyDecided => 409,
        }
    }
}

/// 判断待ちの作成の失敗。どれも台帳の状態を変えない。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "code", rename_all = "snake_case"))]
pub enum CreateDecisionError {
    /// 論点が空白だけ（「何の判断か」は必須。03 §3「判断者と論点を必須保存」）。
    EmptyQuestion,
    /// 選択肢が 2 つ未満（比べるものが無ければ決定にならない）。
    TooFewOptions { options: usize },
    /// `option_id` が空、または同じ `option_id` の選択肢が複数ある。
    InvalidOptionId { option_id: OptionId },
    /// 選択肢の本文が空白だけ。
    EmptyOptionBody { option_id: OptionId },
}

impl CreateDecisionError {
    /// HTTP ステータスへの対応（本文の形は正しいが意味として受け付けないので 422）。
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::EmptyQuestion
            | Self::TooFewOptions { .. }
            | Self::InvalidOptionId { .. }
            | Self::EmptyOptionBody { .. } => 422,
        }
    }
}

/// 判断待ちの作成要求を検査する（台帳・HTTP どちらからも同じ検査）。
/// 論点と選択肢を渡す（`CreateDecisionRequest`・`NewDecision` どちらの
/// フィールドでも同じ型なのでそのまま渡せる）。
///
/// # Errors
/// [`CreateDecisionError`] のいずれか。
pub fn validate_new_decision(
    question: &str,
    options: &[DecisionOption],
) -> Result<(), CreateDecisionError> {
    if question.trim().is_empty() {
        return Err(CreateDecisionError::EmptyQuestion);
    }
    if options.len() < 2 {
        return Err(CreateDecisionError::TooFewOptions {
            options: options.len(),
        });
    }
    for (i, o) in options.iter().enumerate() {
        if o.option_id.as_str().is_empty()
            || options[..i].iter().any(|p| p.option_id == o.option_id)
        {
            return Err(CreateDecisionError::InvalidOptionId {
                option_id: o.option_id.clone(),
            });
        }
        if o.body.trim().is_empty() {
            return Err(CreateDecisionError::EmptyOptionBody {
                option_id: o.option_id.clone(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::ApprovalLedger;

    const WS: &str = "ws-synthetic-decision";
    const CH: &str = "ch-synthetic-decision";
    const REQUESTER: &str = "m-human-tanaka";
    const DECIDER: &str = "m-human-yamamoto";
    const OTHER: &str = "m-human-other";
    const T0: TsMs = 1_000;
    const T1: TsMs = 2_000;

    fn options() -> Vec<DecisionOption> {
        vec![
            DecisionOption {
                option_id: "a".into(),
                body: "案A: 納期を2週間延ばす".to_owned(),
            },
            DecisionOption {
                option_id: "b".into(),
                body: "案B: 範囲を絞って納期を守る".to_owned(),
            },
        ]
    }

    fn new_decision() -> CreateDecisionRequest {
        CreateDecisionRequest {
            channel_id: CH.into(),
            request_message_id: "msg-00000042".into(),
            requester: REQUESTER.into(),
            decider: DECIDER.into(),
            question: "納期延長の返事をどうするか".to_owned(),
            options: options(),
        }
    }

    fn ledger_with_decision() -> (ApprovalLedger, DecisionId) {
        let mut ledger = ApprovalLedger::new();
        let decision_id = ledger
            .create_decision(WS.into(), new_decision(), T0)
            .expect("new decision")
            .decision_id
            .clone();
        (ledger, decision_id)
    }

    fn decide_req(decision_id: &DecisionId, option_id: &str) -> DecideRequest {
        DecideRequest {
            decision_id: decision_id.clone(),
            version: 1,
            option_id: option_id.into(),
        }
    }

    // ---- 受け入れ条件 1: A/B の選択肢つきの判断待ちが作成でき、
    //      依頼メッセージへの参照と待ち開始時刻が記録される ----

    #[test]
    fn created_decision_records_options_message_ref_and_waiting_since() {
        let (ledger, decision_id) = ledger_with_decision();
        let card = ledger.decision(&decision_id).expect("decision exists");
        // ID・版・待ち開始時刻はサーバーが付けた値。
        assert_eq!(decision_id.as_str(), "dec-00000001");
        assert_eq!(card.version, 1);
        assert_eq!(card.waiting_since, T0);
        // 依頼メッセージへの参照とチャンネル・依頼者・決定者・論点・選択肢。
        assert_eq!(card.request_message_id.as_str(), "msg-00000042");
        assert_eq!(card.channel_id.as_str(), CH);
        assert_eq!(card.requester.as_str(), REQUESTER);
        assert_eq!(card.decider.as_str(), DECIDER);
        assert_eq!(card.question, "納期延長の返事をどうするか");
        assert_eq!(card.options, options());
        // 作成直後はまだ決まっていない。
        assert!(card.outcome.is_none());
        // ID は連番でサーバー採番。
        let mut ledger = ledger;
        let second = ledger
            .create_decision(WS.into(), new_decision(), T1)
            .expect("second decision");
        assert_eq!(second.decision_id.as_str(), "dec-00000002");
    }

    #[test]
    fn create_rejects_unusable_input() {
        let mut ledger = ApprovalLedger::new();
        for (req, want) in [
            (
                CreateDecisionRequest {
                    question: "  ".to_owned(),
                    ..new_decision()
                },
                CreateDecisionError::EmptyQuestion,
            ),
            (
                CreateDecisionRequest {
                    options: vec![options()[0].clone()],
                    ..new_decision()
                },
                CreateDecisionError::TooFewOptions { options: 1 },
            ),
            (
                CreateDecisionRequest {
                    options: vec![options()[0].clone(), options()[0].clone()],
                    ..new_decision()
                },
                CreateDecisionError::InvalidOptionId {
                    option_id: "a".into(),
                },
            ),
            (
                CreateDecisionRequest {
                    options: vec![
                        DecisionOption {
                            option_id: "a".into(),
                            body: "案A".to_owned(),
                        },
                        DecisionOption {
                            option_id: "".into(),
                            body: "案B".to_owned(),
                        },
                    ],
                    ..new_decision()
                },
                CreateDecisionError::InvalidOptionId {
                    option_id: "".into(),
                },
            ),
            (
                CreateDecisionRequest {
                    options: vec![
                        options()[0].clone(),
                        DecisionOption {
                            option_id: "b".into(),
                            body: " ".to_owned(),
                        },
                    ],
                    ..new_decision()
                },
                CreateDecisionError::EmptyOptionBody {
                    option_id: "b".into(),
                },
            ),
        ] {
            let got = ledger.create_decision(WS.into(), req, T0);
            assert_eq!(got.map(|c| c.decision_id.clone()), Err(want));
        }
        assert!(
            ledger
                .pending_decisions(&WS.into(), &DECIDER.into(), T1)
                .is_empty(),
            "失敗した作成は判断待ちを残さない"
        );
    }

    // ---- 受け入れ条件 2: 決定すると選択内容・決定者・時刻・版が固定され、
    //      確定後の変更が拒否される ----

    #[test]
    fn decide_fixes_selection_decider_time_and_version() {
        let (mut ledger, decision_id) = ledger_with_decision();
        let outcome = ledger
            .decide(decide_req(&decision_id, "a"), &DECIDER.into(), T1)
            .expect("decide");
        assert_eq!(outcome.decision_id, decision_id);
        assert_eq!(outcome.version, 2);
        assert_eq!(outcome.selected.option_id.as_str(), "a");
        assert_eq!(outcome.selected.body, "案A: 納期を2週間延ばす");
        assert_eq!(outcome.decider.as_str(), DECIDER);
        assert_eq!(outcome.decided_at, T1);
        // 判断待ちにも確定が残る。
        let card = ledger.decision(&decision_id).unwrap();
        assert_eq!(card.version, 2);
        assert_eq!(card.outcome.as_ref(), Some(&outcome));
    }

    #[test]
    fn decided_change_is_rejected_but_same_replay_returns_outcome() {
        let (mut ledger, decision_id) = ledger_with_decision();
        let outcome = ledger
            .decide(decide_req(&decision_id, "a"), &DECIDER.into(), T1)
            .unwrap();
        // 確定後に別の選択肢へ変える要求は拒否。
        assert_eq!(
            ledger.decide(decide_req(&decision_id, "b"), &DECIDER.into(), T1 + 1),
            Err(DecisionError::AlreadyDecided)
        );
        // 同じ選択の再送は確定済みの結果をそのまま返す（冪等リプレイ）。
        assert_eq!(
            ledger
                .decide(decide_req(&decision_id, "a"), &DECIDER.into(), T1 + 2)
                .unwrap(),
            outcome
        );
        // 版と決定記録は変わらない。
        let card = ledger.decision(&decision_id).unwrap();
        assert_eq!(card.version, 2);
        assert_eq!(card.outcome.as_ref(), Some(&outcome));
    }

    // ---- 確定要求の検査（4xx。状態を変えない） ----

    #[test]
    fn decide_checks_decider_version_option_and_id() {
        let (mut ledger, decision_id) = ledger_with_decision();
        // 決定者でない人は決定できない（確定後のリプレイ取得もできない）。
        assert_eq!(
            ledger.decide(decide_req(&decision_id, "a"), &OTHER.into(), T1),
            Err(DecisionError::NotDecider)
        );
        // 画面の版が古い。
        let stale = DecideRequest {
            version: 99,
            ..decide_req(&decision_id, "a")
        };
        assert_eq!(
            ledger.decide(stale, &DECIDER.into(), T1),
            Err(DecisionError::VersionConflict { current_version: 1 })
        );
        // 選択肢に無い option_id。
        assert_eq!(
            ledger.decide(decide_req(&decision_id, "c"), &DECIDER.into(), T1),
            Err(DecisionError::UnknownOption {
                option_id: "c".into()
            })
        );
        // 存在しない判断待ち（経路と本文の一致検査は HTTP 層の役割）。
        assert_eq!(
            ledger.decide(decide_req(&"dec-99999999".into(), "a"), &DECIDER.into(), T1),
            Err(DecisionError::DecisionNotFound)
        );
        // 何も変わっていない。
        assert!(ledger.decision(&decision_id).unwrap().outcome.is_none());
        for (err, status) in [
            (DecisionError::DecisionNotFound, 404),
            (DecisionError::NotDecider, 403),
            (DecisionError::DecisionIdMismatch, 422),
            (
                DecisionError::UnknownOption {
                    option_id: "x".into(),
                },
                422,
            ),
            (DecisionError::VersionConflict { current_version: 1 }, 409),
            (DecisionError::AlreadyDecided, 409),
        ] {
            assert_eq!(err.http_status(), status, "{err:?}");
        }
    }

    // ---- 受け入れ条件 3: 一覧は誰の・何の判断か、待ち時間、
    //      対象チャンネルとメッセージへの参照を返す ----

    #[test]
    fn pending_list_shows_whose_what_wait_time_and_message_ref() {
        let mut ledger = ApprovalLedger::new();
        let first = ledger
            .create_decision(WS.into(), new_decision(), T0)
            .expect("first")
            .decision_id
            .clone();
        let mut second_req = new_decision();
        second_req.question = "2件目の判断".to_owned();
        ledger
            .create_decision(WS.into(), second_req, T0 + 500)
            .expect("second");
        // 別ワークスペースの判断待ちは一覧に混ざらない。
        ledger
            .create_decision("ws-other".into(), new_decision(), T0)
            .expect("other ws");

        let items = ledger.pending_decisions(&WS.into(), &DECIDER.into(), T1);
        assert_eq!(items.len(), 2);
        // 並びは待ち開始が早いものから。
        assert_eq!(items[0].decision_id, first);
        assert_eq!(items[0].wait_ms, T1 - T0);
        assert_eq!(items[1].wait_ms, T1 - (T0 + 500));
        // 誰の・何の判断か、チャンネルとメッセージへの参照、版。
        let item = &items[0];
        assert_eq!(item.decider.as_str(), DECIDER);
        assert_eq!(item.requester.as_str(), REQUESTER);
        assert_eq!(item.question, "納期延長の返事をどうするか");
        assert_eq!(item.options, options());
        assert_eq!(item.channel_id.as_str(), CH);
        assert_eq!(item.request_message_id.as_str(), "msg-00000042");
        assert_eq!(item.waiting_since, T0);
        assert_eq!(item.version, 1);
        // 別の人の一覧には出ない。
        assert!(
            ledger
                .pending_decisions(&WS.into(), &OTHER.into(), T1)
                .is_empty()
        );
    }

    // ---- 受け入れ条件 4: 決定済みの項目が一覧から消える ----

    #[test]
    fn decided_items_leave_the_pending_list() {
        let (mut ledger, decision_id) = ledger_with_decision();
        assert_eq!(
            ledger
                .pending_decisions(&WS.into(), &DECIDER.into(), T1)
                .len(),
            1
        );
        ledger
            .decide(decide_req(&decision_id, "a"), &DECIDER.into(), T1)
            .unwrap();
        assert!(
            ledger
                .pending_decisions(&WS.into(), &DECIDER.into(), T1 + 1)
                .is_empty()
        );
        // 決定記録は正本として残る。
        assert!(ledger.decision(&decision_id).unwrap().outcome.is_some());
    }

    // ---- 経路 ----

    #[test]
    fn path_builders_replace_the_id_segments() {
        assert_eq!(
            decisions_path(&WS.into()),
            "/v1/workspaces/ws-synthetic-decision/decisions"
        );
        assert_eq!(decide_path(&"dec-1".into()), "/v1/decisions/dec-1/decide");
        assert!(decisions_path(&WS.into()).starts_with("/v1/workspaces/"));
    }
}
