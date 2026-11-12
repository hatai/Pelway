//! 監査行のハッシュ鎖（04 の `audit_events` の下書き）。
//!
//! 鎖は workspace 内で線形につなぐ: `hash = SHA-256(領域分離 || prev_hash || 行の内容)`。
//! 最初の行の `prev_hash` は 32 バイトの 0（[`GENESIS`]）。行の内容の符号化は一意に
//! 決まる長さ前置き（u64 ビッグエディアン）にするため、行の差し替え・並べ替え・
//! workspace をまたぐ移植は歩査で検出できる。行の内容に秘密・prompt 全文は含めない
//! （04 の設計）。管理者が全行を書き換えて再計算した場合の検出は [`crate::checkpoint`]
//! （保管済みチェックポイントとの照合）が担う。

use sha2::{Digest, Sha256};

/// 領域分離文字列（他の用途の SHA-256 と衝突させない）。
const DOMAIN: &[u8] = b"pelway-audit-chain/v1";
/// 起点の `prev_hash`（32 バイトの 0）。
pub const GENESIS: [u8; 32] = [0; 32];

/// 監査行の内容（04 の `audit_events` 行の下書き）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// 鎖の単位（workspace 内の鎖。行の移植を検出するためハッシュに入れる）。
    pub workspace_id: String,
    /// 日時（エポックからのミリ秒）。
    pub at_ms: u64,
    /// 実行者。
    pub actor: String,
    /// 対象。
    pub subject: String,
    /// 操作の種類。
    pub kind: String,
    /// 結果。
    pub result: String,
    /// 操作の中身の snapshot hash（#87 の承認の中身の hash と同じ考え方）。
    pub snapshot_hash: [u8; 32],
    /// 承認に使った policy の版。
    pub policy_version: u64,
}

/// 保存済みの監査行（内容と鎖の値）。「DB から読んだ行」に対応する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRow {
    pub event: AuditEvent,
    /// 前の行の `hash`（最初の行は [`GENESIS`]）。
    pub prev_hash: [u8; 32],
    /// この行のハッシュ。
    pub hash: [u8; 32],
}

/// 鎖の歩査の失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    /// `prev_hash` が前の行の `hash` と一致しない（行の削除・差し替え・並べ替え）。
    BrokenLink { row: usize },
    /// 保存済みの `hash` が内容から計算した値と一致しない（内容の書き換え）。
    HashMismatch { row: usize },
}

/// 行のハッシュ。内容と `prev_hash` だけから計算する。
#[must_use]
pub fn row_hash(event: &AuditEvent, prev_hash: &[u8; 32]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update([0x00]);
    put_str(&mut h, &event.workspace_id);
    put_u64(&mut h, event.at_ms);
    put_str(&mut h, &event.actor);
    put_str(&mut h, &event.subject);
    put_str(&mut h, &event.kind);
    put_str(&mut h, &event.result);
    h.update(event.snapshot_hash);
    put_u64(&mut h, event.policy_version);
    h.update(prev_hash);
    h.finalize().into()
}

/// 内容の列から鎖を組み立てる（各行の `prev_hash` と `hash` を計算して詰める）。
/// 監査行の INSERT 側の代わり。
#[must_use]
pub fn build_chain(events: impl IntoIterator<Item = AuditEvent>) -> Vec<StoredRow> {
    let mut rows = Vec::new();
    let mut prev = GENESIS;
    for event in events {
        let hash = row_hash(&event, &prev);
        rows.push(StoredRow {
            event,
            prev_hash: prev,
            hash,
        });
        prev = hash;
    }
    rows
}

/// 先頭から全行を歩査する。各行で `prev_hash` の付き合わせと `hash` の再計算をし、
/// 各行までの最新ハッシュ（`heads[i]` は `i + 1` 行目までの最新ハッシュ）を返す。
///
/// # Errors
/// `prev_hash` の付き合わせか `hash` の再計算が失敗したとき（[`ChainError`]）。
pub fn walk(rows: &[StoredRow]) -> Result<Vec<[u8; 32]>, ChainError> {
    let mut prev = GENESIS;
    let mut heads = Vec::with_capacity(rows.len());
    for (i, row) in rows.iter().enumerate() {
        if row.prev_hash != prev {
            return Err(ChainError::BrokenLink { row: i });
        }
        let hash = row_hash(&row.event, &prev);
        if row.hash != hash {
            return Err(ChainError::HashMismatch { row: i });
        }
        prev = hash;
        heads.push(hash);
    }
    Ok(heads)
}

/// 文字列を長さ前置き（u64 BE の長さ + UTF-8 バイト）でハッシュへ入れる。
fn put_str(h: &mut Sha256, s: &str) {
    put_u64(h, s.len() as u64);
    h.update(s.as_bytes());
}

/// u64 をビッグエディアンでハッシュへ入れる。
fn put_u64(h: &mut Sha256, v: u64) {
    h.update(v.to_be_bytes());
}
