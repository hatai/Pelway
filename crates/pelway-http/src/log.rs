//! 構造化ログ（本文を残さないログの仕組み、GDPR-07）。
//!
//! - 出来事は [`Event`] の列挙型で表す。各出来事の項目は **仮名化した ID・種別・
//!   数値（状態コード・バイト数・所要ミリ秒など）・結果コードだけ** で、
//!   本文・コメント・宛先・エラーの文言・要求の中身を入れる欄は持たせない
//!   （許可リスト方式）。[`std::fmt::Debug`] で要求や本文の型を書き出さない。
//! - 出力は JSON Lines（1 行 1 JSON）。時刻 `ts_ms`（UNIX エポックからのミリ秒）は
//!   サーバーが記録時に付ける。
//! - 出力先は [`LogSink`] で差し替えられる。テスト用のメモリ出力 [`MemorySink`] と
//!   ファイル出力 [`FileSink`]（置き場所は `target/demo-logs/` の下だけ）。
//! - ID の仮名化は [`pelway_contract::pseudonym::pseudonymize`]。salt は
//!   [`Logger`] が持ち、ログにも [`Debug`] にも出さない。
//! - 外部 exporter は無い（S17 と `10-domestic-only` の log/audit/crash 行:
//!   exporter なし・秘密除去）。保持期限（R09/R10/R11）はこのデモでは未実装。

use std::fmt;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pelway_contract::member::MemberKind;
use pelway_contract::pseudonym::pseudonymize;
use serde::Serialize;

/// ファイル出力の置き場所（プロセスの CWD からの相対）。cargo 経由の実行では
/// ワークスペースルートの `target/demo-logs/` になる。
pub const LOG_DIR: &str = "target/demo-logs";

/// ログの 1 行（JSON）を追記する出力先。改行は実装が付ける。
pub trait LogSink: Send + Sync {
    fn append(&self, line: &str);
}

/// 出力先が無い（既定）。[`crate::serve`] などログを要求しない起動はこれを使う。
pub struct NullSink;

impl LogSink for NullSink {
    fn append(&self, _line: &str) {}
}

/// テスト用のメモリ出力。
#[derive(Default)]
pub struct MemorySink {
    lines: Mutex<Vec<String>>,
}

impl MemorySink {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// 書き込まれた行のコピー。
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().expect("memory sink lock").clone()
    }
}

impl LogSink for MemorySink {
    fn append(&self, line: &str) {
        self.lines
            .lock()
            .expect("memory sink lock")
            .push(line.to_owned());
    }
}

/// ファイル出力。置き場所は [`LOG_DIR`] の下だけ。
///
/// ファイル名はディレクトリを含まない 1 成分だけ（`/`・`\`・`..` を拒否）。
/// 最初の追記時にディレクトリを作り、ファイルを作り直す（プロセスごとに
/// 新しいファイル）。書き込みは best-effort で、失敗しても接続処理は止めない。
pub struct FileSink {
    dir: PathBuf,
    filename: String,
    file: Mutex<Option<std::fs::File>>,
}

impl fmt::Debug for FileSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileSink")
            .field("path", &self.path())
            .finish_non_exhaustive()
    }
}

impl FileSink {
    /// `target/demo-logs/<filename>` への追記出力を作る。
    ///
    /// # Errors
    /// ファイル名が空・ディレクトリを含む・`.` や `..` のとき
    /// （[`std::io::ErrorKind::InvalidInput`]）。
    pub fn create(filename: &str) -> std::io::Result<Self> {
        if filename.is_empty()
            || filename.contains(['/', '\\'])
            || Path::new(filename).components().count() != 1
            || !matches!(
                Path::new(filename).components().next(),
                Some(Component::Normal(_))
            )
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "log file name must be a single path component under target/demo-logs",
            ));
        }
        Ok(Self {
            dir: PathBuf::from(LOG_DIR),
            filename: filename.to_owned(),
            file: Mutex::new(None),
        })
    }

    /// 出力先のパス（常に `target/demo-logs/<filename>`）。
    #[must_use]
    pub fn path(&self) -> PathBuf {
        self.dir.join(&self.filename)
    }

    /// 最初の追記時にディレクトリを作り、ファイルを作り直す（プロセスごとに新しい
    /// ファイル）。ハンドルは [`FileSink`] が 1 つだけ持つため `append` は不要で、
    /// 開いた時点の `truncate` で毎回空から始まる。
    fn ensure_open(&self, guard: &mut Option<std::fs::File>) -> bool {
        if guard.is_none() {
            let _ = std::fs::create_dir_all(&self.dir);
            match std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(self.path())
            {
                Ok(f) => *guard = Some(f),
                Err(_) => return false,
            }
        }
        guard.is_some()
    }
}

impl LogSink for FileSink {
    fn append(&self, line: &str) {
        let mut guard = self.file.lock().expect("log file lock");
        if !self.ensure_open(&mut guard) {
            return;
        }
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// ログに出す経路（仮の経路 comments/approve に対応）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Comments,
    Approve,
    /// 経路が確定しない読み取りエラー・未知の経路。
    Unknown,
}

/// HTTP メソッド（ログには POST かどうかだけ出す）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    Post,
    Other,
}

/// コメント書き手の種別（`x-pelway-member-kind`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Human,
    Ai,
}

impl From<MemberKind> for ActorKind {
    fn from(kind: MemberKind) -> Self {
        match kind {
            MemberKind::Human => Self::Human,
            MemberKind::Ai => Self::Ai,
        }
    }
}

/// 拒否の結果コード。契約エラー・HTTP 層エラーの平坦化で、文言は持たない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Code {
    BadRequest,
    CardIdMismatch,
    NotFound,
    MethodNotAllowed,
    PayloadTooLarge,
    RequestTimeout,
    HeadersTooLarge,
    Internal,
    CardNotFound,
    VersionConflict,
    SnapshotMismatch,
    NotAwaitingApproval,
    ApprovalInProgress,
    IdempotencyKeyReused,
    EmptyBody,
    TooLong,
    /// 応答は拒否だが対応する結果コードが不明（通常は出ない）。
    Unknown,
}

/// 仮名化済み ID。生の ID からは [`Logger::id`] だけで作る。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Pseudonym(String);

impl fmt::Display for Pseudonym {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// ログの出来事。項目は仮名化 ID・種別・数値・結果コードだけ
/// （自由な文字列の欄は持たせない）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Event {
    /// 要求を受けた（ヘッダーまで読み終えた時点）。
    RequestReceived {
        route: Route,
        method: Method,
        /// 経路の card_id（仮名化）。経路が確定しない場合は `null`。
        card: Option<Pseudonym>,
    },
    /// 応答した。
    Responded {
        route: Route,
        status: u16,
        body_bytes: usize,
        elapsed_ms: u64,
    },
    /// コメントを保存した。
    CommentSaved {
        card: Pseudonym,
        author: Pseudonym,
        member_kind: ActorKind,
    },
    /// 承認した（この要求で新しい疑似送信が行われたときだけ出す）。
    Approved { card: Pseudonym },
    /// 模擬送信した（実外部 I/O は無い。メモリ上の記録のみ）。
    SimulatedSend { card: Pseudonym, send: Pseudonym },
    /// 拒否した（4xx）。
    Rejected {
        route: Route,
        status: u16,
        code: Code,
    },
    /// 内部エラー（5xx。応答本文は `{"code":"internal"}` のまま）。
    InternalError { route: Route },
}

/// 出力先と仮名化の salt を持つ。`Arc<Logger>` で接続スレッドに共有する。
pub struct Logger {
    sink: Arc<dyn LogSink>,
    salt: Vec<u8>,
}

impl fmt::Debug for Logger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // salt はログにも Debug にも出さない。
        f.debug_struct("Logger").finish_non_exhaustive()
    }
}

impl Logger {
    /// 出力先が無いロガー（[`crate::serve`] の既定）。
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            sink: Arc::new(NullSink),
            salt: Vec::new(),
        }
    }

    /// 出力先と salt を指定して作る。salt はログに出ない。
    #[must_use]
    pub fn with_sink(sink: Arc<dyn LogSink>, salt: Vec<u8>) -> Self {
        Self { sink, salt }
    }

    /// 出来事を 1 行の JSON（`ts_ms` 付き）で書き出す。時刻はここで付ける。
    pub fn emit(&self, event: Event) {
        let record = LogRecord {
            ts_ms: unix_millis(SystemTime::now()),
            event: &event,
        };
        if let Ok(line) = serde_json::to_string(&record) {
            self.sink.append(&line);
        }
    }

    /// 生の ID を仮名化する。ログに出る ID は必ずこの戻り値経由にする。
    #[must_use]
    pub fn id(&self, raw_id: &str) -> Pseudonym {
        Pseudonym(pseudonymize(&self.salt, raw_id))
    }
}

#[derive(Serialize)]
struct LogRecord<'a> {
    ts_ms: u64,
    #[serde(flatten)]
    event: &'a Event,
}

fn unix_millis(now: SystemTime) -> u64 {
    u64::try_from(now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis())).unwrap_or(u64::MAX)
}

/// プロセスごとの salt を作る。
///
/// 環境変数を読まない（依頼の約束）。worktree の外のファイルも読まないため
/// `/dev/urandom` も使わず、PID・起動時刻（ナノ秒）・ヒープ確保アドレス
/// （ASLR の影響を受ける）のバイト列を連結して返す。hash は
/// [`pseudonymize`] の中で掛かるため、ここでは一意性の素材だけを混ぜる。
/// 実運用では CSPRNG からの採取が望ましい（このデモでは未検証）。
#[must_use]
pub fn process_salt(now: SystemTime) -> Vec<u8> {
    let mut salt = Vec::new();
    salt.extend_from_slice(&std::process::id().to_ne_bytes());
    let nanos = now
        .duration_since(UNIX_EPOCH)
        .map_or(0u128, |d| d.as_nanos());
    salt.extend_from_slice(&nanos.to_ne_bytes());
    let probe = Box::new(0u8);
    salt.extend_from_slice(format!("{:p}", &*probe).as_bytes());
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_writes_one_json_line_with_ts_and_flattened_event() {
        let sink = Arc::new(MemorySink::new());
        let logger =
            Logger::with_sink(Arc::clone(&sink) as Arc<dyn LogSink>, b"unit-salt".to_vec());
        logger.emit(Event::RequestReceived {
            route: Route::Comments,
            method: Method::Post,
            card: Some(logger.id("card-unit")),
        });
        let lines = sink.lines();
        assert_eq!(lines.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&lines[0]).expect("valid json");
        assert!(v["ts_ms"].is_u64(), "ts_ms must be a number: {v}");
        assert_eq!(v["kind"], "request_received");
        assert_eq!(v["route"], "comments");
        assert_eq!(v["method"], "post");
        assert_eq!(
            v["card"],
            serde_json::json!(pseudonymize(b"unit-salt", "card-unit"))
        );
    }

    #[test]
    fn disabled_logger_emits_without_panicking() {
        let logger = Logger::disabled();
        logger.emit(Event::InternalError {
            route: Route::Unknown,
        });
    }

    #[test]
    fn file_sink_writes_jsonl_under_log_dir_and_cleans_up() {
        let sink = FileSink::create("unit-file-sink.jsonl").expect("create sink");
        let path = sink.path();
        assert_eq!(path, PathBuf::from(LOG_DIR).join("unit-file-sink.jsonl"));
        let logger = Logger::with_sink(Arc::new(sink), b"unit-salt".to_vec());
        logger.emit(Event::Responded {
            route: Route::Comments,
            status: 200,
            body_bytes: 3,
            elapsed_ms: 1,
        });
        let text = std::fs::read_to_string(&path).expect("read back");
        assert_eq!(text.lines().count(), 1);
        let v: serde_json::Value = serde_json::from_str(text.trim_end()).expect("jsonl line");
        assert_eq!(v["kind"], "responded");
        assert_eq!(v["status"], 200);
        // 後片付け（空いたディレクトリは git には見えない）。
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn file_sink_rejects_names_that_escape_log_dir() {
        for bad in [
            "",
            ".",
            "..",
            "../escape.jsonl",
            "a/b.jsonl",
            "a\\b.jsonl",
            "sub/../x.jsonl",
        ] {
            assert!(FileSink::create(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn process_salt_varies_with_time_and_is_never_empty() {
        let t1 = SystemTime::now();
        let t2 = t1 + std::time::Duration::from_nanos(1);
        // 同一入力でも呼び出しごとに変わる（ヒープ確保アドレス分の追加密は意図どおり）。
        // ここでは保証する性質だけ確かめる: 空でないことと、時刻が変われば必ず変わること。
        assert!(!process_salt(t1).is_empty());
        assert!(!process_salt(t2).is_empty());
        assert_ne!(process_salt(t1), process_salt(t2));
    }
}
