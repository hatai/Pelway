//! チャンネルのメンバー。`member_kind` は必須（デモ決定 vi）。

use crate::ids::MemberId;

/// メンバーの種類。仕様 04 §2 の `actors.kind` の 4 値
/// （human / bot / external_agent / external_tool）。
///
/// 必須項目で、既定値は持たない（欠けた JSON は受け付けない）。
/// UI はこの値で AI 表示（03 §2 「Bot は AI 表示を持つ」）を出す。
/// `bot` は Pelway 内の AI メンバー、`external_agent` / `external_tool` は
/// 外部の主体（04 §2: 単独では権限を持たず委任元を参照する）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum MemberKind {
    Human,
    Bot,
    ExternalAgent,
    ExternalTool,
}

impl MemberKind {
    /// ワイヤ上の値（serde の `snake_case` タグと同じ。テストが一致を確かめる）。
    /// ヘッダーなど素の文字列との変換に使い、対応は [`MemberKind::parse`] とペア。
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Bot => "bot",
            Self::ExternalAgent => "external_agent",
            Self::ExternalTool => "external_tool",
        }
    }

    /// ワイヤ上の値から作る。未知の値（旧・デモの `ai` を含む）は `None`。
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "human" => Some(Self::Human),
            "bot" => Some(Self::Bot),
            "external_agent" => Some(Self::ExternalAgent),
            "external_tool" => Some(Self::ExternalTool),
            _ => None,
        }
    }
}

/// チャンネル参加者。
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ChannelMember {
    pub member_id: MemberId,
    pub display_name: String,
    /// 必須。`#[serde(default)]` を付けないこと。
    pub member_kind: MemberKind,
}
