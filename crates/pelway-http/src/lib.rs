//! HTTP 層（下書き・デモ用・認証なし・ループバックのみ）。製品機能の下書きで、
//! 確定スキーマではない。
//!
//! - 経路は docs/technical/v0.7 に comments/approve の定義が無いための
//!   **仮の経路** `POST /v1/cards/{card_id}/comments` と `POST /v1/cards/{card_id}/approve`。
//! - 認証は無い（デモ用）。要求者は合成ヘッダー（`x-pelway-member-id`、
//!   `x-pelway-member-kind: human|ai`）で受ける。
//! - 待ち受けは 127.0.0.1 だけ。127.0.0.2 など 127/8 のほかのアドレスや `::1` を含む
//!   IPv6 も拒否（QA 18:21、リード決定 2026-11-09 19:14）。外部への接続はしない。
//! - [`ApprovalLedger`](pelway_contract::ledger::ApprovalLedger) 全体を 1 つの
//!   `std::sync::Mutex` で包んで共有する。1 つの要求の中で予約から模擬送信・
//!   完了までをロックを持ったまま行い、途中で別の要求が割り込まない。
//!   ロックが poison した場合は 500 を返して状態を変えない。
//! - 要求本文の上限は [`MAX_BODY_BYTES`]（64 KiB）。`Content-Length` が上限を超えたら
//!   **本文を読み込む前に** 413 を返す（残余の読み捨ては応答の後）。
//!   ヘッダーは大きさ [`MAX_HEAD_BYTES`] と行数 [`MAX_HEADER_LINES`] で制限し、超えたら 431。
//!   大きさはヘッダーの終端（空行）までのバイト数で判定し、同じ読み取りに本文の
//!   バイトが混ざっていても数えない（レビュー5 指摘1）。終端がまだ見つからない
//!   ときは受け取ったバイト数で判定する（ヘッダーだけで超えている場合）。
//! - `Content-Length` は ASCII 数字 1 つだけを受け付ける（重複・負・`+` 付き・非数字は 400）。
//!   `Transfer-Encoding` 付きの要求は扱わず 400。不正な JSON も 400。
//!   リクエスト行の HTTP バージョンは `HTTP/1.1` の形だけ、ヘッダー名は RFC 9110 の
//!   token（空・空白・区切り文字入りは不可）でなければ 400。
//!   いずれも台帳の状態は変えない。
//! - 読み取りは 1 回あたり [`READ_TIMEOUT`] 秒で打ち切る（408 `request_timeout`）。
//!   1 回の読み取りのタイムアウトは「3 秒」と「全体期限までの残り」の短いほうで、
//!   残りが 0 以下なら読まずに打ち切る（レビュー5 指摘2）。
//!   ヘッダー読み取り・本文読み取り・413 後の読み捨てを合わせた 1 接続あたりの全体期限
//!   [`CONNECTION_DEADLINE`]（10 秒）を超えても 408 を返して接続を閉じる
//!   （リード決定 2026-11-09 19:15、20:36 に本文の読み取りへ拡大）。止まった接続が
//!   あっても、接続ごとにスレッドを割り当てるため他の要求は処理される。
//! - 応答のエラー本文は契約エラー（`ApproveError` / `CommentError`）と同じ形の
//!   JSON（tag="code"、snake_case）。HTTP 層独自のエラーも [`HttpError`] で同じ形にする。
//! - 構造化ログ（[`log`]）: 出来事は列挙型で、項目は仮名化 ID・種別・数値・結果
//!   コードだけ。本文・コメント・`client_msg_id`・秘密はログに入れない
//!   （QA-GDPR-07・QA-GDPR-12・QA-MSG-02）。既定の出力先は無し
//!   （[`serve`]）。[`serve_with_logger`] で出力先を指定する（テストと既知 salt
//!   の検証用）。[`serve_with_logger_urandom`] は salt を必ず `/dev/urandom`
//!   から作り、読み取りに失敗したら起動しない（salt の採取元は引数に取らない。
//!   本番のログ付き起動はこの入口）。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};

use pelway_contract::approval::{ApproveRequest, SaveCommentRequest};
use pelway_contract::ids::MemberId;
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::MemberKind;
use serde::Serialize;

pub mod log;

/// 要求本文の上限（バイト）。超えたら 413。
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// 1 回の読み取りのタイムアウトの上限。実際のタイムアウトは「この 3 秒」と
/// 「全体期限までの残り」の短いほう（超えた接続は 408 で切断する）。
pub const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
/// 1 接続の全体期限（ヘッダー読み取り・本文読み取り・413 後の読み捨ての合計）。
/// 超えたら 408 を返して接続を閉じる（リード決定 2026-11-09 19:15、
/// 20:36 に本文の読み取りへ拡大）。
pub const CONNECTION_DEADLINE: std::time::Duration = std::time::Duration::from_secs(10);
/// ヘッダ部の上限（バイト）。超えたら 431。
pub const MAX_HEAD_BYTES: usize = 16 * 1024;
/// ヘッダ行数の上限。超えたら 431。
pub const MAX_HEADER_LINES: usize = 100;
/// 413 のとき読み捨てする上限（バイト）。これを超える要求は読み捨てずに切る。
const DRAIN_LIMIT_BYTES: usize = 1024 * 1024;

const MEMBER_ID_HEADER: &str = "x-pelway-member-id";
const MEMBER_KIND_HEADER: &str = "x-pelway-member-kind";

/// HTTP 層から見る台帳の共有状態。
pub type SharedLedger = Arc<Mutex<ApprovalLedger>>;

/// 起動の失敗。
#[derive(Debug)]
pub enum ServeError {
    /// 127.0.0.1 以外の待ち受けアドレス。127.0.0.2 など 127/8 のほか、`::1` も含む。
    NotLoopback(SocketAddr),
    /// ソケットの失敗。
    Io(std::io::Error),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLoopback(addr) => {
                write!(f, "serve address must be exactly 127.0.0.1, got {addr}")
            }
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ServeError {}

/// HTTP 層独自のエラー応答。契約エラーと同じ serde の形（tag="code"、snake_case）。
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum HttpError {
    /// 不正なリクエストライン・ヘッダー・JSON。
    BadRequest,
    /// comments で経路と本文の card_id が食い違う（approve は契約の
    /// [`ApproveError::CardIdMismatch`](pelway_contract::approval::ApproveError::CardIdMismatch)）。
    CardIdMismatch,
    /// 経路が無い。
    NotFound,
    /// 経路はあるがメソッドが POST 以外。
    MethodNotAllowed,
    /// 本文が [`MAX_BODY_BYTES`] を超えた。
    PayloadTooLarge,
    /// 読み取りが [`READ_TIMEOUT`] 秒を超えた、または接続全体が
    /// [`CONNECTION_DEADLINE`] を超えた（1 回の読み取りのタイムアウトは
    /// 3 秒と全体期限までの残りの短いほう）。
    RequestTimeout,
    /// ヘッダーが大きすぎる、または行数が [`MAX_HEADER_LINES`] を超えた。
    HeadersTooLarge,
    /// ロックが poison するなど、内部状態に触れない。
    Internal,
}

impl HttpError {
    #[must_use]
    pub fn http_status(&self) -> u16 {
        match self {
            Self::BadRequest => 400,
            Self::CardIdMismatch => 422,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::PayloadTooLarge => 413,
            Self::RequestTimeout => 408,
            Self::HeadersTooLarge => 431,
            Self::Internal => 500,
        }
    }
}

/// 応答（状態コードと JSON 本文）。`code` は拒否ログ用の結果コード（4xx/5xx で設定）。
struct Response {
    status: u16,
    body: String,
    /// 応答を書いた後に読み捨てる残りバイト数（413 のとき。本文を読み込む前に応答するため）。
    drain_after_write: Option<usize>,
    /// 拒否の結果コード。成功応答では `None`。
    code: Option<log::Code>,
}

impl Response {
    fn json<T: Serialize + ?Sized>(status: u16, value: &T) -> Self {
        Self {
            status,
            body: serde_json::to_string(value).expect("response serializes"),
            drain_after_write: None,
            code: None,
        }
    }

    fn error<E: Serialize>(status: u16, error: &E, code: log::Code) -> Self {
        Self {
            status,
            body: serde_json::to_string(error).expect("error serializes"),
            drain_after_write: None,
            code: Some(code),
        }
    }
}

fn bad_request() -> Response {
    Response::error(400, &HttpError::BadRequest, log::Code::BadRequest)
}

fn request_timeout() -> Response {
    Response::error(408, &HttpError::RequestTimeout, log::Code::RequestTimeout)
}

fn headers_too_large() -> Response {
    Response::error(431, &HttpError::HeadersTooLarge, log::Code::HeadersTooLarge)
}

fn internal_error() -> Response {
    Response::error(500, &HttpError::Internal, log::Code::Internal)
}

/// 起動したサーバー。テスト用に待ち受けアドレスだけ返す。
/// 停止 API は無い（デモ用。テストはプロセスの終了と共に止まる）。
pub struct Server {
    addr: SocketAddr,
}

impl Server {
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.addr
    }
}

/// サーバーを起動する。接続ごとにスレッドを割り当てる。
///
/// # Errors
/// 127.0.0.1 以外のアドレス、またはソケットの失敗。
pub fn serve(addr: SocketAddr, ledger: SharedLedger) -> Result<Server, ServeError> {
    serve_inner(
        addr,
        ledger,
        CONNECTION_DEADLINE,
        Arc::new(log::Logger::disabled()),
    )
}

/// [`serve`] と同じだが、1 接続の全体期限（既定は [`CONNECTION_DEADLINE`]）を指定できる。
/// テストで短い期限を注入するために分けてある。
/// 期限が `Instant` で表せる範囲を超えるとき（`Duration::MAX` など）は丸めて
/// panic しない（[`deadline_at`] の説明どおり）。
///
/// # Errors
/// 127.0.0.1 以外のアドレス、またはソケットの失敗。
pub fn serve_with_connection_deadline(
    addr: SocketAddr,
    ledger: SharedLedger,
    connection_deadline: std::time::Duration,
) -> Result<Server, ServeError> {
    serve_inner(
        addr,
        ledger,
        connection_deadline,
        Arc::new(log::Logger::disabled()),
    )
}

/// [`serve`] と同じだが、ログの出力先（と仮名化の salt を持つ [`log::Logger`]）を
/// 指定する。**テストと既知 salt の検証用**。本番では `/dev/urandom` 固定の入口
/// （[`serve_with_logger_urandom`]）を使う。ログに出る ID はすべて仮名化され、
/// 本文・コメント・`client_msg_id`・秘密はログに入らない。1 接続の全体期限は
/// [`CONNECTION_DEADLINE`]。
///
/// # Errors
/// 127.0.0.1 以外のアドレス、またはソケットの失敗。
pub fn serve_with_logger(
    addr: SocketAddr,
    ledger: SharedLedger,
    logger: Arc<log::Logger>,
) -> Result<Server, ServeError> {
    serve_inner(addr, ledger, CONNECTION_DEADLINE, logger)
}

/// [`serve_with_logger`] と同じログ付き起動だが、[`log::Logger`] はこちらで作る:
/// salt を必ず [`log::SALT_SOURCE`]（`/dev/urandom`）から [`log::SALT_BYTES`]
/// バイト読む。**salt の採取元を引数に取らない**（呼び出し側が固定内容のファイル
/// などを渡せない形。実体は [`log::SALT_SOURCE`] 固定で
/// `serve_with_logger_salt_from` に委譲）。salt の読み取りは待ち受け（bind）より
/// **前** に行い、失敗したら [`ServeError::Io`] で `Err` を返して待ち受けを始めない。
/// フォールバック（代替の salt）は作らない。PID・時刻・アドレスを使う経路も無い。
/// 本番のログ付き起動はこの入口を使う。
///
/// # Errors
/// salt の読み取りに失敗、127.0.0.1 以外のアドレス、またはソケットの失敗。
pub fn serve_with_logger_urandom(
    addr: SocketAddr,
    ledger: SharedLedger,
    sink: Arc<dyn log::LogSink>,
) -> Result<Server, ServeError> {
    serve_with_logger_salt_from(addr, ledger, sink, Path::new(log::SALT_SOURCE))
}

/// [`serve_with_logger_urandom`] の実体。salt の採取元を `salt_path` で受ける。
/// 公開入口は [`log::SALT_SOURCE`] 固定でこの関数に委譲する。採取元を引数に
/// 受けるのは**クレート内部のテストだけ**（salt の読み取り失敗を注入するために
/// 採取元を差し替える）。クレートの外からは見えない（`pub(crate)`）。本番・
/// example はパスを取らない [`serve_with_logger_urandom`] を使う。salt の読み取りは
/// 待ち受け（bind）より **前** に行い、失敗したら [`ServeError::Io`] で `Err` を
/// 返して待ち受けを始めない。フォールバック（代替の salt）は作らない。
///
/// # Errors
/// salt の読み取りに失敗、127.0.0.1 以外のアドレス、またはソケットの失敗。
pub(crate) fn serve_with_logger_salt_from(
    addr: SocketAddr,
    ledger: SharedLedger,
    sink: Arc<dyn log::LogSink>,
    salt_path: &Path,
) -> Result<Server, ServeError> {
    let salt = log::read_salt_from(salt_path).map_err(ServeError::Io)?;
    let logger = Arc::new(log::Logger::with_sink(sink, salt.to_vec()));
    serve_inner(addr, ledger, CONNECTION_DEADLINE, logger)
}

fn serve_inner(
    addr: SocketAddr,
    ledger: SharedLedger,
    connection_deadline: std::time::Duration,
    logger: Arc<log::Logger>,
) -> Result<Server, ServeError> {
    // 待ち受けは 127.0.0.1 だけ（QA 18:21、リード決定 2026-11-09 19:14）。
    // 127.0.0.2 など 127/8 のほか、`::1` を含む IPv6 も拒否する。
    if addr.ip() != std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST) {
        return Err(ServeError::NotLoopback(addr));
    }
    let listener = TcpListener::bind(addr).map_err(ServeError::Io)?;
    let addr = listener.local_addr().map_err(ServeError::Io)?;
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let (Ok(mut stream), ledger, logger) =
                (stream, Arc::clone(&ledger), Arc::clone(&logger))
            else {
                continue;
            };
            std::thread::spawn(move || {
                handle_conn(&mut stream, &ledger, connection_deadline, &logger);
            });
        }
    });
    Ok(Server { addr })
}

struct Request {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// 接続開始時刻 `now` から全体期限（`Instant`）を作る。`budget` が `Instant` で
/// 表せる範囲を超える（`checked_add` が `None`）ときは 1 年に丸め、それでも
/// 表せないときは `now`（残り 0 の期限切れとして扱われる）を返す。
/// どの場合も panic しない（レビュー6 指摘1）。
///
/// 丸めを選んだ理由: あふれるほど巨大な期限は「無期限」の意図であり、本番の
/// 10 秒より十分長い 1 年は実運用上それと区別がつかない。エラーにしないのは、
/// [`ServeError`] は待ち受けアドレス・ソケットの失敗を表すもので、期限の大きさの
/// 解釈で起動を失敗させるのが不釣り合いなため。
fn deadline_at(now: std::time::Instant, budget: std::time::Duration) -> std::time::Instant {
    /// あふれる指定を丸める先。本番・テストのどの期限より十分長く、`Instant` の
    /// 表現範囲（基準時刻から数百年）には十分収まる。
    const FAR_FUTURE: std::time::Duration = std::time::Duration::from_secs(365 * 24 * 60 * 60);
    now.checked_add(budget)
        .or_else(|| now.checked_add(FAR_FUTURE))
        .unwrap_or(now)
}

fn handle_conn(
    stream: &mut TcpStream,
    ledger: &SharedLedger,
    connection_deadline: std::time::Duration,
    logger: &log::Logger,
) {
    let started = std::time::Instant::now();
    // 全体期限（リード決定 2026-11-09 19:15、20:36 に本文の読み取りへ拡大）:
    // ヘッダー読み取り・本文読み取り・413 後の読み捨てを合わせた全体。
    // 1 回の読み取りのタイムアウトは read_once / drain が読み取りのたびに設定する。
    let deadline = deadline_at(std::time::Instant::now(), connection_deadline);
    let (response, route) = match read_request(stream, deadline) {
        Ok(req) => route(&req, ledger, logger),
        Err(resp) => (resp, log::Route::Unknown),
    };
    let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    // 拒否（4xx）と内部エラー（5xx）の出来事。結果コードは応答と一緒に作ったもの。
    if response.status >= 500 {
        logger.emit(log::Event::InternalError { route });
    } else if response.status >= 400 {
        logger.emit(log::Event::Rejected {
            route,
            status: response.status,
            code: response.code.unwrap_or(log::Code::Unknown),
        });
    }
    // 応答した: 経路・状態コード・本文バイト数・所要ミリ秒だけ。
    logger.emit(log::Event::Responded {
        route,
        status: response.status,
        body_bytes: response.body.len(),
        elapsed_ms,
    });
    write_response(stream, &response);
    // 413 のときは応答を先に書き、残りはこの後で読み捨てる。
    if let Some(remaining) = response.drain_after_write {
        drain(stream, remaining, deadline);
    }
}

/// 1 回の読み取りのタイムアウト: [`READ_TIMEOUT`]（3 秒）と全体期限までの残りの
/// 短いほう。残りが 0 以下（期限切れ）のときは `None`（0 秒のタイムアウトは
/// 設定できないため、期限切れは読み取りの前に判定する）。
fn read_timeout_for(deadline: std::time::Instant) -> Option<std::time::Duration> {
    let remaining = deadline.checked_duration_since(std::time::Instant::now())?;
    if remaining.is_zero() {
        return None;
    }
    Some(READ_TIMEOUT.min(remaining))
}

/// 応答後の読み捨て。EOF・タイムアウト・エラー、または全体期限で打ち切って終了
/// （413 は応答済みなので、応答を追記せず接続を閉じるだけ）。
fn drain(stream: &mut TcpStream, mut remaining: usize, deadline: std::time::Instant) {
    let mut sink = [0u8; 4096];
    while remaining > 0 {
        // 1 回の読み取りも全体期限で打ち切る。期限が来ていれば読まずに終わる。
        let Some(timeout) = read_timeout_for(deadline) else {
            break;
        };
        let _ = stream.set_read_timeout(Some(timeout));
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(n) => remaining -= n.min(remaining),
        }
    }
}

/// 1 回読む。タイムアウト（3 秒と全体期限までの残りの短いほう）は 408、それ以外の
/// 読み取り失敗は 400 に対応付ける。残りが 0 以下なら読まずに 408 を返す。
fn read_once(
    stream: &mut TcpStream,
    chunk: &mut [u8; 4096],
    deadline: std::time::Instant,
) -> Result<usize, Response> {
    let Some(timeout) = read_timeout_for(deadline) else {
        return Err(request_timeout());
    };
    let _ = stream.set_read_timeout(Some(timeout));
    match stream.read(chunk) {
        Ok(n) => Ok(n),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            Err(request_timeout())
        }
        Err(_) => Err(bad_request()),
    }
}

fn find_terminate(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// RFC 9110 の token 構成文字（tchar）なら true。ヘッダー名は 1*token でなければ 400。
fn is_tchar(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

/// 要求を読む。ヘッダ上限・Content-Length の検査・本文上限・全体期限をここで行う。
/// `Err` のときはそのまま書くべき応答（413 は応答後に読み捨てる分を含む）。
fn read_request(stream: &mut TcpStream, deadline: std::time::Instant) -> Result<Request, Response> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        // 大きさの上限はヘッダーの終端（空行）までのバイト数で判定する
        // （レビュー5 指摘1）。同じ読み取りに本文のバイトが混ざっていても
        // 上限には数えない。
        if let Some(pos) = find_terminate(&buf) {
            if pos + 4 > MAX_HEAD_BYTES {
                return Err(headers_too_large());
            }
            break pos;
        }
        // 終端がまだ見つからない: 受け取ったバイト数が上限を超えていれば、
        // ヘッダーだけで超えているので 431。
        if buf.len() > MAX_HEAD_BYTES {
            return Err(headers_too_large());
        }
        // 全体期限は read_once が読み取りごとに見る（期限切れは 408）。
        let n = read_once(stream, &mut chunk, deadline)?;
        if n == 0 {
            return Err(bad_request());
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut parts = lines.next().unwrap_or("").split_whitespace();
    // リクエスト行の HTTP バージョンは HTTP/1.1 の形だけ（ほかは 400）。
    let (method, path) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(method), Some(path), Some("HTTP/1.1"), None) => (method, path),
        _ => return Err(bad_request()),
    };

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        if headers.len() >= MAX_HEADER_LINES {
            return Err(headers_too_large());
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(bad_request());
        };
        // ヘッダー名は RFC 9110 の token だけ（空・空白・区切り文字入りは 400）。
        if name.is_empty() || !name.bytes().all(is_tchar) {
            return Err(bad_request());
        }
        headers.push((name.to_ascii_lowercase(), value.trim().to_owned()));
    }

    let mut content_length: Option<usize> = None;
    for (name, value) in &headers {
        match name.as_str() {
            "content-length" => {
                // ASCII 数字だけを受け付ける（負・+付き・非数字・空は 400）。
                if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad_request());
                }
                let len: usize = value.parse().map_err(|_| bad_request())?;
                if content_length.replace(len).is_some() {
                    return Err(bad_request()); // 重複は断る
                }
            }
            "transfer-encoding" => return Err(bad_request()), // chunked は非対応
            _ => {}
        }
    }
    let Some(content_length) = content_length else {
        return Err(bad_request());
    };

    if content_length > MAX_BODY_BYTES {
        // 本文を読み込む前に 413 を返す（QA 18:21 の2点目）。
        // 応答後に読み捨てる残り。大きすぎる（>1 MiB）場合は読み捨てない。
        let already = buf.len().saturating_sub(head_end + 4);
        let remaining = content_length.saturating_sub(already);
        let mut resp =
            Response::error(413, &HttpError::PayloadTooLarge, log::Code::PayloadTooLarge);
        resp.drain_after_write = (content_length <= DRAIN_LIMIT_BYTES).then_some(remaining);
        return Err(resp);
    }

    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        // 本文の読み取りにも同じ全体期限をかける（リード決定 2026-11-09 20:36）。
        let n = read_once(stream, &mut chunk, deadline)?;
        if n == 0 {
            return Err(bad_request());
        }
        body.extend_from_slice(&chunk[..n]);
    }
    body.truncate(content_length);

    Ok(Request {
        method: method.to_owned(),
        path: path.to_owned(),
        headers,
        body,
    })
}

fn header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.as_str())
}

fn route(req: &Request, ledger: &SharedLedger, logger: &log::Logger) -> (Response, log::Route) {
    let path = req.path.split('?').next().unwrap_or("");
    let segments: Vec<&str> = path.split('/').collect();
    let method = if req.method == "POST" {
        log::Method::Post
    } else {
        log::Method::Other
    };
    // 仮の経路: /v1/cards/{card_id}/comments と /v1/cards/{card_id}/approve
    if segments.len() == 5
        && segments[1] == "v1"
        && segments[2] == "cards"
        && !segments[3].is_empty()
    {
        let card = segments[3];
        let route_kind = match segments[4] {
            "comments" => log::Route::Comments,
            "approve" => log::Route::Approve,
            _ => log::Route::Unknown,
        };
        // 要求を受けた: 経路・メソッド・経路の card_id（仮名化）だけ。
        logger.emit(log::Event::RequestReceived {
            route: route_kind,
            method,
            card: Some(logger.id(card)),
        });
        let resp = match (segments[4], req.method.as_str()) {
            ("comments", "POST") => handle_comments(req, card, ledger, logger),
            ("approve", "POST") => handle_approve(req, card, ledger, logger),
            ("comments", _) | ("approve", _) => Response::error(
                405,
                &HttpError::MethodNotAllowed,
                log::Code::MethodNotAllowed,
            ),
            _ => Response::error(404, &HttpError::NotFound, log::Code::NotFound),
        };
        return (resp, route_kind);
    }
    logger.emit(log::Event::RequestReceived {
        route: log::Route::Unknown,
        method,
        card: None,
    });
    (
        Response::error(404, &HttpError::NotFound, log::Code::NotFound),
        log::Route::Unknown,
    )
}

/// 契約のコメント保存エラー → 拒否ログの結果コード。
fn comment_error_code(e: &pelway_contract::approval::CommentError) -> log::Code {
    use pelway_contract::approval::CommentError as E;
    match e {
        E::CardNotFound => log::Code::CardNotFound,
        E::EmptyBody => log::Code::EmptyBody,
        E::TooLong { .. } => log::Code::TooLong,
        E::NotAwaitingApproval { .. } => log::Code::NotAwaitingApproval,
    }
}

/// 契約の承認エラー → 拒否ログの結果コード。
fn approve_error_code(e: &pelway_contract::approval::ApproveError) -> log::Code {
    use pelway_contract::approval::ApproveError as E;
    match e {
        E::CardNotFound => log::Code::CardNotFound,
        E::CardIdMismatch => log::Code::CardIdMismatch,
        E::VersionConflict { .. } => log::Code::VersionConflict,
        E::SnapshotMismatch => log::Code::SnapshotMismatch,
        E::NotAwaitingApproval { .. } => log::Code::NotAwaitingApproval,
        E::ApprovalInProgress => log::Code::ApprovalInProgress,
        E::IdempotencyKeyReused => log::Code::IdempotencyKeyReused,
    }
}

fn handle_comments(
    req: &Request,
    path_card: &str,
    ledger: &SharedLedger,
    logger: &log::Logger,
) -> Response {
    // 認証は無い（デモ用）。合成ヘッダーで要求者を受ける。
    let Some(member_id) = header(req, MEMBER_ID_HEADER).filter(|v| !v.is_empty()) else {
        return bad_request();
    };
    let Some(kind) = header(req, MEMBER_KIND_HEADER) else {
        return bad_request();
    };
    let member_kind = match kind {
        "human" => MemberKind::Human,
        "ai" => MemberKind::Ai,
        _ => return bad_request(),
    };

    let Ok(save_req) = serde_json::from_slice::<SaveCommentRequest>(&req.body) else {
        return bad_request();
    };
    // comments には契約側に check_path 相当が無いため、HTTP 層で同じ 422 にする。
    if save_req.card_id.as_str() != path_card {
        return Response::error(422, &HttpError::CardIdMismatch, log::Code::CardIdMismatch);
    }

    // 保存はロックを持ったまま 1 回で行う。
    match ledger.lock() {
        Ok(mut ledger) => {
            // ログ用の仮名 ID を先に作る（save_req はこの後動かす）。
            let card_pid = logger.id(save_req.card_id.as_str());
            let author_pid = logger.id(member_id);
            match ledger.save_comment(MemberId::from(member_id), member_kind, save_req) {
                Ok(saved) => {
                    // コメントを保存した: カード・書き手（ともに仮名化）と種別だけ。
                    logger.emit(log::Event::CommentSaved {
                        card: card_pid,
                        author: author_pid,
                        member_kind: member_kind.into(),
                    });
                    Response::json(200, &saved)
                }
                Err(e) => Response::error(e.http_status(), &e, comment_error_code(&e)),
            }
        }
        Err(_) => internal_error(),
    }
}

fn handle_approve(
    req: &Request,
    path_card: &str,
    ledger: &SharedLedger,
    logger: &log::Logger,
) -> Response {
    let Ok(approve_req) = serde_json::from_slice::<ApproveRequest>(&req.body) else {
        return bad_request();
    };
    // 経路と本文の card_id は既存の check_path に従う。
    if let Err(e) = approve_req.check_path(&pelway_contract::ids::CardId::from(path_card)) {
        return Response::error(e.http_status(), &e, approve_error_code(&e));
    }

    // 予約から模擬送信・完了までをロックを持ったまま行い、割り込みを許さない。
    match ledger.lock() {
        Ok(mut ledger) => {
            // 冪等リプレイ（再送）では send_log が増えない。この要求で新しい
            // 疑似送信が行われたときだけ承認・模擬送信の出来事を出す。
            let sends_before = ledger.send_log().len();
            match ledger.approve(approve_req) {
                Ok(outcome) => {
                    if ledger.send_log().len() > sends_before {
                        let card_pid = logger.id(outcome.card_id.as_str());
                        let send_pid = logger.id(outcome.simulated_send_id.as_str());
                        logger.emit(log::Event::Approved {
                            card: card_pid.clone(),
                        });
                        logger.emit(log::Event::SimulatedSend {
                            card: card_pid,
                            send: send_pid,
                        });
                    }
                    Response::json(200, &outcome)
                }
                Err(e) => Response::error(e.http_status(), &e, approve_error_code(&e)),
            }
        }
        Err(_) => internal_error(),
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}

fn write_response(stream: &mut TcpStream, resp: &Response) {
    let head = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        resp.status,
        reason_phrase(resp.status),
        resp.body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(resp.body.as_bytes());
    let _ = stream.flush();
    // 応答を先に届かせる（Connection: close）。読み捨てはこの後。
    let _ = stream.shutdown(std::net::Shutdown::Write);
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use pelway_contract::ledger::ApprovalLedger;

    use super::{ServeError, SharedLedger, serve_with_logger_salt_from};
    use crate::log::{LogSink, MemorySink};

    // (s2) salt の読み取りが失敗する状態では、ログ付きのサーバー起動は Err を返し、
    // 待ち受けを始めない（salt の読み取りは bind より前。フォールバックなし）。
    // 元は結合テスト（tests/no_body_logs.rs）だったが、salt のパスを受けるこの入口を
    // クレート外に公開しない（pub(crate)）形にしたため、ここに移した。
    #[test]
    fn serve_with_failing_salt_source_returns_err_without_listening() {
        let ledger: SharedLedger = Arc::new(Mutex::new(ApprovalLedger::new()));
        let sink = Arc::new(MemorySink::new());
        let res = serve_with_logger_salt_from(
            "127.0.0.1:0".parse().unwrap(),
            Arc::clone(&ledger),
            Arc::clone(&sink) as Arc<dyn LogSink>,
            std::path::Path::new("target/demo-logs/absent-salt-source"),
        );
        // Err の時点でリスナーは立っていない（salt の読み取りは bind の前。
        // Ok が返る場合のみ待ち受けを始める）。
        let err = res
            .map(|_| ())
            .expect_err("salt の読み取りに失敗したら起動は Err");
        match err {
            ServeError::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("salt 由来の Io エラーではない: {other:?}"),
        }
        assert!(sink.lines().is_empty());
    }
}
