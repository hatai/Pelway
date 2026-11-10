//! チャンネルのメンバー。`member_kind` は必須（デモ決定 vi）。

use crate::ids::MemberId;

/// メンバーの種類。デモでは人と AI の 2 値だけを扱う（リード決定 2026-11-09）。
///
/// 必須項目で、既定値は持たない（欠けた JSON は受け付けない）。
/// UI はこの値で AI 表示（03 §2 「Bot は AI 表示を持つ」）を出す。
///
/// TODO(spec 04 §2): 仕様の `actors.kind` は `human/bot/external_agent/external_tool` の 4 値。
/// デモの `human|ai` から仕様の 4 値への対応は、デモ後に決める（ここでは決めない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum MemberKind {
    Human,
    Ai,
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
