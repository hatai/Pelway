//! 分割 AES-256-GCM コンテナの形式（暗号処理は `lib.rs` 側）。
//!
//! ```text
//! magic           18 B  "pelway-at-rest/v1\n"
//! key_id          16 B  包装に使った保存鍵の ID（鍵そのものではない）
//! wrap_nonce      12 B  ファイル鍵の包装に使う乱数 nonce
//! wrapped_key     48 B  AES-256-GCM(保存鍵, ファイル鍵 32 B) + tag 16 B、AAD = magic || key_id
//! payload_salt    16 B  HKDF-SHA256 の salt（ファイル鍵 → 本文鍵）
//! chunks          ...   本文 64 KiB ごとの AES-256-GCM 暗号文 + tag 16 B、AAD = ヘッダー全体
//! ```
//!
//! - チャンクの nonce：11 B のビッグエンディアン連番 + 1 B の最終フラグ（最後のチャンクだけ
//!   0x01）。連番で並べ替え・入れ替え・重複を、最終フラグで切り詰めと末尾への追記を検出する。
//! - チャンクの AAD はヘッダー全体（[`HEADER_LEN`] バイト）。ヘッダーの 1 バイトでも変われば、
//!   どのチャンクの認証も通らない。
//! - ファイルごとに新しい乱数のファイル鍵と salt を使うため、同じ (本文鍵, nonce) の組は
//!   再利用されない。
//!
//! Pelway 独自の形式で、他のファイル形式との互換は持たない。

/// 形式の識別子。
pub const MAGIC: &[u8; 18] = b"pelway-at-rest/v1\n";
pub const KEY_ID_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const TAG_LEN: usize = 16;
pub const FILE_KEY_LEN: usize = 32;
pub const WRAPPED_KEY_LEN: usize = FILE_KEY_LEN + TAG_LEN;
pub const SALT_LEN: usize = 16;
/// 本文チャンクの平文サイズ（64 KiB）。
pub const CHUNK_SIZE: usize = 64 * 1024;
pub const HEADER_LEN: usize = MAGIC.len() + KEY_ID_LEN + NONCE_LEN + WRAPPED_KEY_LEN + SALT_LEN;

/// 形式の誤り。暗号の失敗とは区別しないで外へ出す（lib 側で 1 種類にまとめる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerError {
    TooShort,
    BadMagic,
}

/// ヘッダー。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub key_id: [u8; KEY_ID_LEN],
    pub wrap_nonce: [u8; NONCE_LEN],
    pub wrapped_key: [u8; WRAPPED_KEY_LEN],
    pub payload_salt: [u8; SALT_LEN],
}

impl Header {
    /// ファイル鍵の包装に使う AAD（magic || key_id）。
    #[must_use]
    pub fn wrap_aad(key_id: &[u8; KEY_ID_LEN]) -> Vec<u8> {
        let mut aad = Vec::with_capacity(MAGIC.len() + KEY_ID_LEN);
        aad.extend_from_slice(MAGIC);
        aad.extend_from_slice(key_id);
        aad
    }

    /// ヘッダーのバイト列。チャンクの AAD にもこの値を使う。
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&self.key_id);
        out.extend_from_slice(&self.wrap_nonce);
        out.extend_from_slice(&self.wrapped_key);
        out.extend_from_slice(&self.payload_salt);
        out
    }

    /// 先頭のヘッダーを読み、残り（チャンク列）を返す。
    ///
    /// # Errors
    /// 長さ不足または magic 不一致。
    pub fn parse(input: &[u8]) -> Result<(Self, &[u8]), ContainerError> {
        if input.len() < HEADER_LEN {
            return Err(ContainerError::TooShort);
        }
        let (magic, rest) = input.split_at(MAGIC.len());
        if magic != MAGIC {
            return Err(ContainerError::BadMagic);
        }
        let (key_id, rest) = rest.split_at(KEY_ID_LEN);
        let (wrap_nonce, rest) = rest.split_at(NONCE_LEN);
        let (wrapped_key, rest) = rest.split_at(WRAPPED_KEY_LEN);
        let (payload_salt, rest) = rest.split_at(SALT_LEN);
        let header = Self {
            key_id: key_id.try_into().map_err(|_| ContainerError::TooShort)?,
            wrap_nonce: wrap_nonce
                .try_into()
                .map_err(|_| ContainerError::TooShort)?,
            wrapped_key: wrapped_key
                .try_into()
                .map_err(|_| ContainerError::TooShort)?,
            payload_salt: payload_salt
                .try_into()
                .map_err(|_| ContainerError::TooShort)?,
        };
        Ok((header, rest))
    }
}

/// チャンク `counter` の nonce。11 B ビッグエンディアン連番 + 最終フラグ 1 B。
/// u64 は 8 B なので 11 B の連番に必ず収まる（上位 3 B は 0）。
#[must_use]
pub fn chunk_nonce(counter: u64, last: bool) -> [u8; NONCE_LEN] {
    let mut nonce = [0u8; NONCE_LEN];
    nonce[3..11].copy_from_slice(&counter.to_be_bytes());
    nonce[11] = u8::from(last);
    nonce
}

/// 平文の長さからチャンクの平文長の列を作る。空の平文は長さ 0 の最終チャンク 1 つ。
#[must_use]
pub fn plaintext_chunk_lens(len: usize) -> Vec<usize> {
    if len == 0 {
        return vec![0];
    }
    let full = len / CHUNK_SIZE;
    let rem = len % CHUNK_SIZE;
    let mut lens = vec![CHUNK_SIZE; full];
    if rem != 0 {
        lens.push(rem);
    }
    lens
}

/// 暗号文のチャンク列を (チャンク, 最終か) に分ける。
///
/// 最終チャンク以外は必ず `CHUNK_SIZE + TAG_LEN` バイト。最終チャンクは
/// `TAG_LEN..=CHUNK_SIZE + TAG_LEN` バイト。ただし最終チャンクの平文が 0 バイトなのは
/// チャンクが 1 つだけ（空の平文）の場合に限る。
///
/// # Errors
/// 長さが規則に合わない場合 [`ContainerError::TooShort`]。
pub fn split_ciphertext_chunks(body: &[u8]) -> Result<Vec<(&[u8], bool)>, ContainerError> {
    const FULL: usize = CHUNK_SIZE + TAG_LEN;
    if body.len() < TAG_LEN {
        return Err(ContainerError::TooShort);
    }
    let mut chunks = Vec::new();
    let mut rest = body;
    while rest.len() > FULL {
        let (chunk, tail) = rest.split_at(FULL);
        chunks.push((chunk, false));
        rest = tail;
    }
    // 残りが最終チャンク。
    if rest.len() < TAG_LEN || (rest.len() == TAG_LEN && !chunks.is_empty()) {
        return Err(ContainerError::TooShort);
    }
    chunks.push((rest, true));
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn sample_header() -> Header {
        Header {
            key_id: [1; KEY_ID_LEN],
            wrap_nonce: [2; NONCE_LEN],
            wrapped_key: [3; WRAPPED_KEY_LEN],
            payload_salt: [4; SALT_LEN],
        }
    }

    #[test]
    fn header_roundtrip() {
        let h = sample_header();
        let mut bytes = h.encode();
        assert_eq!(bytes.len(), HEADER_LEN);
        bytes.extend_from_slice(b"body");
        let (parsed, rest) = Header::parse(&bytes).unwrap();
        assert_eq!(parsed, h);
        assert_eq!(rest, b"body");
    }

    #[test]
    fn header_rejects_short_input_and_bad_magic() {
        let bytes = sample_header().encode();
        assert_eq!(
            Header::parse(&bytes[..HEADER_LEN - 1]),
            Err(ContainerError::TooShort)
        );
        let mut bad = bytes.clone();
        bad[0] ^= 0x01;
        assert_eq!(Header::parse(&bad), Err(ContainerError::BadMagic));
    }

    #[test]
    fn chunk_nonce_encodes_counter_and_final_flag() {
        let mut seen = HashSet::new();
        for counter in 0..10_000u64 {
            for last in [false, true] {
                assert!(seen.insert(chunk_nonce(counter, last)));
            }
        }
        assert_eq!(
            chunk_nonce(0x0102_0304_0506_0708, true),
            [0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 1]
        );
        assert_eq!(chunk_nonce(0, false), [0; NONCE_LEN]);
        assert_eq!(chunk_nonce(u64::MAX, false)[3..11], [0xff; 8]);
        assert_eq!(chunk_nonce(u64::MAX, false)[..3], [0; 3]);
    }

    #[test]
    fn chunk_lens_cover_edges() {
        assert_eq!(plaintext_chunk_lens(0), vec![0]);
        assert_eq!(plaintext_chunk_lens(1), vec![1]);
        assert_eq!(plaintext_chunk_lens(CHUNK_SIZE), vec![CHUNK_SIZE]);
        assert_eq!(plaintext_chunk_lens(CHUNK_SIZE + 1), vec![CHUNK_SIZE, 1]);
        assert_eq!(
            plaintext_chunk_lens(2 * CHUNK_SIZE),
            vec![CHUNK_SIZE, CHUNK_SIZE]
        );
    }

    #[test]
    fn split_matches_chunk_lens() {
        for len in [
            0,
            1,
            CHUNK_SIZE - 1,
            CHUNK_SIZE,
            CHUNK_SIZE + 1,
            3 * CHUNK_SIZE + 7,
        ] {
            let lens = plaintext_chunk_lens(len);
            let total: usize = lens.iter().map(|l| l + TAG_LEN).sum();
            let body = vec![0u8; total];
            let chunks = split_ciphertext_chunks(&body).unwrap();
            assert_eq!(chunks.len(), lens.len(), "len={len}");
            for (i, ((chunk, last), plain)) in chunks.iter().zip(&lens).enumerate() {
                assert_eq!(chunk.len(), plain + TAG_LEN);
                assert_eq!(*last, i + 1 == lens.len());
            }
        }
    }

    #[test]
    fn split_rejects_bad_lengths() {
        assert_eq!(split_ciphertext_chunks(&[]), Err(ContainerError::TooShort));
        assert_eq!(
            split_ciphertext_chunks(&[0; TAG_LEN - 1]),
            Err(ContainerError::TooShort)
        );
        // 満杯チャンクの後ろに tag だけの最終チャンク（空）は不正。
        let body = vec![0u8; CHUNK_SIZE + TAG_LEN + TAG_LEN];
        assert_eq!(
            split_ciphertext_chunks(&body),
            Err(ContainerError::TooShort)
        );
        // 満杯チャンクの後ろに tag 未満の端数も不正。
        let body = vec![0u8; CHUNK_SIZE + TAG_LEN + 3];
        assert_eq!(
            split_ciphertext_chunks(&body),
            Err(ContainerError::TooShort)
        );
    }
}
