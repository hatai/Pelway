//! 実物の時計とキー（Issue 333。`testkit` の固定版を本番用に置き換えるもの）。
//!
//! - [`SystemClock`]: [`Clock`] の実装。UNIX エポックからの実時刻（ミリ秒）。
//!   サーバーがメッセージに付ける `created_at` と同じ基準（`pelway-http` の
//!   `now_ms` と同じ計算）。
//! - [`UuidKeySource`]: [`KeySource`] の実装。UUID v4 を冪等キーとして出す
//!   （36 文字。`IdempotencyKey` の書式 16〜128 文字の `[A-Za-z0-9_-]` を満たす）。
//!
//! テストは引き続き `testkit` の `FixedClock` / `FixedKeySource` を使う。

use std::time::{SystemTime, UNIX_EPOCH};

use pelway_client_core::approval::{Clock, KeySource};
use pelway_contract::ids::IdempotencyKey;

/// 実時刻の [`Clock`]。
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> u64 {
        u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis()),
        )
        .unwrap_or(u64::MAX)
    }
}

/// UUID v4 を出す [`KeySource`]。
#[derive(Debug, Clone, Copy, Default)]
pub struct UuidKeySource;

impl UuidKeySource {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl KeySource for UuidKeySource {
    fn fresh_key(&mut self) -> IdempotencyKey {
        // UUID v4 はハイフン込み 36 文字で、冪等キーの書式を必ず満たす。
        IdempotencyKey::new(uuid::Uuid::new_v4().to_string())
            .expect("UUID v4 は冪等キーの書式に収まる")
    }
}

/// 投稿用の送信 ID（`client_msg_id`。冪等キーと同じ UUID v4 を使い回せる形）。
#[must_use]
pub fn fresh_client_msg_id() -> pelway_contract::ids::ClientMsgId {
    pelway_contract::ids::ClientMsgId::from(uuid::Uuid::new_v4().to_string().as_str())
}
