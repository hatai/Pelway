//! A2 の同期で往復する型の見本。
//!
//! 実際の契約型は `pelway-contract` にある。ここでは型生成ツール
//! （ts-rs / specta）を試すのに足る範囲の型を置いて、生成結果を見えるように
//! する。_validation に複雑な型は置かない（実際に入る型に近い形にする）。

use serde::{Deserialize, Serialize};

/// チャンネルの識別子。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS, specta::Type)]
#[ts(export_to = "bindings/")]
pub struct ChannelId(pub String);

/// メッセージの識別子。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS, specta::Type)]
#[ts(export_to = "bindings/")]
pub struct MessageId(pub String);

/// 通知の緊急度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS, specta::Type)]
#[ts(export_to = "bindings/")]
pub enum NoticeLevel {
    Info,
    Warning,
    Critical,
}

/// メッセージの本文の種別。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS, specta::Type)]
#[ts(export_to = "bindings/")]
pub enum MessageBody {
    /// ただの本文。
    Text {
        /// 本文。日本語の文が入ることが多い。
        body: String,
    },
    /// 通知。
    Notice {
        /// 通知の本文。
        body: String,
        /// 緊急度。
        level: NoticeLevel,
    },
    /// ファイル。
    File {
        /// ファイル名。
        name: String,
        /// サイズ（バイト）。
        ///
        /// specta は i64/u64 を JS の number にすると精度が落ちることを理由に
        /// 書き出しを拒む。field ごとに number として出すことを明示する。
        #[specta(type = specta_typescript::Number)]
        size_bytes: u64,
    },
}

/// 同期するメッセージ 1 件。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ts_rs::TS, specta::Type)]
#[ts(export_to = "bindings/")]
pub struct Message {
    /// メッセージの識別子。
    pub message_id: MessageId,
    /// 所属するチャンネル。
    pub channel_id: ChannelId,
    /// 投稿者の識別子。
    pub author_id: String,
    /// 本文。
    pub body: MessageBody,
    /// 投稿時刻（ミリ秒）。
    ///
    /// specta は i64/u64 を JS の number にすると精度が落ちることを理由に
    /// 書き出しを拒む。field ごとに number として出すことを明示する。
    #[specta(type = specta_typescript::Number)]
    pub sent_at_ms: i64,
    /// 編集されたか。
    pub edited: bool,
    /// メンションされた相手の識別子。
    pub mentions: Vec<String>,
}
