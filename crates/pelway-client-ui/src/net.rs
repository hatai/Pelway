//! HTTP/1.1 の最小クライアント（Issue 333。`pelway-http` のサーバーと話す）。
//!
//! 外部の HTTP クレートは足さない——サーバーと同じく `TcpStream` の直書きで、
//! 接続先はループバックだけ、接続ごとに `Connection: close` で 1 要求ずつ送る。
//! 応答の body は `Content-Length` で決まる長さを読む（サーバーも同じ形で返す）。
//! 要求・応答の本文はログに出さない・ファイルに書かない（合成データの方針どおり）。

use std::fmt;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

/// 1 要求の期限（接続・読み・書きのそれぞれ）。UI スレッドを止めない範囲の値。
const IO_TIMEOUT: Duration = Duration::from_secs(3);

/// 応答の上限（デモのデータは小さいが、読み取りに上限を持たせて待ち続けない）。
const MAX_RESPONSE_BYTES: u64 = 1 << 20;

/// HTTP の応答。
#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

/// 通信の失敗。接続できない・途中で切れる・応答の形がおかしい、の 3 つに畳む
/// （UI は「つながっていません」の 1 状態として扱う）。
#[derive(Debug)]
pub enum NetError {
    /// 接続・読み書きの失敗（接続拒否・タイムアウトを含む）。
    Io(std::io::Error),
    /// 応答が HTTP/1.1 として読めない（status 行やヘッダー区切りが無い）、
    /// または要求ターゲット・ヘッダー値が送れない形。
    Malformed,
    /// 接続先がループバックでない（`live::parse_server_addr` を通らない
    /// 経路からの流入をここでも防ぐ）。
    NonLoopback,
}

impl fmt::Display for NetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "connection or I/O failed: {e}"),
            Self::Malformed => f.write_str("malformed HTTP response"),
            Self::NonLoopback => f.write_str("non-loopback target"),
        }
    }
}

impl std::error::Error for NetError {}

impl From<std::io::Error> for NetError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// ループバック固定の最小クライアント。TLS は張らない（Issue 327 の受け側と同じ）。
#[derive(Debug, Clone, Copy)]
pub struct Client {
    addr: SocketAddr,
}

impl Client {
    #[must_use]
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    /// GET。`path` は `/` から始まる経路とクエリ（作る側で組み立て済み）。
    /// サーバーは GET にも `Content-Length` ヘッダーを必須とする（Issue 327 の
    /// `read_request` が全要求で確認する）ため `Content-Length: 0` を付ける。
    ///
    /// # Errors
    /// 接続・読み書きの失敗、応答が読めない形、または `path` が送れない形
    /// （空白・制御文字を含む）なら [`NetError`]。
    pub fn get(&self, path: &str) -> Result<Response, NetError> {
        check_target(path)?;
        let head = format!(
            "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n",
            host = self.addr
        );
        self.exchange(head.into_bytes())
    }

    /// POST（JSON）。`headers` は要求ヘッダー名と値のペア（合成ヘッダー
    /// `x-pelway-member-id` 等を呼び出し側が渡す）。値に CR/LF は受け付けない
    /// （注入を構造で防ぐ）。
    ///
    /// # Errors
    /// 同上。ヘッダー値の CR/LF は [`NetError::Malformed`]。
    pub fn post(
        &self,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<Response, NetError> {
        check_target(path)?;
        let mut head = format!(
            "POST {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nContent-Type: application/json\r\n",
            host = self.addr
        );
        for (name, value) in headers {
            if name.contains(['\r', '\n']) || value.contains(['\r', '\n']) {
                return Err(NetError::Malformed);
            }
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
        let mut request = head.into_bytes();
        request.extend_from_slice(body);
        self.exchange(request)
    }

    fn exchange(&self, request: Vec<u8>) -> Result<Response, NetError> {
        // 接続先はループバックだけ。パーサを通さず `Client::new` に
        // 直接アドレスが渡された経路でも、TCP を張る直前にここで止める。
        if !self.addr.ip().is_loopback() {
            return Err(NetError::NonLoopback);
        }
        let mut stream = TcpStream::connect_timeout(&self.addr, IO_TIMEOUT)?;
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        stream.set_write_timeout(Some(IO_TIMEOUT))?;
        stream.write_all(&request)?;
        let _ = stream.shutdown(Shutdown::Write);
        let mut raw = Vec::new();
        stream.take(MAX_RESPONSE_BYTES).read_to_end(&mut raw)?;
        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or(NetError::Malformed)?;
        let head = std::str::from_utf8(&raw[..split]).map_err(|_| NetError::Malformed)?;
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or(NetError::Malformed)?;
        // サーバーは `Content-Length` で長さを返す（受け側と同じ形）。無い・
        // 重複・`Transfer-Encoding` あり・宣言と本文の長さ不一致は、途中切れや
        // 読み残しとみなして受け付けない（残った分が JSON として読めても使わない）。
        let mut content_length = None;
        for line in lines {
            let Some((name, value)) = line.split_once(':') else {
                continue;
            };
            if name.trim().eq_ignore_ascii_case("transfer-encoding") {
                return Err(NetError::Malformed);
            }
            if name.trim().eq_ignore_ascii_case("content-length") {
                let n = value
                    .trim()
                    .parse::<usize>()
                    .map_err(|_| NetError::Malformed)?;
                if content_length.replace(n).is_some() {
                    return Err(NetError::Malformed);
                }
            }
        }
        let need = content_length.ok_or(NetError::Malformed)?;
        let body = &raw[split + 4..];
        if body.len() != need {
            return Err(NetError::Malformed);
        }
        Ok(Response {
            status,
            body: body.to_vec(),
        })
    }
}

/// 要求ターゲットは `/` 始まりで空白・制御文字を含まない。`path` にはサーバー
/// 応答 JSON から取ったチャンネル ID 等が展開されるため、CR/LF をそのまま
/// リクエスト行へ書くとヘッダー注入になる（送信前にここで止める）。
fn check_target(path: &str) -> Result<(), NetError> {
    if !path.starts_with('/') || path.bytes().any(|b| b <= b' ' || b == 0x7f) {
        return Err(NetError::Malformed);
    }
    Ok(())
}
