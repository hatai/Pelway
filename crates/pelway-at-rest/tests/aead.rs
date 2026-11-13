//! 分割 AES-256-GCM コンテナのテスト。合成データだけを使う。

use std::collections::HashSet;

use pelway_at_rest::container::{
    CHUNK_SIZE, HEADER_LEN, KEY_ID_LEN, MAGIC, NONCE_LEN, SALT_LEN, TAG_LEN, WRAPPED_KEY_LEN,
};
use pelway_at_rest::{MasterKey, OpenError, open, seal};

const FULL: usize = CHUNK_SIZE + TAG_LEN;
const KEY_ID_AT: usize = MAGIC.len();
const NONCE_AT: usize = KEY_ID_AT + KEY_ID_LEN;
const WRAPPED_AT: usize = NONCE_AT + NONCE_LEN;
const SALT_AT: usize = WRAPPED_AT + WRAPPED_KEY_LEN;

fn synthetic(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i % 251) as u8).collect()
}

fn key_a() -> MasterKey {
    MasterKey::from_bytes([0xA1; KEY_ID_LEN], [0x11; 32])
}

/// 暗号文の i 番目のチャンク（最後以外は FULL バイト）。
fn chunk(sealed: &[u8], i: usize) -> &[u8] {
    let start = HEADER_LEN + i * FULL;
    let end = (start + FULL).min(sealed.len());
    &sealed[start..end]
}

const LENS: [usize; 8] = [
    0,
    1,
    100,
    CHUNK_SIZE - 1,
    CHUNK_SIZE,
    CHUNK_SIZE + 1,
    2 * CHUNK_SIZE,
    3 * CHUNK_SIZE + 5,
];

#[test]
fn roundtrip_various_lengths() {
    let key = key_a();
    for len in LENS {
        let pt = synthetic(len);
        let sealed = seal(&key, &pt).unwrap();
        let chunks = if len == 0 {
            1
        } else {
            len.div_ceil(CHUNK_SIZE)
        };
        assert_eq!(
            sealed.len(),
            HEADER_LEN + len + chunks * TAG_LEN,
            "len={len}"
        );
        assert_eq!(
            open(&key, &sealed).unwrap().as_slice(),
            pt.as_slice(),
            "len={len}"
        );
    }
}

#[test]
fn ciphertext_does_not_contain_plaintext() {
    let key = key_a();
    let pt = b"synthetic-marker-0123456789-synthetic-marker".repeat(10);
    let sealed = seal(&key, &pt).unwrap();
    assert!(!sealed.windows(16).any(|w| w == &pt[..16]));
}

#[test]
fn every_header_byte_is_authenticated() {
    let key = key_a();
    let sealed = seal(&key, &synthetic(300)).unwrap();
    for pos in 0..HEADER_LEN {
        let mut bad = sealed.clone();
        bad[pos] ^= 0x01;
        let expected = if pos < KEY_ID_AT {
            OpenError::Malformed
        } else if pos < NONCE_AT {
            OpenError::UnknownKey
        } else {
            OpenError::Decrypt
        };
        assert_eq!(open(&key, &bad).unwrap_err(), expected, "pos={pos}");
    }
}

#[test]
fn body_tamper_is_detected() {
    let key = key_a();
    let sealed = seal(&key, &synthetic(2 * CHUNK_SIZE + 10)).unwrap();
    for pos in [
        HEADER_LEN,
        HEADER_LEN + FULL - 1,
        HEADER_LEN + FULL + 5,
        HEADER_LEN + 2 * FULL,
        sealed.len() - 1,
    ] {
        let mut bad = sealed.clone();
        bad[pos] ^= 0x01;
        assert_eq!(
            open(&key, &bad).unwrap_err(),
            OpenError::Decrypt,
            "pos={pos}"
        );
    }
}

#[test]
fn truncation_is_detected() {
    let key = key_a();
    // 端数のある長さ：最後のチャンクを丸ごと落とすと、残りの最後は最終フラグなしで暗号化されている。
    let sealed = seal(&key, &synthetic(3 * CHUNK_SIZE + 5)).unwrap();
    assert_eq!(
        open(&key, &sealed[..HEADER_LEN + 3 * FULL]).unwrap_err(),
        OpenError::Decrypt
    );
    assert_eq!(
        open(&key, &sealed[..HEADER_LEN + FULL]).unwrap_err(),
        OpenError::Decrypt
    );
    // 末尾 1 バイトを落とす。
    assert!(open(&key, &sealed[..sealed.len() - 1]).is_err());
    // ヘッダーだけ、ヘッダー未満。
    assert_eq!(
        open(&key, &sealed[..HEADER_LEN]).unwrap_err(),
        OpenError::Malformed
    );
    assert_eq!(
        open(&key, &sealed[..HEADER_LEN - 1]).unwrap_err(),
        OpenError::Malformed
    );

    // ちょうどチャンク境界の長さ：満杯チャンク 1 つを落としても、長さの規則だけでは見抜けない。
    // 最終フラグで検出する。
    let exact = seal(&key, &synthetic(2 * CHUNK_SIZE)).unwrap();
    assert_eq!(exact.len(), HEADER_LEN + 2 * FULL);
    assert_eq!(
        open(&key, &exact[..HEADER_LEN + FULL]).unwrap_err(),
        OpenError::Decrypt
    );
}

#[test]
fn append_is_detected() {
    let key = key_a();
    let sealed = seal(&key, &synthetic(CHUNK_SIZE + 5)).unwrap();
    let mut appended = sealed.clone();
    appended.extend_from_slice(&[0u8; 32]);
    assert!(open(&key, &appended).is_err());
    // 正しいチャンクを末尾に重ねても通らない。
    let mut dup_tail = sealed.clone();
    dup_tail.extend_from_slice(chunk(&sealed, 1));
    assert!(open(&key, &dup_tail).is_err());
}

#[test]
fn reorder_and_duplicate_chunks_are_detected() {
    let key = key_a();
    let sealed = seal(&key, &synthetic(3 * CHUNK_SIZE + 5)).unwrap();
    // 先頭 2 チャンクを入れ替える。
    let mut swapped = sealed[..HEADER_LEN].to_vec();
    swapped.extend_from_slice(chunk(&sealed, 1));
    swapped.extend_from_slice(chunk(&sealed, 0));
    swapped.extend_from_slice(&sealed[HEADER_LEN + 2 * FULL..]);
    assert_eq!(swapped.len(), sealed.len());
    assert_eq!(open(&key, &swapped).unwrap_err(), OpenError::Decrypt);
    // 2 番目のチャンクを 1 番目の複製に置き換える。
    let mut dup = sealed[..HEADER_LEN].to_vec();
    dup.extend_from_slice(chunk(&sealed, 0));
    dup.extend_from_slice(chunk(&sealed, 0));
    dup.extend_from_slice(&sealed[HEADER_LEN + 2 * FULL..]);
    assert_eq!(open(&key, &dup).unwrap_err(), OpenError::Decrypt);
    // 最後のチャンクを途中のチャンクと入れ替える（長さが違うので形式か認証で落ちる）。
    let mut last_moved = sealed[..HEADER_LEN].to_vec();
    last_moved.extend_from_slice(chunk(&sealed, 3));
    last_moved.extend_from_slice(&sealed[HEADER_LEN..HEADER_LEN + 3 * FULL]);
    assert!(open(&key, &last_moved).is_err());
}

#[test]
fn chunk_swap_between_files_is_detected() {
    let key = key_a();
    let pt = synthetic(2 * CHUNK_SIZE + 9);
    let one = seal(&key, &pt).unwrap();
    let two = seal(&key, &pt).unwrap();
    // 同じ鍵・同じ平文・同じ位置のチャンクでも、ファイルが違えば通らない。
    let mut mixed = one[..HEADER_LEN + FULL].to_vec();
    mixed.extend_from_slice(chunk(&two, 1));
    mixed.extend_from_slice(&one[HEADER_LEN + 2 * FULL..]);
    assert_eq!(mixed.len(), one.len());
    assert_eq!(open(&key, &mixed).unwrap_err(), OpenError::Decrypt);
}

#[test]
fn header_swap_between_files_is_detected() {
    let key = key_a();
    let one = seal(&key, &synthetic(500)).unwrap();
    let two = seal(&key, &synthetic(500)).unwrap();
    let mut mixed = two[..HEADER_LEN].to_vec();
    mixed.extend_from_slice(&one[HEADER_LEN..]);
    assert_eq!(open(&key, &mixed).unwrap_err(), OpenError::Decrypt);
    // ファイル鍵の包装だけを別ファイルのものに替えても通らない。
    let mut wrapped_swapped = one.clone();
    wrapped_swapped[NONCE_AT..SALT_AT].copy_from_slice(&two[NONCE_AT..SALT_AT]);
    assert_eq!(
        open(&key, &wrapped_swapped).unwrap_err(),
        OpenError::Decrypt
    );
}

#[test]
fn wrong_key_is_rejected() {
    let sealed = seal(&key_a(), &synthetic(1000)).unwrap();
    // 同じ鍵 ID で鍵の値だけ違う。
    let same_id_other_key = MasterKey::from_bytes([0xA1; KEY_ID_LEN], [0x22; 32]);
    assert_eq!(
        open(&same_id_other_key, &sealed).unwrap_err(),
        OpenError::Decrypt
    );
    // 鍵 ID が違う。
    let other_id = MasterKey::from_bytes([0xB2; KEY_ID_LEN], [0x11; 32]);
    assert_eq!(open(&other_id, &sealed).unwrap_err(), OpenError::UnknownKey);
}

#[test]
fn nonces_salts_and_file_keys_are_fresh_per_seal() {
    let key = key_a();
    let pt = synthetic(64);
    let mut wrap_nonces = HashSet::new();
    let mut salts = HashSet::new();
    let mut wrapped_keys = HashSet::new();
    let mut ciphertexts = HashSet::new();
    for _ in 0..512 {
        let s = seal(&key, &pt).unwrap();
        assert!(wrap_nonces.insert(s[NONCE_AT..WRAPPED_AT].to_vec()));
        assert!(wrapped_keys.insert(s[WRAPPED_AT..SALT_AT].to_vec()));
        assert!(salts.insert(s[SALT_AT..SALT_AT + SALT_LEN].to_vec()));
        assert!(ciphertexts.insert(s[HEADER_LEN..].to_vec()));
    }
}

#[test]
fn generated_keys_differ_and_debug_redacts() {
    let a = MasterKey::generate();
    let b = MasterKey::generate();
    assert_ne!(a.key_id(), b.key_id());
    let k = MasterKey::from_bytes([0x01; KEY_ID_LEN], [0x5A; 32]);
    let dbg = format!("{k:?}");
    assert!(dbg.contains("<redacted>"));
    assert!(!dbg.contains("90, 90")); // 0x5A = 90
}
