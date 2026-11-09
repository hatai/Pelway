//! チャンネルとメッセージ。04 §2・§3 の `messages` と `POST/GET /v1/channels/{id}/messages`。

use crate::ids::{ChannelId, ClientMsgId, MemberId, MessageId, WorkspaceId};
use crate::member::{ChannelMember, MemberKind};

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
