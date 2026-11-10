//! 保存時の分割 AES-256-GCM コンテナ（下書き）。デモではデータはメモリだけに置き、
//! ディスクへ書き出すスナップショットだけをこの形式にする想定。
//!
//! 形式は [`container`] を参照（Pelway 独自の形式）。
//!
//! - 保存鍵（[`MasterKey`]）でファイルごとの乱数ファイル鍵を包装する。
//! - ファイル鍵から HKDF-SHA256 で本文鍵を作り、64 KiB ごとに AES-256-GCM で暗号化する。
//!   各チャンクの nonce は連番と最終フラグ、AAD はヘッダー全体。
//! - 復号の失敗は理由を区別せず [`OpenError::Decrypt`] にまとめる（形式・鍵 ID の誤りは別）。
//!
//! TODO(U05): 保存鍵の保管場所・ローテーション・復旧手順は未決。ここでは呼び出し側が
//! 鍵を渡すだけで、鍵をファイルや環境変数から読む処理は持たない。
//! 乱数は OS の乱数源（getrandom）だけを使い、ネットワークには触れない。

#![forbid(unsafe_code)]

pub mod container;

use std::fmt;

use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::{Aead, KeyInit, OsRng, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use container::{
    FILE_KEY_LEN, Header, KEY_ID_LEN, NONCE_LEN, SALT_LEN, WRAPPED_KEY_LEN, chunk_nonce,
    plaintext_chunk_lens, split_ciphertext_chunks,
};

const PAYLOAD_KEY_INFO: &[u8] = b"pelway-at-rest/v1 payload";

/// 保存鍵。値は Debug にも出さず、破棄時に 0 で上書きする。
pub struct MasterKey {
    key_id: [u8; KEY_ID_LEN],
    key: Zeroizing<[u8; 32]>,
}

impl MasterKey {
    /// OS の乱数源から新しい鍵と鍵 ID を作る。
    #[must_use]
    pub fn generate() -> Self {
        let mut key_id = [0u8; KEY_ID_LEN];
        let mut key = Zeroizing::new([0u8; 32]);
        OsRng.fill_bytes(&mut key_id);
        OsRng.fill_bytes(key.as_mut());
        Self { key_id, key }
    }

    /// 既存の鍵から作る（テストでは合成の固定値を使う）。
    #[must_use]
    pub fn from_bytes(key_id: [u8; KEY_ID_LEN], key: [u8; 32]) -> Self {
        Self {
            key_id,
            key: Zeroizing::new(key),
        }
    }

    #[must_use]
    pub fn key_id(&self) -> [u8; KEY_ID_LEN] {
        self.key_id
    }

    fn cipher(&self) -> Aes256Gcm {
        Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(self.key.as_ref()))
    }
}

impl fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MasterKey")
            .field("key_id", &self.key_id)
            .field("key", &"<redacted>")
            .finish()
    }
}

/// 暗号化の失敗（入力が AES-GCM の上限を超えた場合など）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealError;

/// 復号の失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenError {
    /// 形式が合わない（長さ・magic・チャンク長）。
    Malformed,
    /// 別の保存鍵で作られたコンテナ。
    UnknownKey,
    /// 認証に失敗（改ざん・切り詰め・並べ替え・鍵違い）。理由はあえて区別しない。
    Decrypt,
}

fn payload_cipher(file_key: &[u8; FILE_KEY_LEN], salt: &[u8; SALT_LEN]) -> Aes256Gcm {
    let hk = Hkdf::<Sha256>::new(Some(salt), file_key);
    let mut okm = Zeroizing::new([0u8; 32]);
    hk.expand(PAYLOAD_KEY_INFO, okm.as_mut())
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(okm.as_ref()))
}

/// 平文をコンテナ形式に暗号化する。ファイル鍵・nonce・salt は毎回 OS の乱数源から作る。
///
/// # Errors
/// AES-GCM が暗号化を拒んだ場合（実用上は起きない）。
pub fn seal(master: &MasterKey, plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
    let mut file_key = Zeroizing::new([0u8; FILE_KEY_LEN]);
    let mut wrap_nonce = [0u8; NONCE_LEN];
    let mut payload_salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(file_key.as_mut());
    OsRng.fill_bytes(&mut wrap_nonce);
    OsRng.fill_bytes(&mut payload_salt);
    seal_with_material(master, plaintext, &file_key, wrap_nonce, payload_salt)
}

fn seal_with_material(
    master: &MasterKey,
    plaintext: &[u8],
    file_key: &[u8; FILE_KEY_LEN],
    wrap_nonce: [u8; NONCE_LEN],
    payload_salt: [u8; SALT_LEN],
) -> Result<Vec<u8>, SealError> {
    let key_id = master.key_id;
    let wrapped = master
        .cipher()
        .encrypt(
            Nonce::from_slice(&wrap_nonce),
            Payload {
                msg: file_key.as_ref(),
                aad: &Header::wrap_aad(&key_id),
            },
        )
        .map_err(|_| SealError)?;
    let wrapped_key: [u8; WRAPPED_KEY_LEN] = wrapped.try_into().map_err(|_| SealError)?;

    let header = Header {
        key_id,
        wrap_nonce,
        wrapped_key,
        payload_salt,
    };
    let header_bytes = header.encode();
    let mut out = header_bytes.clone();

    let cipher = payload_cipher(file_key, &payload_salt);
    let lens = plaintext_chunk_lens(plaintext.len());
    let mut offset = 0;
    for (i, len) in lens.iter().enumerate() {
        let last = i + 1 == lens.len();
        let nonce = chunk_nonce(i as u64, last);
        let ct = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &plaintext[offset..offset + len],
                    aad: &header_bytes,
                },
            )
            .map_err(|_| SealError)?;
        out.extend_from_slice(&ct);
        offset += len;
    }
    Ok(out)
}

/// コンテナを復号する。成功したときだけ平文を返す（途中までの平文は返さない）。
///
/// # Errors
/// [`OpenError`] のいずれか。
pub fn open(master: &MasterKey, data: &[u8]) -> Result<Zeroizing<Vec<u8>>, OpenError> {
    let (header, body) = Header::parse(data).map_err(|_| OpenError::Malformed)?;
    if header.key_id != master.key_id {
        return Err(OpenError::UnknownKey);
    }
    let header_bytes = header.encode();
    let file_key = Zeroizing::new(
        master
            .cipher()
            .decrypt(
                Nonce::from_slice(&header.wrap_nonce),
                Payload {
                    msg: &header.wrapped_key,
                    aad: &Header::wrap_aad(&header.key_id),
                },
            )
            .map_err(|_| OpenError::Decrypt)?,
    );
    let file_key: Zeroizing<[u8; FILE_KEY_LEN]> = Zeroizing::new(
        file_key
            .as_slice()
            .try_into()
            .map_err(|_| OpenError::Decrypt)?,
    );

    let cipher = payload_cipher(&file_key, &header.payload_salt);
    let chunks = split_ciphertext_chunks(body).map_err(|_| OpenError::Malformed)?;
    let mut plaintext = Zeroizing::new(Vec::with_capacity(body.len()));
    for (i, (chunk, last)) in chunks.into_iter().enumerate() {
        let nonce = chunk_nonce(i as u64, last);
        let pt = Zeroizing::new(
            cipher
                .decrypt(
                    Nonce::from_slice(&nonce),
                    Payload {
                        msg: chunk,
                        aad: &header_bytes,
                    },
                )
                .map_err(|_| OpenError::Decrypt)?,
        );
        plaintext.extend_from_slice(&pt);
    }
    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    //! 内部の材料（ファイル鍵・salt）を固定して、チャンクの nonce と AAD を直接確かめる。
    use super::*;
    use container::{HEADER_LEN, TAG_LEN};

    const FILE_KEY: [u8; FILE_KEY_LEN] = [0x33; FILE_KEY_LEN];
    const SALT: [u8; SALT_LEN] = [0x44; SALT_LEN];

    fn fixed_seal(pt: &[u8]) -> Vec<u8> {
        let master = MasterKey::from_bytes([0xA1; KEY_ID_LEN], [0x11; 32]);
        seal_with_material(&master, pt, &FILE_KEY, [0x22; NONCE_LEN], SALT).unwrap()
    }

    fn try_chunk(chunk: &[u8], nonce: [u8; NONCE_LEN], aad: &[u8]) -> bool {
        payload_cipher(&FILE_KEY, &SALT)
            .decrypt(Nonce::from_slice(&nonce), Payload { msg: chunk, aad })
            .is_ok()
    }

    #[test]
    fn chunk_is_bound_to_header_aad() {
        let sealed = fixed_seal(b"synthetic");
        let (header, body) = sealed.split_at(HEADER_LEN);
        assert!(try_chunk(body, chunk_nonce(0, true), header));
        // AAD 無し・ヘッダーを 1 バイト変えた AAD では通らない。
        assert!(!try_chunk(body, chunk_nonce(0, true), &[]));
        let mut other = header.to_vec();
        other[HEADER_LEN - 1] ^= 0x01;
        assert!(!try_chunk(body, chunk_nonce(0, true), &other));
    }

    #[test]
    fn chunk_is_bound_to_counter_and_final_flag() {
        let sealed = fixed_seal(&vec![7u8; container::CHUNK_SIZE + 3]);
        let (header, body) = sealed.split_at(HEADER_LEN);
        let first = &body[..container::CHUNK_SIZE + TAG_LEN];
        let second = &body[container::CHUNK_SIZE + TAG_LEN..];
        assert!(try_chunk(first, chunk_nonce(0, false), header));
        assert!(try_chunk(second, chunk_nonce(1, true), header));
        // 最終フラグ・連番のどちらが違っても通らない。
        assert!(!try_chunk(first, chunk_nonce(0, true), header));
        assert!(!try_chunk(first, chunk_nonce(1, false), header));
        assert!(!try_chunk(second, chunk_nonce(1, false), header));
        assert!(!try_chunk(second, chunk_nonce(0, true), header));
    }
}
