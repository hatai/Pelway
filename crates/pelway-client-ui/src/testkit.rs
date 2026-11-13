//! テストと骨組みの表示のための、決まった値を返す `KeySource` と `Clock`。
//!
//! 段階Bで本物の実装（UUID 生成・実時間）に置き換える。通信は書かないため、
//! これらは画面を動かすための最小の部品。

use pelway_client_core::approval::{Clock, KeySource};
use pelway_client_core::contract::IdempotencyKey;

/// 決まった形の連番キーを返す `KeySource`。
///
/// キーは `fixed-key-000001` の形（16 文字ちょうど。`IdempotencyKey::MIN_LEN` 内）。
pub struct FixedKeySource {
    next: u32,
}

impl FixedKeySource {
    /// 1 から順に番号を振る。
    #[must_use]
    pub fn new() -> Self {
        Self { next: 0 }
    }
}

impl Default for FixedKeySource {
    fn default() -> Self {
        Self::new()
    }
}

impl KeySource for FixedKeySource {
    fn fresh_key(&mut self) -> IdempotencyKey {
        self.next += 1;
        IdempotencyKey::new(format!("fixed-key-{:06}", self.next))
            .expect("テスト用の固定キーは常に書式内（16 文字・英数字と -）")
    }
}

/// 決まった時刻を返す `Clock`。時間は進まない。
pub struct FixedClock {
    ms: u64,
}

impl FixedClock {
    /// `now_ms()` が常に返す時刻（ミリ秒）。
    #[must_use]
    pub fn new(ms: u64) -> Self {
        Self { ms }
    }
}

impl Clock for FixedClock {
    fn now_ms(&self) -> u64 {
        self.ms
    }
}
