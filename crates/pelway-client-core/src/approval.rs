//! 承認カードの状態機械。「次に何をするか」（[`Effect`]）を返すだけで、通信は書かない。
//!
//! 冪等キーは押すたびに [`KeySource`] から新しいものを 1 つ作る。同じキーを使い回すのは
//! （1）`approval_in_progress` の確認（同じキーでの再送はサーバーが 2 重送信しない）、
//! （2）確認あきらめ後の再押下での確認、のときだけ。
//!
//! `approval_in_progress` の確認は同じキーで最大 3 回、1 秒・2 秒・4 秒の間隔で行う。
//! 3 回で決まらなければ「処理中のため確認できません」を出してボタンを押せる状態に
//! 戻し、もう一度押されたら同じキーで確認し直す（新しい送信をしない）。
//!
//! 409 の `version_conflict`・`snapshot_mismatch`・`not_awaiting_approval`（と
//! `card_not_found`）は自動で再送しない。読み直し（[`ApprovalMachine::reload`]）を
//! 求める状態に移し、次の承認は新しいキーで行う。
//! 422 の `idempotency_key_reused`・`card_id_mismatch` はクライアントの不具合として
//! 扱い、再送しない。
//!
//! コメント欄はコメント API だけを呼び、承認は承認ボタンからだけ呼ぶ。
//! コメント欄はカードが承認待ちのときだけ書ける（リード決定 2026-11-09 16:12）。
//! 承認待ち以外（送信中・送信後など）ではいつも読み取り専用にし、コメント API を
//! 呼ばない。かつての「承認の通信中だけ読み取り専用」はこの規則に置き換えた。
//! コメントを保存したら応答の `snapshot_hash` に差し替える（コメント保存では
//! hash は変わらない、#87。値は保存前と同じ）。`card_version` は
//! 表示中の値をそのまま承認に送る（コメント保存では版は変わらない）。
//! 承認でもコメント保存でも 409 `not_awaiting_approval`（や 404 `card_not_found`）
//! が返ったら自動で再送せず読み直しを求め、次の承認は新しいキーで行う。

use crate::contract::{
    ApproveError, ApproveOutcome, ApproveRequest, COMMENT_MAX_CHARS, CardId, CommentError,
    CommentSaved, IdempotencyKey, SaveCommentRequest, TaskState,
};

/// 承認操作ごとに新しい冪等キーを作る処理。実装は UUID 生成等を注入する。
pub trait KeySource {
    /// 新しい冪等キーを 1 つ返す。
    fn fresh_key(&mut self) -> IdempotencyKey;
}

/// 時計。確認の間隔（1 秒・2 秒・4 秒）を測るために使う。
/// 読み取りのみで、状態は変更しない（実時間の実装も `Instant::now()` などを返す）。
pub trait Clock {
    /// 単調に増える現在時刻（ミリ秒）。実時間との結び付けは呼び出し側が決める。
    fn now_ms(&self) -> u64;
}

impl<T: KeySource + ?Sized> KeySource for &mut T {
    fn fresh_key(&mut self) -> IdempotencyKey {
        (**self).fresh_key()
    }
}

impl<T: Clock + ?Sized> Clock for &T {
    fn now_ms(&self) -> u64 {
        (**self).now_ms()
    }
}

/// `approval_in_progress` を確認する間隔（ミリ秒）。最大 3 回。
const RECHECK_DELAYS_MS: [u64; 3] = [1000, 2000, 4000];

/// 表示中の承認カード。承認・コメント保存の要求はこの値を使う。
/// コメント欄が書けるのはこのカードが承認待ち（`state` が
/// [`TaskState::AwaitingSendApproval`]）のときだけ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardView {
    pub card_id: CardId,
    pub card_version: u64,
    pub snapshot_hash: String,
    pub state: TaskState,
}

/// 読み直しが必要な理由（409 系と 404）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReloadReason {
    /// 画面の版が古い。
    VersionConflict { current_version: u64 },
    /// 画面の snapshot が古い。
    SnapshotMismatch,
    /// 送信承認待ちではない。
    NotAwaitingApproval { state: TaskState },
    /// カードが見つからない。
    CardNotFound,
}

/// クライアントの不具合の内容（422 系）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BugReason {
    IdempotencyKeyReused,
    CardIdMismatch,
}

/// 状態機械が呼び出し側（UI）に伝える表示上の通知。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Notice {
    /// `approval_in_progress` の確認を 3 回行っても決まらなかった。
    CannotCheckApproval,
    /// 読み直しが必要。
    NeedsReload { reason: ReloadReason },
    /// クライアントの不具合。再送しない。
    ClientBug { reason: BugReason },
    /// コメント本文が空白。
    CommentBlank,
    /// コメント本文が長すぎる。
    CommentTooLong { max_chars: usize },
    /// コメント欄が読み取り専用（カードが承認待ちでない）のためコメント API を呼ばない。
    CommentLocked,
}

/// コメント保存の要求を試みた結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentAttempt {
    /// コメント API を呼ぶ。
    Send(SaveCommentRequest),
    /// 呼ばない。理由は [`Notice`]。
    Reject(Notice),
}

/// 状態機械が次にすること。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// 承認 API への要求。初回の押下も、処理中の確認も同じ形。
    Approve(ApproveRequest),
    /// コメント保存 API への要求。
    SaveComment(SaveCommentRequest),
    /// 表示への通知。
    Notice(Notice),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Phase {
    /// 承認待ち。ボタンもコメント欄も有効。
    Idle,
    /// 承認要求（初回押下・再押下・確認のいずれか）の応答待ち。
    /// `checks` はこのキーで `approval_in_progress` の確認を済ませた回数。
    AwaitingResponse { key: IdempotencyKey, checks: u32 },
    /// 確認の実行を予約した状態。[`ApprovalMachine::poll`] で
    /// [`Phase::AwaitingResponse`] に移る。
    RecheckScheduled {
        key: IdempotencyKey,
        checks: u32,
        run_at_ms: u64,
    },
    /// 確認 3 回でも決まらず。ボタンは押せる（同じキーで確認し直す）。
    CannotCheck { key: IdempotencyKey },
    /// 409/404 系。読み直し待ち。
    NeedsReload { reason: ReloadReason },
    /// 422 系。クライアントの不具合。再送しない。
    ClientBug { reason: BugReason },
    /// 承認完了。
    Approved,
}

/// 承認カード 1 枚の状態機械。
///
/// 通信そのものは行わない。[`Effect`] を呼び出し側が実行し、
/// その応答を `on_*` メソッドで戻す。時間が経ったら [`ApprovalMachine::poll`] を
/// 呼び出す（確認の再試行タイマーの期限が来ていれば確認要求を返す）。
pub struct ApprovalMachine<K: KeySource, C: Clock> {
    card: CardView,
    keys: K,
    clock: C,
    phase: Phase,
}

impl<K: KeySource, C: Clock> ApprovalMachine<K, C> {
    /// 表示するカードと、キー生成・時計の実装を受け取る。
    pub fn new(card: CardView, keys: K, clock: C) -> Self {
        Self {
            card,
            keys,
            clock,
            phase: Phase::Idle,
        }
    }

    /// 表示中のカード。コメント保存の応答で `snapshot_hash` が差し替わる。
    #[must_use]
    pub fn card(&self) -> &CardView {
        &self.card
    }

    /// コメント欄が編集可能か。カードが承認待ちのときだけ true（リード決定 2026-11-09 16:12）。
    /// 承認待ち以外（送信中・送信後など）ではいつも読み取り専用にし、コメント API を
    /// 呼ばない。承認の通信中かどうかは見ない（カードが承認待ちなら通信中も書ける）。
    /// 読み直し待ち・不具合・承認済みの各状態では、カードの状態に関係なく読み取り専用にする。
    #[must_use]
    pub fn comment_editable(&self) -> bool {
        self.card.state == TaskState::AwaitingSendApproval
            && !matches!(
                self.phase,
                Phase::NeedsReload { .. } | Phase::ClientBug { .. } | Phase::Approved
            )
    }

    /// 承認ボタンが押せるか。承認待ちのカードで、承認待機中か確認あきらめ後のとき。
    #[must_use]
    pub fn approve_button_enabled(&self) -> bool {
        matches!(self.phase, Phase::Idle | Phase::CannotCheck { .. })
            && self.card.state == TaskState::AwaitingSendApproval
    }

    /// 承認ボタンを押した。押すたびに新しい冪等キーを 1 つ作る。
    /// ただし確認あきらめ後の再押下は、二重送信を避けるため同じキーで確認し直す。
    /// 表示中のカードが承認待ちでなければ `None` を返し、キーを消費しない
    /// （レビュー指摘2: UI の無効表示だけに頼らず状態機械自身が防ぐ）。
    /// 押せない状態（通信中・読み直し待ち・不具合・承認済み）でも `None` を返し何もしない。
    pub fn press_approve(&mut self) -> Option<Effect> {
        if self.card.state != TaskState::AwaitingSendApproval {
            return None;
        }
        let reuse = match &self.phase {
            Phase::Idle => None,
            Phase::CannotCheck { key } => Some(key.clone()),
            Phase::AwaitingResponse { .. }
            | Phase::RecheckScheduled { .. }
            | Phase::NeedsReload { .. }
            | Phase::ClientBug { .. }
            | Phase::Approved => return None,
        };
        let key = reuse.unwrap_or_else(|| self.keys.fresh_key());
        let req = self.approve_request(&key);
        self.phase = Phase::AwaitingResponse { key, checks: 0 };
        Some(Effect::Approve(req))
    }

    /// 時間の経過を伝える。確認の実行時刻（1 秒・2 秒・4 秒後）が来ていれば、
    /// 同じキーでの確認要求を [`Effect::Approve`] として返す。それ以外は `None`。
    pub fn poll(&mut self) -> Option<Effect> {
        let (key, checks, run_at_ms) = match &self.phase {
            Phase::RecheckScheduled {
                key,
                checks,
                run_at_ms,
            } => (key.clone(), *checks, *run_at_ms),
            _ => return None,
        };
        if self.clock.now_ms() < run_at_ms {
            return None;
        }
        let req = self.approve_request(&key);
        self.phase = Phase::AwaitingResponse {
            key,
            checks: checks + 1,
        };
        Some(Effect::Approve(req))
    }

    /// 承認 API の応答を戻す。承認要求の応答待ちのときだけ処理する。
    pub fn on_approve_response(
        &mut self,
        result: Result<ApproveOutcome, ApproveError>,
    ) -> Option<Effect> {
        let (key, checks) = match &self.phase {
            Phase::AwaitingResponse { key, checks } => (key.clone(), *checks),
            _ => return None,
        };
        match result {
            Ok(outcome) => {
                // 承認でカードは送信側の状態へ移る。コメント欄は以後読み取り専用。
                self.card.state = outcome.state;
                self.phase = Phase::Approved;
                None
            }
            Err(err) => match err {
                ApproveError::ApprovalInProgress => {
                    let delay = match RECHECK_DELAYS_MS.get(checks as usize) {
                        Some(delay_ms) => *delay_ms,
                        // 3 回確認しても決まらない → 確認をあきらめる。
                        None => {
                            self.phase = Phase::CannotCheck { key };
                            return Some(Effect::Notice(Notice::CannotCheckApproval));
                        }
                    };
                    let run_at_ms = self.clock.now_ms() + delay;
                    self.phase = Phase::RecheckScheduled {
                        key,
                        checks,
                        run_at_ms,
                    };
                    None
                }
                ApproveError::VersionConflict { current_version } => {
                    let reason = ReloadReason::VersionConflict { current_version };
                    self.enter_needs_reload(reason)
                }
                ApproveError::SnapshotMismatch => {
                    self.enter_needs_reload(ReloadReason::SnapshotMismatch)
                }
                ApproveError::NotAwaitingApproval { state } => {
                    self.enter_needs_reload(ReloadReason::NotAwaitingApproval { state })
                }
                ApproveError::CardNotFound => self.enter_needs_reload(ReloadReason::CardNotFound),
                ApproveError::IdempotencyKeyReused => {
                    let reason = BugReason::IdempotencyKeyReused;
                    self.phase = Phase::ClientBug {
                        reason: reason.clone(),
                    };
                    Some(Effect::Notice(Notice::ClientBug { reason }))
                }
                ApproveError::CardIdMismatch => {
                    let reason = BugReason::CardIdMismatch;
                    self.phase = Phase::ClientBug {
                        reason: reason.clone(),
                    };
                    Some(Effect::Notice(Notice::ClientBug { reason }))
                }
            },
        }
    }

    /// コメントを保存しようとする。コメント欄はコメント API だけを呼ぶ。
    /// カードが承認待ちのときだけ呼べる（承認待ち以外では読み取り専用）。
    /// 本文は [`crate::contract::validate_comment`] で先に検査し、空白・長すぎは
    /// API を呼ばず [`CommentAttempt::Reject`] を返す。
    pub fn request_save_comment(&mut self, body: String) -> CommentAttempt {
        if !self.comment_editable() {
            return CommentAttempt::Reject(Notice::CommentLocked);
        }
        match crate::contract::validate_comment(&body) {
            Ok(()) => CommentAttempt::Send(SaveCommentRequest {
                card_id: self.card.card_id.clone(),
                body,
            }),
            Err(CommentError::EmptyBody) => CommentAttempt::Reject(Notice::CommentBlank),
            Err(CommentError::TooLong { .. }) => CommentAttempt::Reject(Notice::CommentTooLong {
                max_chars: COMMENT_MAX_CHARS,
            }),
            Err(CommentError::CardNotFound) | Err(CommentError::NotAwaitingApproval { .. }) => {
                unreachable!("クライアント側の検査では CardNotFound・NotAwaitingApproval は出ない")
            }
        }
    }

    /// コメント API の成功応答を戻す。返ってきた `snapshot_hash` に差し替える
    /// （コメント保存では hash は変わらない、#87）。
    /// `card_version` はコメント保存では変わらないため、表示中の値をそのまま使う。
    /// （応答の `card_id` が表示中と違う場合は破棄する。）
    pub fn on_comment_saved(&mut self, saved: CommentSaved) {
        if saved.card_id == self.card.card_id {
            self.card.snapshot_hash = saved.snapshot_hash;
        }
    }

    /// コメント API のエラー応答を戻す。
    /// 409 `not_awaiting_approval`（と 404 `card_not_found`）は自動で再送せず、
    /// カードの読み直しを求める状態へ移す。`snapshot_hash` は差し替えない。
    /// クライアント側で先に弾いているはずの `empty_body`・`too_long` は状態を変えない。
    pub fn on_comment_error(&mut self, err: CommentError) -> Option<Effect> {
        match err {
            CommentError::NotAwaitingApproval { state } => {
                self.enter_needs_reload(ReloadReason::NotAwaitingApproval { state })
            }
            CommentError::CardNotFound => self.enter_needs_reload(ReloadReason::CardNotFound),
            CommentError::EmptyBody | CommentError::TooLong { .. } => None,
        }
    }

    /// カードを読み直した。どの状態からでも承認待ちに戻る。次の承認は新しいキーで行う。
    pub fn reload(&mut self, card: CardView) {
        self.card = card;
        self.phase = Phase::Idle;
    }

    fn approve_request(&self, key: &IdempotencyKey) -> ApproveRequest {
        ApproveRequest {
            card_id: self.card.card_id.clone(),
            card_version: self.card.card_version,
            snapshot_hash: self.card.snapshot_hash.clone(),
            idempotency_key: key.clone(),
        }
    }

    fn enter_needs_reload(&mut self, reason: ReloadReason) -> Option<Effect> {
        self.phase = Phase::NeedsReload {
            reason: reason.clone(),
        };
        Some(Effect::Notice(Notice::NeedsReload { reason }))
    }
}
