//! 03 §3 の 9 状態と 04 §9 の操作種類。

/// タスクの状態。正規定義は 03 §3 のとおり Rust の `TaskState` 一つにする。
/// 翻訳キーと両 UI の表示はここへ対応付ける。取消は第 10 状態を作らない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum TaskState {
    Requested,
    Working,
    AwaitingDecision,
    AwaitingSendApproval,
    Sending,
    SentAwaitingReply,
    OutcomeUnknown,
    Stopped,
    Done,
}

impl TaskState {
    /// 送信済み（または送信の途中）とみなす状態か。コメント保存はこの状態へ移さない。
    #[must_use]
    pub fn is_send_side(self) -> bool {
        matches!(
            self,
            Self::Sending | Self::SentAwaitingReply | Self::OutcomeUnknown
        )
    }
}

/// ActionSnapshot の `action_kind`（04 §9 の 4 値案）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ActionKind {
    CustomerSend,
    AgentHandoff,
    ToolRead,
    ToolWrite,
}
