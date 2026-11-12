//! A2 の同期で往復する型の見本。実際の契約型は `pelway-contract` にある。型生成
//! ツール（ts-rs / specta）を試すのに足る範囲の型を置いてある。

use serde::{Deserialize, Serialize};

/// 通知の緊急度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export_to = "bindings/")]
pub enum NoticeLevel {
    Info,
    Warning,
    Critical,
}

/// メッセージの本文の種別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export_to = "bindings/")]
pub enum MessageBody {
    /// ただの本文。
    Text { body: String },
    /// 通知。
    Notice { body: String, level: NoticeLevel },
    /// ファイル。
    File {
        name: String,
        /// サイズ（バイト）。
        size_bytes: u64,
    },
}

/// 同期するメッセージ 1 件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS)]
#[ts(export_to = "bindings/")]
pub struct Message {
    /// メッセージの識別子。
    pub message_id: String,
    /// 所属するチャンネル。
    pub channel_id: String,
    /// 投稿者の識別子。
    pub author_id: String,
    /// 本文。
    pub body: MessageBody,
    /// 投稿時刻（ミリ秒）。
    pub sent_at_ms: i64,
    /// 編集されたか。
    pub edited: bool,
    /// メンションされた相手の識別子。
    pub mentions: Vec<String>,
}
