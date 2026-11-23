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

use std::collections::{HashMap, VecDeque};
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
/// ワーカーを占有し続けないよう、ここにも上限を持たせる。期限は各ページの
/// 開始時点で判定する（ページの途中では止めない。期限直前に始まった
/// ページはそのまま取り終え得る）。
const FETCH_DEADLINE: Duration = Duration::from_secs(60);

/// メッセージ取得 1 回で保持する履歴の量の目安（応答バイト数相当）。
/// ページを足し続けるとメモリを際限なく占有するため、保持するのは
/// 最新側からこの量までとし、超えた古い側は捨てながら末尾まで取る
/// ——上限到達をもって取得を失敗にすると、上限を超える履歴のある
/// チャンネルが二度と復帰できなくなるため
/// （対象デモの履歴はこの値を大きく下回る）。
const MAX_FETCH_BYTES: usize = 32 << 20;

/// `fetch_messages` が保持量を見積もるための 1 件あたりのサイズ。
/// 可変長フィールド（本文・各種 ID）の実長にエンベロープ分の定数を足す
/// ——本文だけ数えると、本文が空のメッセージを詰めた応答が上限をすり抜ける。
fn message_size(m: &StoredMessage) -> usize {
    m.body.len()
        + m.message_id.as_str().len()
        + m.author_id.as_str().len()
        + m.workspace_id.as_str().len()
        + m.channel_id.as_str().len()
        + m.thread_root.as_ref().map_or(0, |id| id.as_str().len())
        + m.client_msg_id.as_ref().map_or(0, |id| id.as_str().len())
        + 256
}

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
    server_addr_from(std::env::var(SERVER_URL_ENV))
}

/// [`server_addr`] の本体。環境変数の読み取り結果を値で受ける
/// （設定あり・未設定・非 Unicode の 3 分岐を、環境を変えずに単体テストで
/// 確かめられるようにする。edition 2024 では `set_var` が `unsafe`）。
fn server_addr_from(value: Result<String, std::env::VarError>) -> Result<SocketAddr, AddrError> {
    match value {
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
    /// 投稿を受け付けた時点の宛先・名義と食い違った（断線からの再接続で
    /// `bootstrap` がチャンネルや表示ユーザーを選び直した場合、または同じ
    /// `client_msg_id` の再送を初回と違う宛先・名義で送ろうとした場合。
    /// 別チャンネル・別人名義に届かないよう投稿を止めた）。
    StaleTarget,
    /// サーバーの応答が取得の前提を満たさない（満杯のページがカーソルを
    /// 進めない・満杯ページの末尾が `seq=0`・ページが要求件数を超える・
    /// 取得 1 呼び出しの期限に達した）。文言に応答内容は含めない。
    Protocol(&'static str),
}

impl fmt::Display for LiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Net(e) => write!(f, "{e}"),
            Self::Status(code) => write!(f, "server returned status {code}"),
            Self::Json(e) => {
                write!(f, "unexpected response JSON at {}:{}", e.line(), e.column())
            }
            Self::NoChannel => f.write_str("no channel selected"),
            Self::StaleTarget => f.write_str("post target or actor changed"),
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
    /// `fetch_messages` の中断した取得の途中経過（チャンネル・名義が同じ
    /// 呼び出しでのみ続きから再開する）。
    fetch_progress: Option<FetchProgress>,
    /// 論理投稿（`client_msg_id`）ごとに初回の宛先・名義を記録する。
    /// 応答を受け取れず再送する間に表示ユーザーやチャンネルが切り替わっても、
    /// 同じ ID を別の宛先・名義で送らないため（サーバーは
    /// workspace×author×client_msg_id で冪等に扱う——名義が変わると同じ ID
    /// でも別の投稿として保存される）。
    post_targets: HashMap<ClientMsgId, (ChannelId, MemberId)>,
}

/// `fetch_messages` が呼び出しの途中で中断したときに残す途中経過。
/// 期限や通信の失敗で打ち切られても、同じチャンネル・名義の次の取得は
/// カーソルの続きから再開する——毎回先頭から取り直すと、履歴が長い
/// チャンネルでは 1 呼び出しの期限の中でいつまでも末尾に届かず、
/// 切断状態から復帰できなくなるため。
struct FetchProgress {
    channel_id: ChannelId,
    actor: MemberId,
    after: Option<u64>,
    page_limit: u32,
    out: VecDeque<StoredMessage>,
    out_bytes: usize,
}

impl FetchProgress {
    fn new(channel_id: &ChannelId, actor: &MemberId) -> Self {
        Self {
            channel_id: channel_id.clone(),
            actor: actor.clone(),
            after: None,
            page_limit: MESSAGE_PAGE_LIMIT,
            out: VecDeque::new(),
            out_bytes: 0,
        }
    }
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
            fetch_progress: None,
            post_targets: HashMap::new(),
        }
    }

    /// 接続先を変える（テスト・復帰で同じセッションを使い回すため）。
    pub fn set_addr(&mut self, addr: SocketAddr) {
        self.client = Client::new(addr);
        // 別の接続先では採番が食い違うため、前の接続先の取得カーソルは捨てる。
        self.fetch_progress = None;
    }

    /// `path` に表示ユーザーの `actor` クエリを付けた要求ターゲット。
    fn actor_path(&self, path: &str) -> String {
        self.actor_path_for(&self.actor, path)
    }

    /// `path` に指定した `actor` のクエリを付けた要求ターゲット。
    /// 値はパーセントエンコードする（unreserved: `[A-Za-z0-9-._~]` 以外は %XX。
    /// member_id 以外の文字が混入してもクエリやリクエスト行を壊さない）。
    /// `bootstrap` が名義を選び直した直後（`self.actor` 反映前）にも、
    /// 取得に使う名義を渡せるよう引数で受ける。
    fn actor_path_for(&self, actor: &MemberId, path: &str) -> String {
        let mut out = String::with_capacity(path.len() + actor.as_str().len() + 7);
        out.push_str(path);
        out.push_str("?actor=");
        for b in actor.as_str().bytes() {
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
        // 履歴の取得は選び直した名義で行う（GET の actor をサーバーが記録する
        // ため、表示ユーザーと取得名義を食い違わせない）。
        let messages = self.fetch_messages(&channel_id, &actor)?;
        // 取得途中の失敗（`fetch_messages` 内で縮退が起きた場合も含む）で
        // `connected` が false に戻されていることがあるため、ここでも一緒に
        // 立て直す。全部そろってから反映する原子性の一部。
        self.connected = true;
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
        let actor = self.actor.clone();
        let all = self.fetch_messages(&channel_id, &actor)?;
        self.connected = true;
        self.messages = all;
        Ok(())
    }

    /// メッセージ一覧を `limit` + `after_seq` のページングで取る。
    /// 満杯のページが返ったら最後の seq の続きを要求し、短いページで終わる。
    /// 件数は応答のバイト数を抑えられないため、応答が上限超過
    /// （[`crate::net::NetError::TooLarge`]）なら同じカーソルから件数を半減して
    /// 再試行する（1 件でも超える場合だけエラー）。1 回の取得全体には
    /// [`FETCH_DEADLINE`] の上限を掛け、満杯なのにカーソルが進まない応答
    /// （サーバーが `after_seq` を見ない等）・満杯ページの末尾が `seq=0`・
    /// 要求した件数を超えるページは [`LiveError::Protocol`] にする。
    /// 保持する履歴は最新側から [`MAX_FETCH_BYTES`] 相当まで。それを超えた
    /// 古い側は捨てながら末尾まで取り続ける——上限到達をもって失敗にすると、
    /// 予算を超える履歴のあるチャンネルが二度と復帰できなくなるため
    /// （一覧の `limit` は先頭からの昇順で、末尾側の直接取得はできない）。
    /// `actor` は履歴取得に使う名義（`bootstrap` が選び直した名義を渡す）。
    /// 期限や通信の失敗で中断しても、同じチャンネル・名義の次の呼び出しは
    /// カーソルの続きから再開する（`FetchProgress`）。毎回先頭から取り直すと、
    /// 履歴が長いチャンネルでは 1 呼び出しの期限の中で末尾に届かず、
    /// 切断状態から復帰できなくなるため。
    fn fetch_messages(
        &mut self,
        channel_id: &ChannelId,
        actor: &MemberId,
    ) -> Result<Vec<StoredMessage>, LiveError> {
        let mut p = match self.fetch_progress.take() {
            Some(p) if p.channel_id == *channel_id && p.actor == *actor => p,
            _ => FetchProgress::new(channel_id, actor),
        };
        // 保存したカーソルが今の履歴に属することを確かめる（同じアドレスで
        // サーバーが再起動し、同じチャンネルに別の履歴が入っていると、
        // 古い seq の続きから取っても空ページが返り、古い一覧が残って
        // 新しい履歴が欠ける）。カーソル位置の 1 件を probe して、保持中の
        // 末尾と一致しなければ先頭から取り直す。
        if p.after.is_some() {
            match self.cursor_belongs(&p) {
                Ok(true) => {}
                Ok(false) => {
                    p = FetchProgress::new(channel_id, actor);
                }
                Err(e) => {
                    self.fetch_progress = Some(p);
                    return Err(e);
                }
            }
        }
        let result = self.fetch_pages(&mut p);
        if result.is_err() {
            // 中断した分は次の取得に引き継ぐ（成功したときは残さない）。
            self.fetch_progress = Some(p);
        }
        result
    }

    fn fetch_pages(&mut self, p: &mut FetchProgress) -> Result<Vec<StoredMessage>, LiveError> {
        let deadline = Instant::now() + FETCH_DEADLINE;
        let base = channel_messages_path(&p.channel_id);
        loop {
            if Instant::now() >= deadline {
                return Err(self.protocol_err("message fetch deadline"));
            }
            let mut path = self.actor_path_for(&p.actor, &base);
            path.push_str(&format!("&limit={}", p.page_limit));
            if let Some(s) = p.after {
                path.push_str(&format!("&after_seq={s}"));
            }
            let resp = match self.get_response(&path) {
                Err(LiveError::Net(NetError::TooLarge)) if p.page_limit > 1 => {
                    p.page_limit = (p.page_limit / 2).max(1);
                    continue;
                }
                result => result?,
            };
            let page: Vec<StoredMessage> = self.parse_json(&resp.body)?;
            if page.len() > p.page_limit as usize {
                return Err(self.protocol_err("message page exceeds requested limit"));
            }
            let full = page.len() >= p.page_limit as usize;
            let last_seq = page.last().map(|m| m.seq);
            // 途中経過に足す前にページを検査する——拒否したページの内容が
            // 残ると、次の取得がそれを含んだ結果を返してしまう。`seq=0`
            // のカーソルは probe の `after_seq = カーソル-1` を組めないため
            // 採番違反として拒否する（保存してしまうと以後の取得が必ず
            // カーソル検査で失敗し、復旧できなくなる）。
            let advance = match (full, last_seq) {
                (true, Some(0)) => return Err(self.protocol_err("message seq is zero")),
                (true, Some(s)) if p.after.is_none_or(|a| s > a) => Some(s),
                (true, _) => return Err(self.protocol_err("message page did not advance")),
                _ => None,
            };
            for m in page {
                p.out_bytes += message_size(&m);
                p.out.push_back(m);
            }
            while p.out_bytes > MAX_FETCH_BYTES
                && let Some(m) = p.out.pop_front()
            {
                p.out_bytes -= message_size(&m);
            }
            match advance {
                Some(s) => p.after = Some(s),
                // `advance` が無いのは短いページ（取得の終わり）。満杯で
                // 進まないページは上の検査で既に `Protocol` を返している。
                None => return Ok(p.out.drain(..).collect()),
            }
        }
    }

    /// 保存した取得カーソル（`p.after`）が今の履歴に属するかを probe で確かめる。
    /// `after_seq = カーソル-1, limit = 1` でカーソル位置の 1 件を取り、保持中の
    /// 末尾と全フィールドが一致すれば同じ履歴とみなす。サーバーが再起動して
    /// 採番をやり直すと、別の投稿に同じ `seq` と `message_id` が付き得るため、
    /// 識別子だけでなく本文・投稿者・保存時刻まで一致を要求する。
    /// 保持中の一覧が空なら比較できないため `false` を返す。
    fn cursor_belongs(&mut self, p: &FetchProgress) -> Result<bool, LiveError> {
        let (Some(after), Some(tail)) = (p.after, p.out.back()) else {
            return Ok(false);
        };
        let Some(prev) = after.checked_sub(1) else {
            // 現行のページ検査ではカーソル 0 は保存されない（満杯ページの
            // 末尾が `seq=0` なら拒否する）。到達しない前提の防御として、
            // もしあれば破棄して先頭から取り直す。
            return Ok(false);
        };
        let mut path = self.actor_path_for(&p.actor, &channel_messages_path(&p.channel_id));
        path.push_str(&format!("&limit=1&after_seq={prev}"));
        let resp = self.get_response(&path)?;
        let probe: Vec<StoredMessage> = self.parse_json(&resp.body)?;
        Ok(probe.first().is_some_and(|m| m == tail))
    }

    /// 投稿の冪等キー（`client_msg_id`）を新規発行する。応答の受け取りに
    /// 失敗した投稿を再送したい呼び出し元は、この ID を保持して
    /// [`Self::post_with_id`] に渡す。同じ論理投稿の再送は同じ ID で送る——
    /// サーバーは `workspace`×`author`×`client_msg_id` で冪等に扱い、
    /// 同じ ID の再投稿は保存済みのものを返す（内容が違えば 409）。
    /// `post` は呼び出しごとに新しい ID を発行するため、再送しても
    /// 冪等にはならない。
    #[must_use]
    pub fn new_client_msg_id() -> ClientMsgId {
        ClientMsgId::from(uuid::Uuid::new_v4().to_string().as_str())
    }

    /// 表示ユーザーとして `body` を投稿する。成功したら保存済みメッセージを返し、
    /// 一覧にも足す。断線中は先に `bootstrap` で取り直してから投稿する
    /// （サーバー再起動で履歴が入れ替わっていても、古い一覧の上に追記しない）。
    /// 再接続で `bootstrap` がチャンネルの並びやメンバーを取り直し、受け付け
    /// 時点の宛先・名義と食い違ったら [`LiveError::StaleTarget`] で止める
    /// （同じ操作が別のチャンネル・別人名義に届くのを構造で防ぐ）。
    /// 冪等キーは呼び出しごとに [`Self::new_client_msg_id`] で発行するため、
    /// `post` の再送は冪等にならない——応答を受け取れなかった投稿を再送
    /// したい呼び出し元は [`Self::post_with_id`] を使う。
    ///
    /// # Errors
    /// チャンネル未選択・宛先の食い違い・要求の失敗。
    pub fn post(&mut self, body: &str) -> Result<StoredMessage, LiveError> {
        self.post_with_id(body, Self::new_client_msg_id())
    }

    /// [`Self::post`] に冪等キーを指定する版。呼び出し元が保持した
    /// `client_msg_id` で送る（応答を受け取れなかった投稿の再送用）。
    /// 初めて受け付けた ID にはその時点の宛先・名義を記録し、同じ ID の
    /// 再送はその組と今の宛先・名義が一致するときだけ送る——応答の喪失後に
    /// 表示ユーザーやチャンネルが切り替わっていても、同じ ID を別人名義・
    /// 別チャンネルには送らない（一致しなければ [`LiveError::StaleTarget`]）。
    /// 宛先・名義の確定と食い違い検査は `post` と同じ。
    ///
    /// # Errors
    /// チャンネル未選択・宛先の食い違い・要求の失敗。
    pub fn post_with_id(
        &mut self,
        body: &str,
        client_msg_id: ClientMsgId,
    ) -> Result<StoredMessage, LiveError> {
        // 受け付け時点の宛先（チャンネル）と名義（表示ユーザー）を保持する。
        // 宛先が未選択ならここで止める——通信を始めて `bootstrap` が選び直した
        // チャンネルへ（表示ユーザーが見ていない先へ）送ることはしない。
        let Some((want_channel, want_actor)) = self
            .channel
            .as_ref()
            .map(|c| (c.channel_id.clone(), self.actor.clone()))
        else {
            return Err(LiveError::NoChannel);
        };
        if !self.connected {
            self.bootstrap()?;
        }
        let after = self.channel.as_ref().map(|c| c.channel_id.clone());
        if after.as_ref() != Some(&want_channel) || self.actor != want_actor {
            return Err(LiveError::StaleTarget);
        }
        // 初回の宛先・名義をこの ID に記録する。再送は同じ組でだけ送る
        // （応答を受け取れず再送する間に名義・チャンネルが切り替わっても、
        // 同じ ID が別人・別チャンネルの投稿として保存されないよう送信前に
        // 止める。記録は送信前に済ませる——応答が途切れても残る）。
        match self.post_targets.get(&client_msg_id).cloned() {
            Some((c, a)) if c == want_channel && a == want_actor => {}
            Some(_) => return Err(LiveError::StaleTarget),
            None => {
                self.post_targets
                    .insert(client_msg_id.clone(), (want_channel.clone(), want_actor));
            }
        }
        let channel_id = want_channel;
        let request = PostMessageRequest {
            client_msg_id,
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
    /// （`message_id` の重複は足さない。呼び出しは `post` の応答 1 件だけで、
    /// `poll` は一覧の全件置き換えなのでここを通らない。サーバーはチャンネル内で
    /// seq を単調に採番するため通常は末尾への追加で済むが、保存部が
    /// 再初期化されて採番が振り直された場合など、一覧と応答の seq が
    /// 交差し得る経路でも順序が崩れないよう位置に挿入する）。
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

    /// GET で応答をそのまま取る。200 以外・通信の失敗は `connected = false`
    /// にしてから [`LiveError`] を返す。縮退の再試行の判断に使う
    /// （`fetch_messages` は [`crate::net::NetError::TooLarge`] を見て件数を
    /// 半減する）ため、JSON 化前の [`crate::net::Response`] を返す。
    fn get_response(&mut self, path: &str) -> Result<crate::net::Response, LiveError> {
        let resp = self.client.get(path).map_err(|e| {
            self.connected = false;
            LiveError::Net(e)
        })?;
        if resp.status != 200 {
            self.connected = false;
            return Err(LiveError::Status(resp.status));
        }
        Ok(resp)
    }

    /// [`LiveError::Protocol`] を返す前に接続状態を下げる。`decode`/`parse_json`
    /// を通らない取得条件の失敗（取得全体の期限・ページ形状の
    /// 不整合）でも、操作の最終結果として `connected = false` に揃える
    /// ——通信自体は正常でも、取得が打ち切られたなら「つながっていません」
    /// 側に倒す。
    fn protocol_err(&mut self, msg: &'static str) -> LiveError {
        self.connected = false;
        LiveError::Protocol(msg)
    }

    /// 応答 body を JSON に読む。形式の失敗は `connected = false` にしてから
    /// [`LiveError`] を返す。
    fn parse_json<T: serde::de::DeserializeOwned>(&mut self, body: &[u8]) -> Result<T, LiveError> {
        serde_json::from_slice(body).map_err(|e| {
            self.connected = false;
            LiveError::Json(e)
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
        self.parse_json(&resp.body)
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

    #[test]
    fn server_addr_from_covers_env_read_branches() {
        // 設定あり → その値をパースする。
        assert_eq!(
            server_addr_from(Ok("http://127.0.0.1:9999".to_owned()))
                .expect("valid value parses")
                .port(),
            9999
        );
        // 未設定 → 既定値。
        assert_eq!(
            server_addr_from(Err(std::env::VarError::NotPresent))
                .expect("default applies when unset"),
            parse_server_addr(DEFAULT_SERVER_URL).expect("default URL parses")
        );
        // 設定ありだが非 Unicode → エラー（既定値には戻らない）。
        assert!(
            server_addr_from(Err(std::env::VarError::NotUnicode(
                std::ffi::OsString::from("not-utf8")
            )))
            .is_err()
        );
    }
}
