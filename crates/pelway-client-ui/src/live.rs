//! 実サーバーにつながる通信層（Issue 333・PR-1。`state::sample()` の置き換えの土台）。
//!
//! Slint 非依存の層: `pelway-http` のサーバー（Issue 327 の契約）と HTTP で会話して、
//! 起動時の取得・ポーリング・投稿・表示ユーザーの切り替え・
//! 「つながっていません」への遷移と復帰をこの層に閉じ込める（UI スレッドとは
//! 別のワーカースレッドで回す前提。Slint のウィジェットは触らない）。
//! `poll()` は呼び出しごとに 1 回だけ取り直す。呼び出す間隔（想定は
//! 1〜2 秒ごと）はこの層の外が決める。
//! 画面に出す行モデル（送信者名・時刻の組み立て）は PR-2 でこの層の上に足す。
//!
//! 通信の約束（Issue 327）:
//! - GET は必ずクエリ `actor`（表示ユーザーの member_id。空は 400）。
//! - POST は合成ヘッダー `x-pelway-member-id` と `x-pelway-member-kind`
//!   （`human`/`bot`/`external_agent`/`external_tool` の 4 値）で投稿者を名乗る。
//! - 応答の成功条件: GET は 200 のみ、POST は 2xx すべて。
//! - 接続先は環境変数 `PELWAY_SERVER_URL`（`http://127.0.0.1:{port}` の形。
//!   ポートはデモ準備キット Issue 390 の固定ポートと同じ値を渡す前提）。

use std::fmt;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

use pelway_contract::ids::{ChannelId, ClientMsgId, MemberId, WorkspaceId};
use pelway_contract::member::MemberKind;
use pelway_contract::message::{
    Channel, PostMessageRequest, channel_messages_path, channel_path, channels_path,
};
use pelway_contract::store::StoredMessage;

use crate::net::{Client, NetError};

/// 接続先を読む環境変数（Issue 390 の準備キットが export する値と同じ形）。
pub const SERVER_URL_ENV: &str = "PELWAY_SERVER_URL";

/// 環境変数が無いときの既定値。デモ用の暫定値——Issue 390 の起動スクリプトの
/// 固定ポートは未確定のため、確定したら同じ値に揃える。
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";

/// 起動直後の表示ユーザー。デモ seed（Issue 326）の先頭の人（田中）。seed に居なければ
/// チャンネルの最初の人に切り替える（[`LiveSession::bootstrap`] 内）。
const DEFAULT_ACTOR: &str = "m-tanaka";

/// メッセージ取得 1 ページの初期件数。サーバーは `limit` 件までを seq 昇順で
/// 返す（`limit` 未指定なら全件）。応答が `net` の上限（1 MiB）を超えたときは
/// 同じカーソルから件数を半減して再試行する（件数だけでは応答バイト数を
/// 抑えられないため——本文 1 件が最大 64 KiB の要求をサーバーは受け付ける）。
pub const MESSAGE_PAGE_LIMIT: u32 = 200;

/// メッセージ取得 1 回（`bootstrap`/`poll` の中の `fetch_messages`）全体の期限。
/// 各ページの通信は別途 `net` の期限が掛かるが、ページを繰り返すループ全体が
/// ワーカーを占有し続けないよう、ここにも上限を持たせる。
const FETCH_DEADLINE: Duration = Duration::from_secs(60);

/// メッセージ取得 1 回で蓄積する本文バイト数の上限。ページを足し続けると
/// メモリを際限なく占有するため、本文の合計サイズで打ち切る
/// （対象デモの履歴はこの値を大きく下回る）。
const MAX_FETCH_BODY_BYTES: usize = 32 << 20;

/// サーバー URL 文字列のパース失敗。文言に入力値は含めない
/// （接続先として拒否した文字列をエラー出力へ持ち出さない）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrError(&'static str);

impl fmt::Display for AddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid server URL ({})", self.0)
    }
}

impl std::error::Error for AddrError {}

/// `PELWAY_SERVER_URL` の値をアドレスに読み替える。`http://` 接頭辞と
/// `host:port` 素の形の両方を受け、接続先は `127.0.0.1` だけを許す
/// （デモのサーバーは 127.0.0.1 にだけ bind する——サーバーの `serve_inner` と
/// 同じ判定で、`::1` や `127.0.0.2` も拒否。`localhost` の名前解決はしない）。
///
/// # Errors
/// 形が不正・ホストが `127.0.0.1` でない場合は [`AddrError`]。
pub fn parse_server_addr(text: &str) -> Result<SocketAddr, AddrError> {
    let trimmed = text.trim();
    let hostport = trimmed
        .strip_prefix("http://")
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    let addr: SocketAddr = hostport.parse().map_err(|_| AddrError("malformed"))?;
    if addr.ip() != std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST) {
        return Err(AddrError("not 127.0.0.1"));
    }
    Ok(addr)
}

/// 環境変数 `PELWAY_SERVER_URL`（未設定なら [`DEFAULT_SERVER_URL`]）から接続先を作る。
///
/// # Errors
/// 変数が設定されているのに値が [`parse_server_addr`] で読めない形（空・空白・
/// 非 Unicode を含む）なら [`AddrError`]。既定値に戻すのは「未設定」のときだけ
/// （設定ミスをデフォルト接続で取りこぼさない）。
pub fn server_addr() -> Result<SocketAddr, AddrError> {
    match std::env::var(SERVER_URL_ENV) {
        Ok(value) => parse_server_addr(&value),
        Err(std::env::VarError::NotPresent) => parse_server_addr(DEFAULT_SERVER_URL),
        Err(std::env::VarError::NotUnicode(_)) => Err(AddrError("not Unicode")),
    }
}

/// 通信と応答の失敗。画面では「つながっていません」の 1 状態に畳む。
#[derive(Debug)]
pub enum LiveError {
    /// TCP 接続・読み書き・応答形式の失敗。
    Net(NetError),
    /// サーバーが成功条件を返さなかった（GET は 200 のみ・POST は 2xx すべて。
    /// 400・404 等も含む）。
    Status(u16),
    /// 応答 JSON が契約の形でない。
    Json(serde_json::Error),
    /// チャンネルがまだ選ばれていない（`bootstrap` 前に投稿した等）。
    NoChannel,
    /// 断線からの再接続で `bootstrap` がチャンネルや表示ユーザーを選び直し、
    /// 投稿を受け付けた時点の宛先・名義と食い違った（別チャンネル・別人名義に
    /// 送らないため投稿を止めた）。
    StaleTarget,
    /// サーバーの応答が取得の前提を満たさない（ページが進まない・履歴が
    /// クライアント側の蓄積上限や期限を超える）。文言に応答内容は含めない。
    Protocol(&'static str),
}

impl fmt::Display for LiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Net(e) => write!(f, "{e}"),
            Self::Status(code) => write!(f, "server returned status {code}"),
            Self::Json(e) => write!(f, "unexpected response JSON: {e}"),
            Self::NoChannel => f.write_str("no channel selected"),
            Self::StaleTarget => f.write_str("post target changed while reconnecting"),
            Self::Protocol(msg) => write!(f, "unexpected server response: {msg}"),
        }
    }
}

impl std::error::Error for LiveError {}

/// サーバーと会話してチャンネル・メッセージ・表示ユーザーを持つセッション。
/// 取得・投稿・切替・接続状態はここに閉じ込める（`Send`。ワーカースレッドに
/// 置いて UI と mpsc でやり取りする）。
pub struct LiveSession {
    client: Client,
    workspace_id: WorkspaceId,
    /// 表示ユーザー（GET の `actor`・POST の `x-pelway-member-id`）。
    actor: MemberId,
    actor_kind: MemberKind,
    /// 表示中のチャンネル（最初に見つかったものを選ぶ）。
    channel: Option<Channel>,
    /// 表示中のチャンネルのメッセージ（seq 昇順）。
    messages: Vec<StoredMessage>,
    /// 直近の要求が成功しているか。失敗したら「つながっていません」。
    connected: bool,
}

impl LiveSession {
    /// `addr` のサーバー・`workspace_id` の workspace に接続するセッション。
    /// 表示ユーザーは [`DEFAULT_ACTOR`]。`bootstrap` でメンバー表を見て直す。
    #[must_use]
    pub fn new(addr: SocketAddr, workspace_id: WorkspaceId) -> Self {
        Self {
            client: Client::new(addr),
            workspace_id,
            actor: MemberId::from(DEFAULT_ACTOR),
            actor_kind: MemberKind::Human,
            channel: None,
            messages: Vec::new(),
            connected: false,
        }
    }

    /// 接続先を変える（テスト・復帰で同じセッションを使い回すため）。
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.client = Client::new(addr);
    }

    /// `path` に表示ユーザーの `actor` クエリを付けた要求ターゲット。
    /// 値はパーセントエンコードする（unreserved: `[A-Za-z0-9-._~]` 以外は %XX。
    /// member_id 以外の文字が混入してもクエリやリクエスト行を壊さない）。
    fn actor_path(&self, path: &str) -> String {
        let mut out = String::with_capacity(path.len() + self.actor.as_str().len() + 7);
        out.push_str(path);
        out.push_str("?actor=");
        for b in self.actor.as_str().bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
                out.push(b as char);
            } else {
                out.push_str(&format!("%{b:02X}"));
            }
        }
        out
    }

    /// 起動時の取得: チャンネル一覧 → 最初のチャンネルの情報とメッセージ。
    /// 成功したら `connected = true`。
    ///
    /// # Errors
    /// どれかの要求が失敗したら [`LiveError`]（`connected = false`）。
    pub fn bootstrap(&mut self) -> Result<(), LiveError> {
        let channels: Vec<Channel> =
            self.get_json(&self.actor_path(&channels_path(&self.workspace_id)))?;
        self.connected = true;
        let Some(channel_id) = channels.first().map(|c| c.channel_id.clone()) else {
            // 再取得でチャンネルが無くなっていたら選択を捨てる
            // （古い channel/messages を残すと投稿・ポーリングが幽霊チャンネルを指す）。
            self.channel = None;
            self.messages.clear();
            return Ok(());
        };
        let channel: Channel = self.get_json(&self.actor_path(&channel_path(&channel_id)))?;
        // 表示ユーザーがチャンネルの人のメンバーに居なければ先頭の人に直す。
        // ここでは次の名義を計算するだけで、メッセージの取得が成功するまで
        // セッションには反映しない（途中の失敗でチャンネルと名義の
        // 組み合わせが壊れたまま残らないよう、全件そろってから一緒に更新する）。
        let mut actor = self.actor.clone();
        let mut actor_kind = self.actor_kind;
        if !channel
            .members
            .iter()
            .any(|m| m.member_id == actor && m.member_kind == MemberKind::Human)
            && let Some(first_human) = channel
                .members
                .iter()
                .find(|m| m.member_kind == MemberKind::Human)
        {
            actor = first_human.member_id.clone();
            actor_kind = MemberKind::Human;
        }
        let messages = self.fetch_messages(&channel_id)?;
        self.channel = Some(channel);
        self.messages = messages;
        self.actor = actor;
        self.actor_kind = actor_kind;
        Ok(())
    }

    /// ポーリング: 表示中チャンネルのメッセージをページングで全件取り直して
    /// 置き換える（差分取得はせず、一覧をサーバーの正とすることで、再起動で
    /// seq や message_id が振り直された・投稿応答でカーソルが飛んだ、といった
    /// 局所状態とサーバーの食い違いを構造で排除する）。
    /// 失敗したら `connected = false`、成功したら `true`（復帰すると通常表示に戻る）。
    /// 断線からの復帰・チャンネル未選択では `bootstrap` で取り直す。
    ///
    /// # Errors
    /// 要求の失敗。
    pub fn poll(&mut self) -> Result<(), LiveError> {
        if !self.connected {
            return self.bootstrap();
        }
        let Some(channel_id) = self.channel.as_ref().map(|c| c.channel_id.clone()) else {
            return self.bootstrap();
        };
        let all = self.fetch_messages(&channel_id)?;
        self.connected = true;
        self.messages = all;
        Ok(())
    }

    /// メッセージ一覧を `limit` + `after_seq` のページングで全件取る。
    /// 満杯のページが返ったら最後の seq の続きを要求し、短いページで終わる。
    /// 件数は応答のバイト数を抑えられないため、応答が上限超過
    /// （[`crate::net::NetError::TooLarge`]）なら同じカーソルから件数を半減して
    /// 再試行する（1 件でも超える場合だけエラー）。1 回の取得全体には
    /// [`FETCH_DEADLINE`] と蓄積 [`MAX_FETCH_BODY_BYTES`] の上限を掛け、
    /// 満杯なのにカーソルが進まない応答（サーバーが `after_seq` を見ない等）は
    /// 部分結果を成功とせず [`LiveError::Protocol`] にする。
    fn fetch_messages(&mut self, channel_id: &ChannelId) -> Result<Vec<StoredMessage>, LiveError> {
        let deadline = Instant::now() + FETCH_DEADLINE;
        let base = channel_messages_path(channel_id);
        let mut out = Vec::new();
        let mut after: Option<u64> = None;
        let mut page_limit = MESSAGE_PAGE_LIMIT;
        let mut body_bytes: usize = 0;
        loop {
            if Instant::now() >= deadline {
                return Err(LiveError::Protocol("message fetch deadline"));
            }
            let mut path = self.actor_path(&base);
            path.push_str(&format!("&limit={page_limit}"));
            if let Some(s) = after {
                path.push_str(&format!("&after_seq={s}"));
            }
            let page: Vec<StoredMessage> = match self.get_json(&path) {
                Err(LiveError::Net(NetError::TooLarge)) if page_limit > 1 => {
                    page_limit = (page_limit / 2).max(1);
                    continue;
                }
                result => result?,
            };
            body_bytes += page.iter().map(|m| m.body.len()).sum::<usize>();
            if body_bytes > MAX_FETCH_BODY_BYTES {
                return Err(LiveError::Protocol("message history exceeds fetch limit"));
            }
            let full = page.len() >= page_limit as usize;
            let last_seq = page.last().map(|m| m.seq);
            out.extend(page);
            match (full, last_seq) {
                (true, Some(s)) if after.is_none_or(|a| s > a) => after = Some(s),
                (true, _) => return Err(LiveError::Protocol("message page did not advance")),
                _ => return Ok(out),
            }
        }
    }

    /// 表示ユーザーとして `body` を投稿する。成功したら保存済みメッセージを返し、
    /// 一覧にも足す。断線中は先に `bootstrap` で取り直してから投稿する
    /// （サーバー再起動で履歴が入れ替わっていても、古い一覧の上に追記しない）。
    /// 再接続で `bootstrap` がチャンネルの並びやメンバーを取り直し、受け付け
    /// 時点の宛先・名義と食い違ったら [`LiveError::StaleTarget`] で止める
    /// （同じ操作が別のチャンネル・別人名義に届くのを構造で防ぐ）。
    ///
    /// # Errors
    /// チャンネル未選択・宛先の食い違い・要求の失敗。
    pub fn post(&mut self, body: &str) -> Result<StoredMessage, LiveError> {
        // 受け付け時点の宛先（チャンネル）と名義（表示ユーザー）を保持する。
        let want = self
            .channel
            .as_ref()
            .map(|c| (c.channel_id.clone(), self.actor.clone()));
        if !self.connected {
            self.bootstrap()?;
        }
        let Some(channel_id) = self.channel.as_ref().map(|c| c.channel_id.clone()) else {
            return Err(LiveError::NoChannel);
        };
        if let Some((want_channel, want_actor)) = want
            && (channel_id != want_channel || self.actor != want_actor)
        {
            return Err(LiveError::StaleTarget);
        }
        let request = PostMessageRequest {
            client_msg_id: ClientMsgId::from(uuid::Uuid::new_v4().to_string().as_str()),
            body: body.to_owned(),
            thread_root: None,
        };
        let json = serde_json::to_vec(&request).map_err(LiveError::Json)?;
        let path = channel_messages_path(&channel_id);
        let member_id = self.actor.as_str().to_owned();
        let member_kind = self.actor_kind.as_str();
        let stored: StoredMessage = self.post_json(
            &path,
            &[
                ("x-pelway-member-id", member_id.as_str()),
                ("x-pelway-member-kind", member_kind),
            ],
            &json,
        )?;
        self.connected = true;
        self.append_new(std::iter::once(stored.clone()).collect());
        Ok(stored)
    }

    /// 表示ユーザーを人のメンバーに切り替える（Issue 392 の切替 UI が呼ぶ）。
    /// 戻り値は切り替えられたか（bot・未知のメンバーは `false` で現在のまま）。
    pub fn switch_actor(&mut self, member_id: &str) -> bool {
        let Some(channel) = &self.channel else {
            return false;
        };
        let Some(member) = channel
            .members
            .iter()
            .find(|m| m.member_id.as_str() == member_id)
        else {
            return false;
        };
        if member.member_kind != MemberKind::Human {
            return false;
        }
        self.actor = member.member_id.clone();
        self.actor_kind = MemberKind::Human;
        true
    }

    /// 直近の要求が成功しているか（`false` → 画面は「つながっていません」）。
    #[must_use]
    pub fn connected(&self) -> bool {
        self.connected
    }

    /// 表示中のチャンネル（`bootstrap` がまだ成功していないなら `None`）。
    /// ワイヤ上の型のまま返す。表示用の行への組み立ては PR-2 で足す。
    #[must_use]
    pub fn channel(&self) -> Option<&Channel> {
        self.channel.as_ref()
    }

    /// 表示中のチャンネルのメッセージ（seq 昇順・ワイヤ上の型のまま）。
    #[must_use]
    pub fn messages(&self) -> &[StoredMessage] {
        &self.messages
    }

    /// 現在の表示ユーザー（GET の `actor`・POST の `x-pelway-member-id` に使う値）。
    #[must_use]
    pub fn actor(&self) -> &MemberId {
        &self.actor
    }

    /// 投稿応答の保存済みメッセージを seq 昇順を保つ位置に挿入する
    /// （`message_id` の重複は足さない。`poll` は一覧の取り直しでここを通らない。
    /// 投稿応答を先に足しているとポーリングでそれより小さい seq が届き得るため、
    /// 末尾への追加だけだと順序が崩れる）。
    fn append_new(&mut self, new: Vec<StoredMessage>) {
        for m in new {
            if self.messages.iter().any(|e| e.message_id == m.message_id) {
                continue;
            }
            let pos = self.messages.partition_point(|e| e.seq <= m.seq);
            self.messages.insert(pos, m);
        }
    }

    /// GET で JSON を取る。失敗したら `connected = false`。
    fn get_json<T: serde::de::DeserializeOwned>(&mut self, path: &str) -> Result<T, LiveError> {
        self.decode(self.client.get(path), |s| s == 200)
    }

    /// POST（JSON）で応答 JSON を取る。失敗したら `connected = false`。
    fn post_json<T: serde::de::DeserializeOwned>(
        &mut self,
        path: &str,
        headers: &[(&str, &str)],
        body: &[u8],
    ) -> Result<T, LiveError> {
        self.decode(self.client.post(path, headers, body), |s| {
            (200..300).contains(&s)
        })
    }

    /// HTTP 応答を JSON に読む。`accepted` を満たさないステータス・通信・
    /// 形式の失敗はすべて `connected = false` にしてから [`LiveError`] を返す。
    fn decode<T: serde::de::DeserializeOwned>(
        &mut self,
        resp: Result<crate::net::Response, NetError>,
        accepted: impl FnOnce(u16) -> bool,
    ) -> Result<T, LiveError> {
        let resp = resp.map_err(|e| {
            self.connected = false;
            LiveError::Net(e)
        })?;
        if !accepted(resp.status) {
            self.connected = false;
            return Err(LiveError::Status(resp.status));
        }
        serde_json::from_slice(&resp.body).map_err(|e| {
            self.connected = false;
            LiveError::Json(e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_path_percent_encodes_query_value() {
        let mut s = LiveSession::new("127.0.0.1:1".parse().unwrap(), "w".into());
        s.actor = MemberId::from("m a?&=b");
        assert_eq!(s.actor_path("/p"), "/p?actor=m%20a%3F%26%3Db");
    }
}
