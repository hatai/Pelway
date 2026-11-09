//! HTTP 層（下書き・デモ用・認証なし・ループバックのみ）。製品機能の下書きで、
//! 確定スキーマではない。
//!
//! - 経路は docs/technical/v0.7 に comments/approve の定義が無いための
//!   **仮の経路** `POST /v1/cards/{card_id}/comments` と `POST /v1/cards/{card_id}/approve`。
//! - 認証は無い（デモ用）。要求者は合成ヘッダー（`x-pelway-member-id`、
//!   `x-pelway-member-kind: human|ai`）で受ける。
//! - 待ち受けはループバック（127.0.0.1 / ::1）だけ。ほかのアドレスは拒否。
//!   外部への接続はしない。
//! - [`ApprovalLedger`](pelway_contract::ledger::ApprovalLedger) 全体を 1 つの
//!   `std::sync::Mutex` で包んで共有する。1 つの要求の中で予約から模擬送信・
//!   完了までをロックを持ったまま行い、途中で別の要求が割り込まない。
//!   ロックが poison した場合は 500 を返して状態を変えない。
//! - 要求本文の上限は [`MAX_BODY_BYTES`]（64 KiB）。超えたら 413。
//!   不正な JSON は 400。どちらも台帳の状態は変えない。
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
/// ヘッダ部の上限（バイト）。超えたら 400。
const MAX_HEAD_BYTES: usize = 16 * 1024;
/// 413 のとき読み捨てする上限（バイト）。これを超える要求は読み捨てずに切る。
const DRAIN_LIMIT_BYTES: usize = 1024 * 1024;

const MEMBER_ID_HEADER: &str = "x-pelway-member-id";
const MEMBER_KIND_HEADER: &str = "x-pelway-member-kind";

/// HTTP 層から見る台帳の共有状態。
pub type SharedLedger = Arc<Mutex<ApprovalLedger>>;

/// 起動の失敗。
#[derive(Debug)]
pub enum ServeError {
    /// ループバック以外の待ち受けアドレス。
    NotLoopback(SocketAddr),
    /// ソケットの失敗。
    Io(std::io::Error),
}

impl std::fmt::Display for ServeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLoopback(addr) => write!(f, "serve address must be loopback, got {addr}"),
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
            Self::Internal => 500,
        }
    }
}

/// 応答（状態コードと JSON 本文）。
struct Response {
    status: u16,
    body: String,
}

impl Response {
    fn json<T: Serialize + ?Sized>(status: u16, value: &T) -> Self {
        Self {
            status,
            body: serde_json::to_string(value).expect("response serializes"),
        }
    }

    fn error<E: Serialize>(status: u16, error: &E) -> Self {
        Self {
            status,
            body: serde_json::to_string(error).expect("error serializes"),
        }
    }
}

fn bad_request() -> Response {
    Response::error(400, &HttpError::BadRequest)
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
/// ループバック以外のアドレス、またはソケットの失敗。
pub fn serve(addr: SocketAddr, ledger: SharedLedger) -> Result<Server, ServeError> {
    if !addr.ip().is_loopback() {
        return Err(ServeError::NotLoopback(addr));
    }
    let listener = TcpListener::bind(addr).map_err(ServeError::Io)?;
    let addr = listener.local_addr().map_err(ServeError::Io)?;
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let (Ok(mut stream), ledger) = (stream, Arc::clone(&ledger)) else {
                continue;
            };
            std::thread::spawn(move || handle_conn(&mut stream, &ledger));
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

fn handle_conn(stream: &mut TcpStream, ledger: &SharedLedger) {
    let response = match read_request(stream) {
        Ok(req) => route(&req, ledger),
        Err(resp) => resp,
    };
    write_response(stream, &response);
}

fn find_terminate(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

/// 要求を読む。ヘッダ上限・Content-Length の有無・本文上限をここで検査する。
/// `Err` のときはそのまま書くべき応答。
fn read_request(stream: &mut TcpStream) -> Result<Request, Response> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let head_end = loop {
        if let Some(pos) = find_terminate(&buf) {
            break pos;
        }
        if buf.len() > MAX_HEAD_BYTES {
            return Err(bad_request());
        }
        let n = stream.read(&mut chunk).map_err(|_| bad_request())?;
        if n == 0 {
            return Err(bad_request());
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let mut parts = lines.next().unwrap_or("").split_whitespace();
    let (method, path) = match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(method), Some(path), Some(_version), None) => (method, path),
        _ => return Err(bad_request()),
    };

    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(bad_request());
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
    }

    let mut content_length: Option<usize> = None;
    for (name, value) in &headers {
        match name.as_str() {
            "content-length" => {
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
        // RST で応答が壊れないよう、小さな超過なら読み捨ててから 413 を返す。
        if content_length <= DRAIN_LIMIT_BYTES {
            let mut remaining = content_length.saturating_sub(buf.len() - head_end - 4);
            let mut sink = [0u8; 4096];
            while remaining > 0 {
                let n = stream.read(&mut sink).unwrap_or(0);
                if n == 0 {
                    break;
                }
                remaining -= n.min(remaining);
            }
        }
        return Err(Response::error(413, &HttpError::PayloadTooLarge));
    }

    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).map_err(|_| bad_request())?;
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
        409 => "Conflict",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
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
}
