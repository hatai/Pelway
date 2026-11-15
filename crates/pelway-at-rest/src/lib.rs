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

pub mod attachments;
pub mod container;

use std::fmt;

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use container::{
    FILE_KEY_LEN, Header, KEY_ID_LEN, NONCE_LEN, SALT_LEN, WRAPPED_KEY_LEN, chunk_nonce,
    plaintext_chunk_lens, split_ciphertext_chunks,
};

const PAYLOAD_KEY_INFO: &[u8] = b"pelway-at-rest/v1 payload";

/// OS の乱数源（getrandom）で埋める。失敗は実用上起きない。
fn os_fill(buf: &mut [u8]) {
    getrandom::fill(buf).expect("OS の乱数源にアクセスできない");
}

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
        os_fill(&mut key_id);
        os_fill(key.as_mut());
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
        Aes256Gcm::new((&*self.key).into())
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

fn payload_key(file_key: &[u8; FILE_KEY_LEN], salt: &[u8; SALT_LEN]) -> Zeroizing<[u8; 32]> {
    let hk = Hkdf::<Sha256>::new(Some(salt), file_key);
    let mut okm = Zeroizing::new([0u8; 32]);
    hk.expand(PAYLOAD_KEY_INFO, okm.as_mut())
        .expect("32 bytes is a valid HKDF-SHA256 output length");
    okm
}

fn payload_cipher(file_key: &[u8; FILE_KEY_LEN], salt: &[u8; SALT_LEN]) -> Aes256Gcm {
    let okm = payload_key(file_key, salt);
    Aes256Gcm::new((&*okm).into())
}

/// 平文をコンテナ形式に暗号化する。ファイル鍵・nonce・salt は毎回 OS の乱数源から作る。
///
/// # Errors
/// AES-GCM が暗号化を拒んだ場合（実用上は起きない）。
pub fn seal(master: &MasterKey, plaintext: &[u8]) -> Result<Vec<u8>, SealError> {
    let mut file_key = Zeroizing::new([0u8; FILE_KEY_LEN]);
    let mut wrap_nonce = [0u8; NONCE_LEN];
    let mut payload_salt = [0u8; SALT_LEN];
    os_fill(file_key.as_mut());
    os_fill(&mut wrap_nonce);
    os_fill(&mut payload_salt);
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
            (&wrap_nonce).into(),
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
                (&nonce).into(),
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
                (&header.wrap_nonce).into(),
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
                    (&nonce).into(),
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

    /// 保存形式の互換ベクトル（`b"pelway-at-rest format compatibility vector"` を
    /// [`fixed_seal`] と同じ材料で暗号化した 168 バイト）。aes-gcm 0.10.3 +
    /// hkdf 0.12.4 + sha2 0.10.9 の組み合わせ（main 8d756fd 時点）で作った。
    /// 新しい版で既存形式のコンテナが読めること・出力が旧版と一致することの保証。
    const AES_GCM_0_10_CONTAINER: &[u8] = &[
        0x70, 0x65, 0x6c, 0x77, 0x61, 0x79, 0x2d, 0x61, 0x74, 0x2d, 0x72, 0x65, 0x73, 0x74, 0x2f,
        0x76, 0x31, 0x0a, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1, 0xa1,
        0xa1, 0xa1, 0xa1, 0xa1, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
        0x22, 0x24, 0xc4, 0x34, 0x7a, 0xf3, 0xfc, 0xac, 0x6c, 0xd6, 0x0c, 0xed, 0x0f, 0x7b, 0x95,
        0xda, 0xe6, 0x2c, 0xa0, 0xfe, 0xd1, 0x59, 0xb2, 0x9f, 0x78, 0x61, 0x7c, 0x39, 0xdb, 0x3a,
        0xd2, 0x56, 0x30, 0x48, 0xe3, 0x05, 0x86, 0xdf, 0x48, 0x0d, 0x33, 0x93, 0x10, 0x46, 0x7c,
        0x11, 0x63, 0x73, 0xb2, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44, 0x44,
        0x44, 0x44, 0x44, 0x44, 0x44, 0x71, 0x6e, 0xf9, 0x86, 0x4e, 0x18, 0x44, 0xed, 0x73, 0x85,
        0x55, 0xb8, 0xe1, 0x26, 0x16, 0xd5, 0x40, 0xbc, 0x67, 0xe9, 0x35, 0xa2, 0xb5, 0x24, 0xd5,
        0x11, 0xbd, 0x05, 0x15, 0x2c, 0x13, 0x1e, 0x54, 0x78, 0x87, 0x92, 0xe6, 0xce, 0xf0, 0xce,
        0xfc, 0xa5, 0xd9, 0x14, 0x0c, 0xbd, 0x77, 0x6f, 0xa6, 0x21, 0x96, 0xa2, 0x83, 0x50, 0x07,
        0x64, 0x77, 0xc5,
    ];
    const AES_GCM_0_10_PLAINTEXT: &[u8] = b"pelway-at-rest format compatibility vector";

    /// HKDF-SHA256 で導出する本文鍵の固定値（ikm = `FILE_KEY`、salt = `SALT`、
    /// info = `PAYLOAD_KEY_INFO`）。hkdf 0.12.4 + sha2 0.10.9 で求めた値で、
    /// 依存の版を上げても導出される鍵が変わらないことの保証。
    const PAYLOAD_KEY_0_12: [u8; 32] = [
        0x43, 0x19, 0x7c, 0xa4, 0x59, 0xfa, 0x5a, 0xaf, 0x53, 0xe0, 0xf7, 0x1d, 0x41, 0x40, 0xf6,
        0x17, 0xb8, 0xd0, 0x21, 0x30, 0x22, 0x64, 0x88, 0xcc, 0xf0, 0xe1, 0xd3, 0x5e, 0x31, 0x50,
        0xb1, 0x07,
    ];

    #[test]
    fn payload_key_matches_hkdf_0_12_fixed_value() {
        let okm = payload_key(&FILE_KEY, &SALT);
        assert_eq!(&*okm, &PAYLOAD_KEY_0_12);
    }

    #[test]
    fn container_format_is_compatible_with_aes_gcm_0_10() {
        // 新版（aes-gcm 0.11 + hkdf 0.13 + sha2 0.11）の出力が旧版と一致する。
        // 鍵導出（HKDF-SHA256）と AES-256-GCM が決定論的なので、材料が同じなら
        // バイト列も同じになる。
        assert_eq!(fixed_seal(AES_GCM_0_10_PLAINTEXT), AES_GCM_0_10_CONTAINER);
        // 旧版で作ったコンテナを新版で読める。
        let master = MasterKey::from_bytes([0xA1; KEY_ID_LEN], [0x11; 32]);
        assert_eq!(
            open(&master, AES_GCM_0_10_CONTAINER).unwrap().as_slice(),
            AES_GCM_0_10_PLAINTEXT
        );
    }

    fn fixed_seal(pt: &[u8]) -> Vec<u8> {
        let master = MasterKey::from_bytes([0xA1; KEY_ID_LEN], [0x11; 32]);
        seal_with_material(&master, pt, &FILE_KEY, [0x22; NONCE_LEN], SALT).unwrap()
    }

    fn try_chunk(chunk: &[u8], nonce: [u8; NONCE_LEN], aad: &[u8]) -> bool {
        payload_cipher(&FILE_KEY, &SALT)
            .decrypt((&nonce).into(), Payload { msg: chunk, aad })
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
