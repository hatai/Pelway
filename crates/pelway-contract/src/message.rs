//! チャンネルとメッセージ。04 §2・§3 の `messages` と `POST/GET /v1/channels/{id}/messages`。

use crate::ids::{ChannelId, ClientMsgId, MemberId, MessageId, WorkspaceId};
use crate::member::{ChannelMember, MemberKind};

/// チャンネル一覧 API のパス（workspace ごとの集合。一覧は `GET`）。
/// デモでは workspace の下に置く（判断待ち一覧 `DECISIONS_PATH` と同じ形）。
pub const CHANNELS_PATH: &str = "/v1/workspaces/{workspace_id}/channels";

/// チャンネル情報 API のパス（`GET`。channel を直接指す経路）。
pub const CHANNEL_PATH: &str = "/v1/channels/{channel_id}";

/// メッセージの投稿・取得 API のパス（04 §3 の
/// `POST/GET /v1/channels/{id}/messages`。投稿は `POST`、取得は `GET`）。
pub const CHANNEL_MESSAGES_PATH: &str = "/v1/channels/{channel_id}/messages";

/// [`CHANNELS_PATH`] の `{workspace_id}` を差し替えて、要求に使う実際の経路を作る。
/// サーバー（`pelway-http`）の経路判定もテストもこの関数で経路を作り、
/// 文字列リテラルで書き写さない（Issue #91）。
#[must_use]
pub fn channels_path(workspace_id: &WorkspaceId) -> String {
    CHANNELS_PATH.replace("{workspace_id}", workspace_id.as_str())
}

/// [`CHANNEL_PATH`] の `{channel_id}` を差し替えて、要求に使う実際の経路を作る。
#[must_use]
pub fn channel_path(channel_id: &ChannelId) -> String {
    CHANNEL_PATH.replace("{channel_id}", channel_id.as_str())
}

/// [`CHANNEL_MESSAGES_PATH`] の `{channel_id}` を差し替えて、要求に使う実際の
/// 経路を作る。
#[must_use]
pub fn channel_messages_path(channel_id: &ChannelId) -> String {
    CHANNEL_MESSAGES_PATH.replace("{channel_id}", channel_id.as_str())
}

/// チャンネル。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Channel {
    pub workspace_id: WorkspaceId,
    pub channel_id: ChannelId,
    pub name: String,
    pub members: Vec<ChannelMember>,
}

/// 保存済みメッセージ。`seq` はチャンネル内の発言順（04 §4）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Message {
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub seq: u64,
    pub author_id: MemberId,
    /// 必須。発言者が人か AI かを UI が表示に使う。
    pub member_kind: MemberKind,
    pub client_msg_id: ClientMsgId,
    pub body: String,
    pub thread_root: Option<MessageId>,
    /// UTC の RFC 3339 文字列（06 §1：保存時刻は UTC）。
    pub created_at: String,
}

/// `POST /v1/channels/{channel_id}/messages` の入力。
/// 同じ `client_msg_id` の再送は同じ結果、本文が違えば 409（04 §2）。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct PostMessageRequest {
    pub client_msg_id: ClientMsgId,
    pub body: String,
    pub thread_root: Option<MessageId>,
}

/// `GET /v1/channels/{channel_id}/messages` のクエリ。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ListMessagesQuery {
    pub before_seq: Option<u64>,
    pub after_seq: Option<u64>,
    pub limit: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_helpers_build_expected_paths() {
        assert_eq!(
            channels_path(&"ws-1".into()),
            "/v1/workspaces/ws-1/channels"
        );
        assert_eq!(channel_path(&"ch-1".into()), "/v1/channels/ch-1");
        assert_eq!(
            channel_messages_path(&"ch-1".into()),
            "/v1/channels/ch-1/messages"
        );
        assert!(channels_path(&"ws-1".into()).starts_with("/v1/workspaces/"));
    }
}
