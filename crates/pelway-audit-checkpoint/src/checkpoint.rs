//! チェックポイント（鎖の最新ハッシュと行数への署名）。
//!
//! 本文は tlog-checkpoint（<https://c2sp.org/tlog-checkpoint>）にならった 3 行:
//!
//! ```text
//! pelway.example/w/ws-1     ← origin（workspace ごとの鎖の識別子）
//! 5                         ← 行数（10進・先頭の 0 を認めない）
//! 3q2+7vL…=                 ← その行数までの鎖の最新ハッシュ（base64・32バイト）
//!
//! — pelway.example/w/ws-1 Uw2Q…=
//! ```
//!
//! tlog-checkpoint との違いは、3 行目が RFC 6962 の Merkle 根ではなく線形鎖の最新
//! ハッシュであること（04 の `prev_hash` 鎖をそのまま使う。#92 の調査・ADR 0007）。
//! 拡張行は使わない（tlog-checkpoint でも非推奨）。

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

use crate::chain::{self, StoredRow};
use crate::note::{self, NoteSigner, NoteVerifier};

/// チェックポイントの本体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpoint {
    /// workspace ごとの鎖の識別子（origin）。検証時に期待値と付き合わせる。
    pub origin: String,
    /// チェックポイント時点の行数。
    pub size: u64,
    /// `size` 行までの鎖の最新ハッシュ。
    pub hash: [u8; 32],
}

/// チェックポイント本文（3 行）の形式の誤り。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointError {
    /// 行数が 3 行でない、origin が空・空白・`+` を含む（拡張行は認めない）。
    Lines,
    /// 2 行目（行数）が 10 進でない、先頭の 0 がある、u64 を超える。
    Size,
    /// 3 行目（最新ハッシュ）が base64 の 32 バイトでない。
    Hash,
}

/// 検証の失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyError {
    /// 署名付き note としての形式・署名の失敗。
    Note(note::NoteError),
    /// 本文（3 行）の形式の誤り。
    Checkpoint(CheckpointError),
    /// 期待した origin と一致しない（別 workspace 向けのチェックポイント）。
    OriginMismatch,
    /// 行数がチェックポイント時点より少ない（切り詰め・削除）。
    Truncated { have: usize, checkpoint_size: u64 },
    /// 鎖の歩査の失敗（`prev_hash`・`hash` の付き合わせ）。
    Chain(chain::ChainError),
    /// 鎖の最新ハッシュがチェックポイントと一致しない（書き換え・再計算）。
    HashMismatch {
        checkpoint_size: u64,
        checkpoint: [u8; 32],
        recomputed: [u8; 32],
    },
}

impl Checkpoint {
    /// 3 行の本文（最終改行を含む）。
    #[must_use]
    pub fn note_text(&self) -> String {
        format!(
            "{}\n{}\n{}\n",
            self.origin,
            self.size,
            BASE64.encode(self.hash)
        )
    }
}

/// 検証済み note の本文からチェックポイントを読む。
///
/// # Errors
/// 3 行・行数・ハッシュの形式の誤り（[`CheckpointError`]）。
pub fn parse_note_text(text: &str) -> Result<Checkpoint, CheckpointError> {
    let body = text.strip_suffix('\n').ok_or(CheckpointError::Lines)?;
    let mut lines = body.split('\n');
    let (origin, size_line, hash_line) = match (lines.next(), lines.next(), lines.next()) {
        (Some(a), Some(b), Some(c)) if lines.next().is_none() => (a, b, c),
        _ => return Err(CheckpointError::Lines),
    };
    // origin は鍵名と同じ要件にする（署名行の鍵名に使うことを想定）。
    if origin.is_empty() || origin.chars().any(char::is_whitespace) || origin.contains('+') {
        return Err(CheckpointError::Lines);
    }
    let size = parse_size(size_line).ok_or(CheckpointError::Size)?;
    let decoded = BASE64
        .decode(hash_line)
        .map_err(|_| CheckpointError::Hash)?;
    let hash: [u8; 32] = decoded
        .as_slice()
        .try_into()
        .map_err(|_| CheckpointError::Hash)?;
    Ok(Checkpoint {
        origin: origin.to_owned(),
        size,
        hash,
    })
}

/// 行数行: 10 進のみ、先頭の 0 は認めない（tlog-checkpoint と同じ規則）。
fn parse_size(line: &str) -> Option<u64> {
    if line.is_empty() || (line.len() > 1 && line.starts_with('0')) {
        return None;
    }
    if !line.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    line.parse().ok()
}

/// 行列の現在の状態（行数と最新ハッシュ）に署名してチェックポイントを作る。
/// 鎖が内部で一貫していなければ署かない。
///
/// # Errors
/// 鎖の歩査の失敗（[`chain::ChainError`]）。
pub fn sign_checkpoint(
    rows: &[StoredRow],
    origin: &str,
    signer: &NoteSigner,
) -> Result<String, chain::ChainError> {
    let heads = chain::walk(rows)?;
    let cp = Checkpoint {
        origin: origin.to_owned(),
        size: rows.len() as u64,
        hash: heads.last().copied().unwrap_or(chain::GENESIS),
    };
    Ok(signer.sign_note_text(&cp.note_text()))
}

/// 保管済みの署名チェックポイントと DB から読んだ行列を照合する。
///
/// 順序: (1) 署名の検証 (2) 本文の形式 (3) origin の一致 (4) 鎖の歩査
/// (5) 行数（切り詰めでないか）と最新ハッシュ（書き換えられていないか）の照合。
///
/// # Errors
/// 上のどれかの段階の失敗（[`VerifyError`]）。最新チェックポイントより後の追記分は
/// ここでは担保しない（次のチェックポイントで担保する。ADR 0007）。
pub fn verify_checkpoint(
    rows: &[StoredRow],
    signed_note: &str,
    verifiers: &[NoteVerifier],
    expected_origin: &str,
) -> Result<Checkpoint, VerifyError> {
    let text = note::verify_note(signed_note, verifiers).map_err(VerifyError::Note)?;
    let cp = parse_note_text(&text).map_err(VerifyError::Checkpoint)?;
    if cp.origin != expected_origin {
        return Err(VerifyError::OriginMismatch);
    }
    let heads = chain::walk(rows).map_err(VerifyError::Chain)?;
    if (rows.len() as u64) < cp.size {
        return Err(VerifyError::Truncated {
            have: rows.len(),
            checkpoint_size: cp.size,
        });
    }
    let recomputed = if cp.size == 0 {
        chain::GENESIS
    } else {
        heads[(cp.size - 1) as usize]
    };
    if recomputed != cp.hash {
        return Err(VerifyError::HashMismatch {
            checkpoint_size: cp.size,
            checkpoint: cp.hash,
            recomputed,
        });
    }
    Ok(cp)
}
