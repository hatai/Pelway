//! C2SP signed-note（<https://c2sp.org/signed-note>）の署名形式の最小実装。
//!
//! Pelway が使うのは Ed25519（署名種別 `0x01`）1 種類だけ。Merkle 木・witness は
//! 対象外。形式の適合は仕様書のテストベクトル（`example.com/foo`）で確かめる。
//!
//! ```text
//! This is an example message.
//!
//! — example.com/foo Uw2QOkn8srV1yJGh2VYRlL1Tnagv1YEq6TfXppzi2ONncAlTgK7Ztg1ERYNZXsYjOBH3mFXmRKuwHjG1Yu72IneyaQM=
//! ```
//!
//! - 本文（note text）は改行で終わり、空行を 1 つ挟んで署名行が続く。署名は本文全体
//!   （最終改行を含み、区切りの空行を含まない）に対して計算する。
//! - 署名行は「`— 鍵名 base64(鍵ID 4バイト || Ed25519署名 64バイト)`」。鍵IDは
//!   `SHA-256(鍵名 || 0x0A || 0x01 || 公開鍵32バイト)` の先頭 4 バイト。
//! - 検証は、既知の鍵と (鍵名, 鍵ID) が一致する署名を必ず検証し、1 つでも失敗したら
//!   note 全体を拒否、既知の鍵の署名が 1 つも無ければ拒否、知らない鍵の署名は無視。
//! - signed note は UTF-8 で、U+0020 未満の制御文字は改行だけを許す。

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use sha2::{Digest, Sha256};

/// Ed25519 の署名種別識別子（signed-note 仕様の割り当て）。
pub const SIGNATURE_TYPE_ED25519: u8 = 0x01;
/// 署名に並べる鍵 ID の長さ。
pub const KEY_ID_LEN: usize = 4;
/// Ed25519 署名の長さ（RFC 8032）。
pub const SIGNATURE_LEN: usize = 64;
/// 署名行の接頭辞（em dash U+2014 と空白）。
const SIG_PREFIX: &str = "— ";

/// note の形式・署名の検証の失敗。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteError {
    /// 形式が守られていない（制御文字・署名行の構文・base64・鍵名・鍵の値）。
    Format,
    /// 既知の鍵に一致する署名が 1 つでも検証に失敗した（note 全体を拒否する）。
    InvalidSignature,
    /// 既知の鍵に一致する署名が 1 つも無い。
    NoKnownKey,
}

/// 鍵 ID = `SHA-256(鍵名 || 0x0A || 0x01 || 公開鍵)` の先頭 4 バイト（仕様の推奨）。
#[must_use]
pub fn compute_key_id(key_name: &str, public_key: &[u8; 32]) -> [u8; KEY_ID_LEN] {
    let mut h = Sha256::new();
    h.update(key_name.as_bytes());
    h.update([0x0A, SIGNATURE_TYPE_ED25519]);
    h.update(public_key);
    let out = h.finalize();
    let mut id = [0u8; KEY_ID_LEN];
    id.copy_from_slice(&out[..KEY_ID_LEN]);
    id
}

/// 鍵名の要件（仕様）: 空でない、Unicode 空白を含まない、`+` を含まない。
fn validate_key_name(key_name: &str) -> Result<(), NoteError> {
    if key_name.is_empty() || key_name.chars().any(char::is_whitespace) || key_name.contains('+') {
        return Err(NoteError::Format);
    }
    Ok(())
}

/// 署名鍵。Ed25519 秘密鍵は破棄時に 0 埋めされる（ed25519-dalek の zeroize 機能）。
/// DB の管理者が触れない場所で保持する（ADR 0007）。
#[derive(Clone)]
pub struct NoteSigner {
    name: String,
    key: SigningKey,
}

impl NoteSigner {
    /// OS の乱数源から新しい鍵を作る。
    #[must_use]
    pub fn generate(key_name: &str) -> Self {
        Self {
            name: key_name.to_owned(),
            key: SigningKey::generate(&mut rand_core::OsRng),
        }
    }

    /// 既存の秘密鍵（32 バイト）から作る。テストでは合成の固定値を使う。
    #[must_use]
    pub fn from_bytes(key_name: &str, secret: [u8; 32]) -> Self {
        Self {
            name: key_name.to_owned(),
            key: SigningKey::from_bytes(&secret),
        }
    }

    /// 鍵名。
    #[must_use]
    pub fn key_name(&self) -> &str {
        &self.name
    }

    /// 鍵 ID。
    #[must_use]
    pub fn key_id(&self) -> [u8; KEY_ID_LEN] {
        compute_key_id(&self.name, &self.key.verifying_key().to_bytes())
    }

    /// 対応する検証鍵。
    #[must_use]
    pub fn verifying_key(&self) -> NoteVerifier {
        NoteVerifier::from_verifying_key(&self.name, self.key.verifying_key())
    }

    /// 本文に署名し、署名付き note（本文 + 空行 + 署名行）を返す。
    /// `text` は改行で終わっていること。
    #[must_use]
    pub fn sign_note_text(&self, text: &str) -> String {
        let signature = self.key.sign(text.as_bytes());
        let mut payload = Vec::with_capacity(KEY_ID_LEN + SIGNATURE_LEN);
        payload.extend_from_slice(&self.key_id());
        payload.extend_from_slice(&signature.to_bytes());
        format!(
            "{text}\n{SIG_PREFIX}{} {}\n",
            self.name,
            BASE64.encode(payload)
        )
    }
}

/// 検証鍵（鍵名・鍵 ID・Ed25519 公開鍵）。検証側の設定に置く。
#[derive(Clone)]
pub struct NoteVerifier {
    name: String,
    key_id: [u8; KEY_ID_LEN],
    public_key: VerifyingKey,
}

impl NoteVerifier {
    fn from_verifying_key(key_name: &str, public_key: VerifyingKey) -> Self {
        Self {
            key_id: compute_key_id(key_name, &public_key.to_bytes()),
            name: key_name.to_owned(),
            public_key,
        }
    }

    /// 公開鍵（32 バイト）から作る。鍵名は空・Unicode 空白・`+` を含まないこと。
    ///
    /// # Errors
    /// 鍵名が要件を満たさないか、公開鍵が曲線上の点でないとき [`NoteError::Format`]。
    pub fn from_public_key(key_name: &str, public_key: &[u8; 32]) -> Result<Self, NoteError> {
        validate_key_name(key_name)?;
        let public_key = VerifyingKey::from_bytes(public_key).map_err(|_| NoteError::Format)?;
        Ok(Self::from_verifying_key(key_name, public_key))
    }

    /// 鍵名。
    #[must_use]
    pub fn key_name(&self) -> &str {
        &self.name
    }

    /// 鍵 ID。
    #[must_use]
    pub fn key_id(&self) -> [u8; KEY_ID_LEN] {
        self.key_id
    }
}

/// 署名付き note を検証し、検証済みの本文（最終改行を含む）を返す。
///
/// # Errors
/// 形式の誤り・既知の鍵の署名の失敗・既知の鍵の署名が無いこと（[`NoteError`]）。
pub fn verify_note(signed_note: &str, verifiers: &[NoteVerifier]) -> Result<String, NoteError> {
    // 仕様: signed note は UTF-8 で、U+0020 未満の制御文字は改行だけを許す。
    if signed_note.bytes().any(|b| b < 0x20 && b != b'\n') {
        return Err(NoteError::Format);
    }
    // 本文と署名行は「最後の空行」で区切る。本文は区切りの 1 つ目の改行まで（最終
    // 改行を含む）。署名行は区切りの 2 つ目の改行の後ろに 1 行以上続く。
    let Some(sep) = signed_note.rfind("\n\n") else {
        return Err(NoteError::Format);
    };
    let text = &signed_note[..sep + 1];
    let signatures = &signed_note[sep + 2..];
    if text.is_empty() || !signatures.ends_with('\n') {
        return Err(NoteError::Format);
    }
    let mut verified = 0usize;
    for line in signatures
        .strip_suffix('\n')
        .unwrap_or(signatures)
        .split('\n')
    {
        let Some(rest) = line.strip_prefix(SIG_PREFIX) else {
            return Err(NoteError::Format);
        };
        let Some((name, encoded)) = rest.rsplit_once(' ') else {
            return Err(NoteError::Format);
        };
        validate_key_name(name)?;
        let decoded = BASE64.decode(encoded).map_err(|_| NoteError::Format)?;
        if decoded.len() < KEY_ID_LEN {
            return Err(NoteError::Format);
        }
        let (key_id, signature) = decoded.split_at(KEY_ID_LEN);
        for v in verifiers {
            // 知らない鍵の署名は無視する。既知の鍵と (鍵名, 鍵ID) が一致する署名は
            // 必ず検証し、失敗したら note 全体を拒否する（仕様の要件）。
            if v.key_name() == name && v.key_id().as_slice() == key_id {
                let Ok(bytes) = <&[u8; SIGNATURE_LEN]>::try_from(signature) else {
                    return Err(NoteError::Format);
                };
                v.public_key
                    .verify(text.as_bytes(), &Signature::from_bytes(bytes))
                    .map_err(|_| NoteError::InvalidSignature)?;
                verified += 1;
            }
        }
    }
    if verified == 0 {
        return Err(NoteError::NoKnownKey);
    }
    Ok(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// C2SP signed-note 仕様書のテストベクトル（example.com/foo）。
    const SPEC_KEY_B64: &str = "AekyeRrm56hApGFkyQR4ZCbV54Id2LKaANYcrnKv3U2k";
    const SPEC_NOTE: &str = "This is an example message.\n\n\u{2014} example.com/foo Uw2QOkn8srV1yJGh2VYRlL1Tnagv1YEq6TfXppzi2ONncAlTgK7Ztg1ERYNZXsYjOBH3mFXmRKuwHjG1Yu72IneyaQM=\n";

    /// 仕様の公開鍵（base64 の先頭 1 バイトは署名種別 0x01）。
    fn spec_verifier() -> NoteVerifier {
        let material = BASE64.decode(SPEC_KEY_B64).unwrap();
        let public_key: [u8; 32] = material[1..].try_into().unwrap();
        NoteVerifier::from_public_key("example.com/foo", &public_key).unwrap()
    }

    #[test]
    fn spec_vector_verifies() {
        let v = spec_verifier();
        // 仕様の鍵 ID（530d903a）が導出できる。
        assert_eq!(v.key_id(), [0x53, 0x0d, 0x90, 0x3a]);
        let text = verify_note(SPEC_NOTE, &[v]).unwrap();
        assert_eq!(text, "This is an example message.\n");
        // 本文を書き換えた署名は検証に失敗する（note 全体を拒否する）。
        let bad = SPEC_NOTE.replace("example message", "tampered message");
        assert_eq!(
            verify_note(&bad, &[spec_verifier()]),
            Err(NoteError::InvalidSignature)
        );
    }

    #[test]
    fn sign_and_verify_roundtrip_with_unknown_key_rejected() {
        let signer = NoteSigner::from_bytes("pelway.test/audit", [7; 32]);
        let text = "line1\nline2\n";
        let signed = signer.sign_note_text(text);
        // 鍵名が同じでも鍵が違えば (鍵名, 鍵ID) が一致しない。
        let other = NoteSigner::from_bytes("pelway.test/audit", [9; 32]).verifying_key();
        assert_eq!(verify_note(&signed, &[other]), Err(NoteError::NoKnownKey));
        assert_eq!(
            verify_note(&signed, &[signer.verifying_key()]).unwrap(),
            text
        );
    }
}
