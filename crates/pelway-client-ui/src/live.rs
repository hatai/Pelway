//! 実サーバーにつながる通信層（Issue 333・PR-1。`state::sample()` の置き換えの土台）。
//!
//! Slint 非依存の層: `pelway-http` のサーバー（Issue 327 の契約）と HTTP で会話して、
//! 起動時の取得・1〜2 秒ごとのポーリング・投稿・表示ユーザーの切り替え・
//! 「つながっていません」への遷移と復帰をこの層に閉じ込める（UI スレッドとは
//! 別のワーカースレッドで回す前提。Slint のウィジェットは触らない）。
//! 画面に出す行モデル（送信者名・時刻の組み立て）は PR-2 でこの層の上に足す。
//!
//! 通信の約束（Issue 327）:
//! - GET は必ずクエリ `actor`（表示ユーザーの member_id。空は 400）。
//! - POST は合成ヘッダー `x-pelway-member-id` と `x-pelway-member-kind`
//!   （`human`/`bot`/`external_agent`/`external_tool` の 4 値）で投稿者を名乗る。
//! - 接続先は環境変数 `PELWAY_SERVER_URL`（`http://127.0.0.1:{port}` の形。
//!   ポートはデモ準備キット Issue 390 の固定ポートと同じ値を渡す前提）。

use std::fmt;
use std::net::SocketAddr;

use pelway_contract::ids::{ClientMsgId, MemberId, WorkspaceId};
use pelway_contract::member::MemberKind;
use pelway_contract::message::{
    Channel, PostMessageRequest, channel_messages_path, channel_path, channels_path,
};
use pelway_contract::store::StoredMessage;

use crate::net::{Client, NetError};
use crate::realtime::fresh_client_msg_id;

/// 接続先を読む環境変数（Issue 390 の準備キットが export する値と同じ形）。
pub const SERVER_URL_ENV: &str = "PELWAY_SERVER_URL";

/// 環境変数が無いときの既定値。デモ準備キットの固定ポート（Issue 390）に合わせる。
pub const DEFAULT_SERVER_URL: &str = "http://127.0.0.1:8787";

/// 起動直後の表示ユーザー。デモ seed（Issue 326）の先頭の人（田中）。seed に居なければ
/// チャンネルの最初の人に切り替える（[`LiveSession::bootstrap`] 内）。
const DEFAULT_ACTOR: &str = "m-tanaka";

/// サーバー URL 文字列のパース失敗。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddrError(String);

impl fmt::Display for AddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid server URL: {}", self.0)
    }
}

impl std::error::Error for AddrError {}

/// `PELWAY_SERVER_URL` の値をアドレスに読み替える。`http://` 接頭辞と
/// `host:port` 素の形の両方を受け、接続先はループバックだけを許す
/// （デモのサーバーは 127.0.0.1 にだけ bind する。`localhost` の名前解決はしない）。
///
/// # Errors
/// 形が不正・ホストがループバックでない場合は [`AddrError`]。
pub fn parse_server_addr(text: &str) -> Result<SocketAddr, AddrError> {
    let trimmed = text.trim();
    let hostport = trimmed
        .strip_prefix("http://")
        .unwrap_or(trimmed)
        .trim_end_matches('/');
    let addr: SocketAddr = hostport
        .parse()
        .map_err(|_| AddrError(trimmed.to_owned()))?;
    if !addr.ip().is_loopback() {
        return Err(AddrError(trimmed.to_owned()));
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
        Err(std::env::VarError::NotUnicode(_)) => {
            Err(AddrError(format!("{SERVER_URL_ENV} is not Unicode")))
        }
    }
}

/// 通信と応答の失敗。画面では「つながっていません」の 1 状態に畳む。
#[derive(Debug)]
pub enum LiveError {
    /// TCP 接続・読み書き・応答形式の失敗。
    Net(NetError),
    /// サーバーが 2xx 以外を返した（400・404 等も含む）。
    Status(u16),
    /// 応答 JSON が契約の形でない。
    Json(serde_json::Error),
    /// チャンネルがまだ選ばれていない（`bootstrap` 前に投稿した等）。
    NoChannel,
}

impl fmt::Display for LiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Net(e) => write!(f, "{e}"),
            Self::Status(code) => write!(f, "server returned status {code}"),
            Self::Json(e) => write!(f, "unexpected response JSON: {e}"),
            Self::NoChannel => f.write_str("no channel selected"),
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
    /// 直近の一覧に出たチャンネル（レールの表示用）。
    channels: Vec<Channel>,
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
            channels: Vec::new(),
            channel: None,
            messages: Vec::new(),
            connected: false,
        }
    }

    /// 接続先を変える（テスト・復帰で同じセッションを使い回すため）。
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.client = Client::new(addr);
    }

    /// 起動時の取得: チャンネル一覧 → 最初のチャンネルの情報とメッセージ。
    /// 成功したら `connected = true`。
    ///
    /// # Errors
    /// どれかの要求が失敗したら [`LiveError`]（`connected = false`）。
    pub fn bootstrap(&mut self) -> Result<(), LiveError> {
        let channels: Vec<Channel> = self.get_json(&format!(
            "{}?actor={}",
            channels_path(&self.workspace_id),
            self.actor.as_str()
        ))?;
        self.connected = true;
        let Some(channel_id) = channels.first().map(|c| c.channel_id.clone()) else {
            // 再取得でチャンネルが無くなっていたら選択を捨てる
            // （古い channel/messages を残すと投稿・ポーリングが幽霊チャンネルを指す）。
            self.channels = channels;
            self.channel = None;
            self.messages.clear();
            return Ok(());
        };
        let channel: Channel = self.get_json(&format!(
            "{}?actor={}",
            channel_path(&channel_id),
            self.actor.as_str()
        ))?;
        // 表示ユーザーがチャンネルの人のメンバーに居なければ先頭の人に直す。
        if !channel
            .members
            .iter()
            .any(|m| m.member_id == self.actor && m.member_kind == MemberKind::Human)
            && let Some(first_human) = channel
                .members
                .iter()
                .find(|m| m.member_kind == MemberKind::Human)
        {
            self.actor = first_human.member_id.clone();
            self.actor_kind = MemberKind::Human;
        }
        let messages: Vec<StoredMessage> = self.get_json(&format!(
            "{}?actor={}",
            channel_messages_path(&channel_id),
            self.actor.as_str()
        ))?;
        self.channels = channels;
        self.channel = Some(channel);
        self.messages = messages;
        Ok(())
    }

    /// ポーリング: 表示中チャンネルのメッセージを全件取り直して置き換える
    /// （デモの履歴は小さいため差分 `after_seq` は使わない。一覧をサーバーの
    /// 正とすることで、再起動で seq や message_id が振り直された・投稿応答で
    /// カーソルが飛んだ、といった局所状態とサーバーの食い違いを構造で排除する）。
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
        let all: Vec<StoredMessage> = self.get_json(&format!(
            "{}?actor={}",
            channel_messages_path(&channel_id),
            self.actor.as_str()
        ))?;
        self.connected = true;
        self.messages = all;
        Ok(())
    }

    /// 表示ユーザーとして `body` を投稿する。成功したら保存済みメッセージを返し、
    /// 一覧にも足す。断線中は先に `bootstrap` で取り直してから投稿する
    /// （サーバー再起動で履歴が入れ替わっていても、古い一覧の上に追記しない）。
    ///
    /// # Errors
    /// チャンネル未選択・要求の失敗。
    pub fn post(&mut self, body: &str) -> Result<StoredMessage, LiveError> {
        if !self.connected {
            self.bootstrap()?;
        }
        let Some(channel_id) = self.channel.as_ref().map(|c| c.channel_id.clone()) else {
            return Err(LiveError::NoChannel);
        };
        let request = PostMessageRequest {
            client_msg_id: ClientMsgId::from(fresh_client_msg_id().as_str()),
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

    /// チャンネルが選ばれているか（`bootstrap` がまだ成功していないなら `false`。
    /// 起動時にサーバーへつながらなかった場合の再取得の判定に使う）。
    #[must_use]
    pub fn has_channel(&self) -> bool {
        self.channel.is_some()
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

    /// 新着分を seq 昇順を保つ位置に挿入する（`message_id` の重複は足さない。
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
