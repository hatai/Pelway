//! チェックポイントの保管先（DB の管理者が触れない別権限の保存場所の参照実装）。
//!
//! 追記専用の 1 ファイルに、署名済みチェックポイント 1 通を 1 エントリとして長さ
//! 前置き（u32 ビッグエディアン）で追記する。検証側は全エントリから最新の 1 通を
//! 取って DB と照合する。追記しかしないのは、保管先の書き換え・削除で検出を無効化
//! できないようにするため。実運用ではこのファイルを DB 管理者から権限と到達の両面
//! で分離した保管先（別ホストの volume・別資格情報のオブジェクトストレージなど）に
//! 置く（ADR 0007）。

use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

/// 追記専用チェックポイントログ。
#[derive(Debug, Clone)]
pub struct CheckpointLog {
    path: PathBuf,
}

impl CheckpointLog {
    /// 保管先のパス。ファイルは [`CheckpointLog::append`] のときに作る。
    #[must_use]
    pub fn open(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// 署名済みチェックポイントを 1 通追記する。
    ///
    /// # Errors
    /// 書き込みの失敗（[`io::Error`]）。
    pub fn append(&self, entry: &str) -> io::Result<()> {
        let bytes = entry.as_bytes();
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(&(bytes.len() as u32).to_be_bytes())?;
        f.write_all(bytes)
    }

    /// 全エントリ（追記順）。ファイルが無ければ空。
    ///
    /// # Errors
    /// 読み取りの失敗、または長さ前置きが切れている（追記の途中で壊れた）とき。
    pub fn entries(&self) -> io::Result<Vec<String>> {
        let mut buf = Vec::new();
        match fs::File::open(&self.path) {
            Ok(mut f) => {
                f.read_to_end(&mut buf)?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        }
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < buf.len() {
            if buf.len() - i < 4 {
                return Err(corrupt(&self.path));
            }
            let len = u32::from_be_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]) as usize;
            i += 4;
            if buf.len() - i < len {
                return Err(corrupt(&self.path));
            }
            let entry = std::str::from_utf8(&buf[i..i + len]).map_err(|_| corrupt(&self.path))?;
            out.push(entry.to_owned());
            i += len;
        }
        Ok(out)
    }
}

/// 長さ前置きが切れているときのエラー。
fn corrupt(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("checkpoint log is corrupt: {}", path.display()),
    )
}
