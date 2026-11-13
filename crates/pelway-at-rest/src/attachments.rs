//! 添付の保存と読み出し（#103・02「添付の保存」: `object_store` 経由）。
//!
//! 保存先は **ローカルのまま**、S3 互換の interface（[`object_store::ObjectStore`]）で
//! 扱う。実装は [`object_store::local::LocalFileSystem`]（ローカルファイル実装）で、
//! 将来 S3 互換サーバー（rustfs など）へ移るときはこの実装の差し替えだけで済む
//! （呼び出し側は [`object_store::ObjectStore`] trait を見ているため）。
//!
//! - キーは `workspaces/<workspace_id>/attachments/<name>`。`workspace_id` と `name` は
//!   それぞれ **安全な 1 パス成分**（空・`.`・`..`・`/`・`\`・NUL・制御文字を含まない）
//!   だけを受け付ける。path traversal はここで断る。
//! - 同期 API。[`LocalFileSystem`] の操作は tokio ランタイムが無い環境ではその場で
//!   実行される（`maybe_spawn_blocking` の退化動作）ため、[`futures::executor::block_on`]
//!   で駆動する。ランタイムを持ち込まない。
//! - ネットワークには触れない（ローカル実装のみ）。

#![forbid(unsafe_code)]

use std::path::Path;

use object_store::local::LocalFileSystem;
use object_store::path::Path as StorePath;
use object_store::{ObjectStoreExt, PutPayload};

/// 添付の保存・読み出しの失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentError {
    /// `workspace_id` または `name` が安全な 1 パス成分でない。
    InvalidSegment { segment: String },
    /// 読み出し対象が無い。
    NotFound,
    /// 保存・読み出しの下位失敗（ローカルファイルの I/O エラーなど）。
    Io(String),
}

impl std::fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSegment { segment } => {
                write!(f, "1 パス成分でない: {segment:?}")
            }
            Self::NotFound => write!(f, "添付が無い"),
            Self::Io(message) => write!(f, "添付の下位失敗: {message}"),
        }
    }
}

impl std::error::Error for AttachmentError {}

/// 添付の保存先（`object_store` のローカルファイル実装）。
///
/// `root` は添付を置くディレクトリ。この下に `workspaces/<workspace_id>/attachments/<name>`
/// として並ぶ。
#[derive(Debug)]
pub struct AttachmentStore {
    store: LocalFileSystem,
}

impl AttachmentStore {
    /// ローカルの `root` の下に保存する store を作る。
    ///
    /// # Errors
    /// `root` が存在しない・正規化できないとき
    /// （[`LocalFileSystem::new_with_prefix`] は canonicalize して存在を確かめる）。
    pub fn local(root: &Path) -> Result<Self, AttachmentError> {
        let store = LocalFileSystem::new_with_prefix(root)
            .map_err(|e| AttachmentError::Io(e.to_string()))?;
        Ok(Self { store })
    }

    /// 添付 `name` の中身を `workspace` の下に保存する（上書きする）。
    ///
    /// # Errors
    /// `workspace`・`name` が安全な 1 パス成分でない、または下位の I/O 失敗。
    pub fn put(&self, workspace: &str, name: &str, bytes: &[u8]) -> Result<(), AttachmentError> {
        let location = location(workspace, name)?;
        futures::executor::block_on(self.store.put(&location, PutPayload::from(bytes.to_vec())))
            .map(|_| ())
            .map_err(map_store_error)
    }

    /// 添付 `name` の中身を `workspace` の下から読み出す。
    ///
    /// # Errors
    /// [`AttachmentError::NotFound`]（対象が無い）、セグメント検査の失敗、下位の I/O 失敗。
    pub fn get(&self, workspace: &str, name: &str) -> Result<Vec<u8>, AttachmentError> {
        let location = location(workspace, name)?;
        let result =
            futures::executor::block_on(self.store.get(&location)).map_err(map_store_error)?;
        futures::executor::block_on(result.bytes())
            .map(|bytes| bytes.to_vec())
            .map_err(map_store_error)
    }
}

/// オブジェクトキー `workspaces/<workspace>/attachments/<name>` を作る。
fn location(workspace: &str, name: &str) -> Result<StorePath, AttachmentError> {
    Ok(StorePath::from_iter([
        "workspaces",
        safe_segment(workspace)?,
        "attachments",
        safe_segment(name)?,
    ]))
}

/// 安全な 1 パス成分か確かめる（path traversal の拒否）。
///
/// 通すのは「空でなく、`.`・`..` と一致せず、`/`・`\`・NUL・制御文字を含まない」
/// 文字列だけ。object_store の [`StorePath`] も独自に検査するが、ここで先に断ることで
/// 「どの文字が拒否されたか」を呼び出し側が扱える形（[`AttachmentError::InvalidSegment`]）で
/// 返す。
fn safe_segment(segment: &str) -> Result<&str, AttachmentError> {
    let ok = !segment.is_empty()
        && segment != "."
        && segment != ".."
        && !segment.contains('/')
        && !segment.contains('\\')
        && !segment.contains('\0')
        && !segment.chars().any(char::is_control);
    if ok {
        Ok(segment)
    } else {
        Err(AttachmentError::InvalidSegment {
            segment: segment.to_owned(),
        })
    }
}

/// 下位の `object_store` エラーをこの crate のエラーへ対応付ける。
fn map_store_error(error: object_store::Error) -> AttachmentError {
    match error {
        object_store::Error::NotFound { .. } => AttachmentError::NotFound,
        other => AttachmentError::Io(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// テストごとに一意のルートを作る（並行テストで衝突しない）。
    fn temp_root(label: &str) -> PathBuf {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pelway-at-rest-attachments-{label}-{}-{n}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    #[test]
    fn put_then_get_round_trips_bytes() {
        let root = temp_root("roundtrip");
        let store = AttachmentStore::local(&root).expect("local store");

        let bytes = "合成データの添付の中身\n".as_bytes().to_vec();
        store
            .put("ws-synthetic", "sha256:att-1", &bytes)
            .expect("put");
        assert_eq!(
            store.get("ws-synthetic", "sha256:att-1").expect("get"),
            bytes
        );

        // 別の添付は混ざらない。
        store
            .put(
                "ws-synthetic",
                "sha256:att-2",
                b"second".to_vec().as_slice(),
            )
            .expect("put 2");
        assert_eq!(
            store.get("ws-synthetic", "sha256:att-2").expect("get 2"),
            b"second"
        );
        assert_eq!(
            store
                .get("ws-synthetic", "sha256:att-1")
                .expect("get 1 のまま"),
            bytes
        );
    }

    #[test]
    fn get_missing_attachment_is_not_found() {
        let root = temp_root("missing");
        let store = AttachmentStore::local(&root).expect("local store");
        assert_eq!(
            store.get("ws-synthetic", "sha256:absent"),
            Err(AttachmentError::NotFound)
        );
    }

    #[test]
    fn same_name_in_different_workspaces_does_not_collide() {
        let root = temp_root("workspaces");
        let store = AttachmentStore::local(&root).expect("local store");
        store
            .put("ws-a", "sha256:att-1", b"for ws-a")
            .expect("put a");
        store
            .put("ws-b", "sha256:att-1", b"for ws-b")
            .expect("put b");
        assert_eq!(
            store.get("ws-a", "sha256:att-1").expect("get a"),
            b"for ws-a"
        );
        assert_eq!(
            store.get("ws-b", "sha256:att-1").expect("get b"),
            b"for ws-b"
        );
    }

    #[test]
    fn put_writes_the_file_under_the_local_root() {
        let root = temp_root("on-disk");
        let store = AttachmentStore::local(&root).expect("local store");
        store
            .put("ws-synthetic", "sha256:att-1", b"bytes on disk")
            .expect("put");
        // object_store のローカル実装が root の下へ実際のファイルとして書くこと。
        let file = root
            .join("workspaces")
            .join("ws-synthetic")
            .join("attachments")
            .join("sha256:att-1");
        assert_eq!(
            std::fs::read(&file).expect("ファイルが root の下にある"),
            b"bytes on disk"
        );
    }

    #[test]
    fn unsafe_segments_are_rejected_and_nothing_is_written() {
        let root = temp_root("unsafe");
        let store = AttachmentStore::local(&root).expect("local store");
        let bad_names = [
            "",
            ".",
            "..",
            "a/b",
            "../escape",
            "..\\escape",
            "back\\slash",
            "nul\0",
            "ctrl\u{1}",
        ];
        for name in bad_names {
            assert_eq!(
                store.put("ws-synthetic", name, b"x"),
                Err(AttachmentError::InvalidSegment {
                    segment: name.to_owned()
                }),
                "name={name:?}"
            );
            assert_eq!(
                store.get("ws-synthetic", name),
                Err(AttachmentError::InvalidSegment {
                    segment: name.to_owned()
                }),
                "name={name:?}"
            );
        }
        for workspace in ["..", "a/b", ""] {
            assert_eq!(
                store.put(workspace, "sha256:att-1", b"x"),
                Err(AttachmentError::InvalidSegment {
                    segment: workspace.to_owned()
                }),
                "workspace={workspace:?}"
            );
        }
        // root の外・下のどちらにも書いていない。
        assert!(
            !root.join("workspaces").exists(),
            "拒否したキーでディレクトリを作ってはいけない"
        );
    }

    #[test]
    fn local_requires_existing_root() {
        // 存在しない root は new_with_prefix の canonicalize が断る（Io に対応付く）。
        let absent = std::env::temp_dir().join(format!(
            "pelway-at-rest-attachments-absent-{}",
            std::process::id()
        ));
        let err = AttachmentStore::local(&absent).unwrap_err();
        assert!(matches!(err, AttachmentError::Io(_)), "{err:?}");
    }
}
