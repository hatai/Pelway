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
//!   ヘッダーは大きさ [`MAX_HEAD_BYTES`]（受け取った合計バイト数で判定）と行数
//!   [`MAX_HEADER_LINES`] で制限し、超えたら 431。
//! - `Content-Length` は ASCII 数字 1 つだけを受け付ける（重複・負・`+` 付き・非数字は 400）。
//!   `Transfer-Encoding` 付きの要求は扱わず 400。不正な JSON も 400。
//!   リクエスト行の HTTP バージョンは `HTTP/1.1` の形だけ、ヘッダー名は RFC 9110 の
//!   token（空・空白・区切り文字入りは不可）でなければ 400。
//!   いずれも台帳の状態は変えない。
//! - 読み取りは [`READ_TIMEOUT`] 秒で打ち切る（408 `request_timeout`）。さらにヘッダー
//!   読み取りと 413 後の読み捨てを合わせた 1 接続あたりの全体期限
//!   [`CONNECTION_DEADLINE`]（10 秒）があり、超えたら 408 を返して接続を閉じる
//!   （リード決定 2026-11-09 19:15）。止まった接続があっても、接続ごとにスレッドを
//!   割り当てるため他の要求は処理される。
//! - 応答のエラー本文は契約エラー（`ApproveError` / `CommentError`）と同じ形の
//!   JSON（tag="code"、snake_case）。HTTP 層独自のエラーも [`HttpError`] で同じ形にする。

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use pelway_contract::approval::{ApproveRequest, SaveCommentRequest};
use pelway_contract::ids::MemberId;
use pelway_contract::ledger::ApprovalLedger;
use pelway_contract::member::MemberKind;
use serde::Serialize;

/// 要求本文の上限（バイト）。超えたら 413。
pub const MAX_BODY_BYTES: usize = 64 * 1024;
/// 読み取りのタイムアウト。超えた接続は 408 で切断する。
pub const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);
/// 1 接続の全体期限（ヘッダー読み取りと 413 後の読み捨ての合計）。
/// 超えたら 408 を返して接続を閉じる（リード決定 2026-11-09 19:15）。
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
    /// [`CONNECTION_DEADLINE`] を超えた。
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

/// 応答（状態コードと JSON 本文）。
struct Response {
    status: u16,
    body: String,
    /// 応答を書いた後に読み捨てる残りバイト数（413 のとき。本文を読み込む前に応答するため）。
    drain_after_write: Option<usize>,
}

impl Response {
    fn json<T: Serialize + ?Sized>(status: u16, value: &T) -> Self {
        Self {
            status,
            body: serde_json::to_string(value).expect("response serializes"),
            drain_after_write: None,
        }
    }

    fn error<E: Serialize>(status: u16, error: &E) -> Self {
        Self {
            status,
            body: serde_json::to_string(error).expect("error serializes"),
            drain_after_write: None,
        }
    }
}

fn bad_request() -> Response {
    Response::error(400, &HttpError::BadRequest)
}

fn request_timeout() -> Response {
    Response::error(408, &HttpError::RequestTimeout)
}

fn headers_too_large() -> Response {
    Response::error(431, &HttpError::HeadersTooLarge)
}

fn internal_error() -> Response {
    Response::error(500, &HttpError::Internal)
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
    serve_with_connection_deadline(addr, ledger, CONNECTION_DEADLINE)
}

/// [`serve`] と同じだが、1 接続の全体期限（既定は [`CONNECTION_DEADLINE`]）を指定できる。
/// テストで短い期限を注入するために分けてある。
///
/// # Errors
/// 127.0.0.1 以外のアドレス、またはソケットの失敗。
pub fn serve_with_connection_deadline(
    addr: SocketAddr,
    ledger: SharedLedger,
    connection_deadline: std::time::Duration,
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
            let (Ok(mut stream), ledger) = (stream, Arc::clone(&ledger)) else {
                continue;
            };
            std::thread::spawn(move || handle_conn(&mut stream, &ledger, connection_deadline));
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

fn handle_conn(
    stream: &mut TcpStream,
    ledger: &SharedLedger,
    connection_deadline: std::time::Duration,
) {
    // 全体期限（リード決定 2026-11-09 19:15）: ヘッダー読み取りと 413 後の読み捨ての合計。
    let deadline = std::time::Instant::now() + connection_deadline;
    let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
    let response = match read_request(stream, deadline) {
        Ok(req) => route(&req, ledger),
        Err(resp) => resp,
    };
    write_response(stream, &response);
    // 413 のときは応答を先に書き、残りはこの後で読み捨てる。
    if let Some(remaining) = response.drain_after_write {
        drain(stream, remaining, deadline);
    }
}

/// 応答後の読み捨て。EOF・タイムアウト・エラー、または全体期限で打ち切って終了。
fn drain(stream: &mut TcpStream, mut remaining: usize, deadline: std::time::Instant) {
    let mut sink = [0u8; 4096];
    while remaining > 0 {
        if std::time::Instant::now() >= deadline {
            break;
        }
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(n) => remaining -= n.min(remaining),
        }
    }
}

/// 1 回読む。タイムアウトは 408、それ以外の読み取り失敗は 400 に対応付ける。
fn read_once(stream: &mut TcpStream, chunk: &mut [u8; 4096]) -> Result<usize, Response> {
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
        // 大きさの上限は「受け取った合計バイト数」で判定する（終端を探す前に）。
        // 終端（空行）が上限を超えた最後の読み取りに含まれる場合も 431。
        if buf.len() > MAX_HEAD_BYTES {
            return Err(headers_too_large());
        }
        if let Some(pos) = find_terminate(&buf) {
            break pos;
        }
        // 全体期限（リード決定 19:15）。超えたら 408 を返して接続を閉じる。
        if std::time::Instant::now() >= deadline {
            return Err(request_timeout());
        }
        let n = read_once(stream, &mut chunk)?;
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
        let mut resp = Response::error(413, &HttpError::PayloadTooLarge);
        resp.drain_after_write = (content_length <= DRAIN_LIMIT_BYTES).then_some(remaining);
        return Err(resp);
    }

    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        let n = read_once(stream, &mut chunk)?;
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

fn route(req: &Request, ledger: &SharedLedger) -> Response {
    let path = req.path.split('?').next().unwrap_or("");
    let segments: Vec<&str> = path.split('/').collect();
    // 仮の経路: /v1/cards/{card_id}/comments と /v1/cards/{card_id}/approve
    if segments.len() == 5
        && segments[1] == "v1"
        && segments[2] == "cards"
        && !segments[3].is_empty()
    {
        let card = segments[3];
        return match (segments[4], req.method.as_str()) {
            ("comments", "POST") => handle_comments(req, card, ledger),
            ("approve", "POST") => handle_approve(req, card, ledger),
            ("comments", _) | ("approve", _) => Response::error(405, &HttpError::MethodNotAllowed),
            _ => Response::error(404, &HttpError::NotFound),
        };
    }
    Response::error(404, &HttpError::NotFound)
}

fn handle_comments(req: &Request, path_card: &str, ledger: &SharedLedger) -> Response {
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
        return Response::error(422, &HttpError::CardIdMismatch);
    }

    // 保存はロックを持ったまま 1 回で行う。
    match ledger.lock() {
        Ok(mut ledger) => {
            match ledger.save_comment(MemberId::from(member_id), member_kind, save_req) {
                Ok(saved) => Response::json(200, &saved),
                Err(e) => Response::error(e.http_status(), &e),
            }
        }
        Err(_) => internal_error(),
    }
}

fn handle_approve(req: &Request, path_card: &str, ledger: &SharedLedger) -> Response {
    let Ok(approve_req) = serde_json::from_slice::<ApproveRequest>(&req.body) else {
        return bad_request();
    };
    // 経路と本文の card_id は既存の check_path に従う。
    if let Err(e) = approve_req.check_path(&pelway_contract::ids::CardId::from(path_card)) {
        return Response::error(e.http_status(), &e);
    }

    // 予約から模擬送信・完了までをロックを持ったまま行い、割り込みを許さない。
    match ledger.lock() {
        Ok(mut ledger) => match ledger.approve(approve_req) {
            Ok(outcome) => Response::json(200, &outcome),
            Err(e) => Response::error(e.http_status(), &e),
        },
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
