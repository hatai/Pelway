//! 画面の表示用の状態（Slint 非依存）と合成データ。
//!
//! 部品の並びはデスクトップのモック（`docs/ui-mocks/csv.html`・`csv_dark.html`、
//! 試作モック・暫定承認 11/8・最終承認は未）を参考にした。モックにない部品は
//! 足さない。色の値とフォントの URL は写していない（トークン名だけを使う）。
//! 会社名・人名はモックと同じ架空のもの（合成データ）。

/// アバターの種類。人は円形、AI は四角形に「AI」の印を付ける。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AvatarKind {
    /// 人。引数はアバターの頭文字。
    Human(String),
    /// AI 社員。引数はアバターの頭文字。
    Ai(String),
}

impl AvatarKind {
    /// アバターの頭文字。
    #[must_use]
    pub fn initial(&self) -> &str {
        match self {
            Self::Human(s) | Self::Ai(s) => s,
        }
    }

    /// AI かどうか（四角いアバター＋「AI」の印）。
    #[must_use]
    pub fn is_ai(&self) -> bool {
        matches!(self, Self::Ai(_))
    }
}

/// AI 社員の在席状態（左の列の表示）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiPresence {
    /// 待機中。
    Standby,
    /// 作業中。
    Working,
    /// 判断待ち。
    WaitingDecision,
}

/// AI 社員の在席状態に対応する文言キー。
#[must_use]
pub fn ai_presence_key(presence: AiPresence) -> &'static str {
    match presence {
        AiPresence::Standby => "ai-presence-standby",
        AiPresence::Working => "ai-presence-working",
        AiPresence::WaitingDecision => "ai-presence-waiting",
    }
}

/// DM・人 の1項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonDm {
    /// 表示名（架空の人物）。
    pub name: String,
    /// 補足表示（例: 「16時まで集中」）。なければ `None`。
    pub note: Option<String>,
}

/// DM・AI社員 の1項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiDm {
    /// 表示名（架空の AI 社員）。
    pub name: String,
    /// 在席状態。
    pub presence: AiPresence,
}

/// 左の列（rail）の表示データ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailData {
    /// 組織名（架空）。
    pub org_name: String,
    /// 表示中の人の名前（「濱田として表示」の濱田）。
    pub viewing_as: String,
    /// チャンネルの一覧。
    pub channels: Vec<String>,
    /// 選択中のチャンネル（あればその名前）。
    pub selected_channel: Option<String>,
    /// DM・人 の一覧。
    pub people: Vec<PersonDm>,
    /// DM・AI社員 の一覧。
    pub ai_staff: Vec<AiDm>,
}

/// 「今日」の画面の3つの区切り。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodaySectionKind {
    /// あなたへの相談。
    Consultations,
    /// 判断待ち。
    Pending,
    /// 離れている間に決まったこと。
    DecidedWhileAway,
}

impl TodaySectionKind {
    /// 区切りの見出しの文言キー。
    #[must_use]
    pub fn heading_key(self) -> &'static str {
        match self {
            Self::Consultations => "today-section-consultations",
            Self::Pending => "today-section-pending",
            Self::DecidedWhileAway => "today-section-decided",
        }
    }
}

/// 「今日」の区切りの1項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodayItem {
    /// 相談・決定の相手（架空）。
    pub who: String,
    /// アバター。
    pub avatar: AvatarKind,
    /// 補足表示（例: 「# 案件B · 2:10」）。
    pub meta: String,
    /// 本文（合成データ）。
    pub text: String,
    /// 「離れている間に決まったこと」の ◆ の有無。
    pub decided: bool,
}

/// 「今日」の区切り1つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodaySection {
    /// 区切りの種類。
    pub kind: TodaySectionKind,
    /// 項目。空なら「なし」の表示。
    pub items: Vec<TodayItem>,
}

/// 「今日」の画面。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TodayView {
    /// 3つの区切り（あなたへの相談・判断待ち・離れている間に決まったこと）。
    pub sections: [TodaySection; 3],
}

/// チャンネル画面のスレッドの要約（メッセージ行の下の「返信 N 件」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadSummary {
    /// 返信の件数。
    pub replies: u32,
    /// 作業中なら true（「{name} 作業中」の表示）。
    pub working: bool,
    /// 完了時刻（「完了 10:58」の表示）。作業中は `None`。
    pub done_at: Option<String>,
}

/// チャンネル画面のメッセージ1件。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageEntry {
    /// 送信者（架空）。
    pub sender: String,
    /// アバター。
    pub avatar: AvatarKind,
    /// 本文（メンションは `@名前` を含む文字列。合成データ）。
    pub body: String,
    /// 添付されるスレッドの要約（なければ `None`）。
    pub thread: Option<ThreadSummary>,
}

/// 受領済みの引き継ぎ（「受領 · 濱田 15:03」）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckState {
    /// 受領した人（架空）。
    pub who: String,
    /// 受領時刻。
    pub time: String,
}

/// 引き継ぎカードの選択肢（A/B 案）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffOption {
    /// 選択肢のラベル（A・B）。
    pub label: String,
    /// 案の本文（合成データ）。
    pub text: String,
}

/// 引き継ぎカード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffData {
    /// 渡す人。
    pub from: String,
    /// 受け取る人。
    pub to: String,
    /// 件名（例: 「仕様変更の件」）。
    pub title: String,
    /// 受領状態（未受領なら `None`）。
    pub acknowledged: Option<AckState>,
    /// 納期などを決める人（「納期の決定：濱田（代表）」）。
    pub decision_owner: String,
    /// 選択肢の一覧。
    pub options: Vec<HandoffOption>,
}

/// 社外への送信の確認カードの状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExternalSendState {
    /// 「送る」「直す」と鍵の注記を表示。
    AwaitingApproval,
    /// 送信後の表示（「濱田 15:20 送信 · 顧客回答待ち」）。
    Sent {
        /// 承認した人（架空）。
        who: String,
        /// 送信時刻。
        time: String,
    },
}

/// 社外への送信の確認カード。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSendCard {
    /// 宛先（架空のアドレス）。
    pub address: String,
    /// 件名。
    pub subject: String,
    /// 根拠（決定の表示など）。
    pub basis: String,
    /// 本文（合成データ）。
    pub body: String,
    /// 待ちか送信後か。
    pub state: ExternalSendState,
}

/// チャンネル画面の時系列の1項目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimelineEntry {
    /// 日付の区切り（例: 「11月9日（月）」）。
    DayDivider { label: String },
    /// 「ここから未読」の区切り。
    UnreadMarker,
    /// メッセージ。
    Message(MessageEntry),
    /// 決定の印（◆ 決定 · 濱田 15:12）。
    Decision { who: String, time: String },
    /// 引き継ぎカード。
    Handoff(HandoffData),
    /// 「あなただけに表示」の行。
    PrivateNote {
        sender: String,
        avatar: AvatarKind,
        body: String,
    },
    /// 社外への送信の確認カード（または送信後の表示）。
    ExternalSend(ExternalSendCard),
}

/// チャンネル画面。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelView {
    /// 画面の題名（例: 「# 案件B」）。
    pub title: String,
    /// 参加者の表示（例: 「田中・佐藤・濱田・リク ほか」）。
    pub members: String,
    /// 時系列。
    pub timeline: Vec<TimelineEntry>,
}

/// 右の枠（スレッドまたは AI の作業）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RightPane {
    /// スレッド。
    Thread(ThreadPane),
    /// AI の作業。
    AiWork(AiWorkPane),
}

/// スレッドの枠。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPane {
    /// 元のチャンネル（例: 「# 案件B」）。
    pub channel: String,
    /// スレッド内のメッセージ。
    pub messages: Vec<MessageEntry>,
    /// 作業中の表示（AI の名前）。完了していれば `None`。
    pub working: Option<String>,
}

/// AI の作業の枠。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiWorkPane {
    /// 作業している AI の名前。
    pub name: String,
    /// 済ませた手順の数。
    pub steps_done: u32,
    /// 手順の総数。
    pub steps_total: u32,
    /// 依頼元（例: 「# 案件B · 田中 9:30」）。
    pub source: String,
    /// 読んだ範囲の件数。
    pub read_range: u32,
    /// 手順のログ（時刻と文言の組）。
    pub log: Vec<(String, String)>,
}

/// 画面全体の表示データ（合成データ）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppData {
    /// 左の列。
    pub rail: RailData,
    /// 「今日」の画面。
    pub today: TodayView,
    /// チャンネルの画面。
    pub channel: ChannelView,
    /// 右の枠。
    pub right_pane: RightPane,
}

/// 合成データを1式返す（モックの架空の会社名・人名を使う。実在の情報は入れない）。
///
/// モックの場面（15:15「顧客宛てメールを確認して送る」）の並びを代表に据える。
#[must_use]
pub fn sample() -> AppData {
    AppData {
        rail: RailData {
            org_name: "株式会社サンプル開発".to_owned(),
            viewing_as: "濱田".to_owned(),
            channels: ["# 案件A", "# 案件B", "# 全体", "# 雑談"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect(),
            selected_channel: Some("# 案件B".to_owned()),
            people: vec![
                PersonDm {
                    name: "田中".to_owned(),
                    note: None,
                },
                PersonDm {
                    name: "佐藤".to_owned(),
                    note: Some("16時まで集中".to_owned()),
                },
            ],
            ai_staff: vec![
                AiDm {
                    name: "ユイ".to_owned(),
                    presence: AiPresence::Standby,
                },
                AiDm {
                    name: "リク".to_owned(),
                    presence: AiPresence::Standby,
                },
                AiDm {
                    name: "ハル".to_owned(),
                    presence: AiPresence::WaitingDecision,
                },
            ],
        },
        today: TodayView {
            sections: [
                TodaySection {
                    kind: TodaySectionKind::Consultations,
                    items: vec![
                        TodayItem {
                            who: "田中".to_owned(),
                            avatar: AvatarKind::Human("田".to_owned()),
                            meta: "# 案件B · 18:42".to_owned(),
                            text: "B社の仕様変更（CSV出力）、夕方に方針を相談させてください".to_owned(),
                            decided: false,
                        },
                        TodayItem {
                            who: "佐藤".to_owned(),
                            avatar: AvatarKind::Human("佐".to_owned()),
                            meta: "DM · 8:05".to_owned(),
                            text: "検証環境の権限を1つ追加してほしい".to_owned(),
                            decided: false,
                        },
                    ],
                },
                TodaySection {
                    kind: TodaySectionKind::Pending,
                    items: vec![TodayItem {
                        who: "ユイ".to_owned(),
                        avatar: AvatarKind::Ai("ユ".to_owned()),
                        meta: "# 案件A · 7:50".to_owned(),
                        text: "A社向けの週報を共有してよいか".to_owned(),
                        decided: false,
                    }],
                },
                TodaySection {
                    kind: TodaySectionKind::DecidedWhileAway,
                    items: vec![TodayItem {
                        who: "田中".to_owned(),
                        avatar: AvatarKind::Human("田".to_owned()),
                        meta: "◆ # 案件A · 昨日 19:30".to_owned(),
                        text: "リリースを11/12に1日延期".to_owned(),
                        decided: true,
                    }],
                },
            ],
        },
        channel: ChannelView {
            title: "# 案件B".to_owned(),
            members: "田中・佐藤・濱田・リク ほか".to_owned(),
            timeline: vec![
                TimelineEntry::DayDivider {
                    label: "11月10日（火）".to_owned(),
                },
                TimelineEntry::Message(MessageEntry {
                    sender: "田中".to_owned(),
                    avatar: AvatarKind::Human("田".to_owned()),
                    body: "B社からの要望の要点です。注文一覧にCSV出力を足したい。納期の12/1は変えたくない、とのこと。".to_owned(),
                    thread: None,
                }),
                TimelineEntry::Message(MessageEntry {
                    sender: "田中".to_owned(),
                    avatar: AvatarKind::Human("田".to_owned()),
                    body: "@濱田 さん、方針は明日の夕方に相談させてください。".to_owned(),
                    thread: None,
                }),
                TimelineEntry::Message(MessageEntry {
                    sender: "リク".to_owned(),
                    avatar: AvatarKind::Ai("リ".to_owned()),
                    body: "昨日18:40〜今日9:30の5件を読みました。調べます。続きはスレッドで進めます。".to_owned(),
                    thread: Some(ThreadSummary {
                        replies: 5,
                        working: false,
                        done_at: Some("10:58".to_owned()),
                    }),
                }),
                TimelineEntry::UnreadMarker,
                TimelineEntry::Handoff(HandoffData {
                    from: "田中".to_owned(),
                    to: "濱田".to_owned(),
                    title: "仕様変更の件".to_owned(),
                    acknowledged: Some(AckState {
                        who: "濱田".to_owned(),
                        time: "15:03".to_owned(),
                    }),
                    decision_owner: "濱田（代表）".to_owned(),
                    options: vec![
                        HandoffOption {
                            label: "A".to_owned(),
                            text: "範囲を絞って12/1（CSVだけ。約1日）".to_owned(),
                        },
                        HandoffOption {
                            label: "B".to_owned(),
                            text: "絞り込みも含めて12/3に延ばす（+2日）".to_owned(),
                        },
                    ],
                }),
                TimelineEntry::Message(MessageEntry {
                    sender: "濱田".to_owned(),
                    avatar: AvatarKind::Human("濱".to_owned()),
                    body: "Aで進めます。範囲を絞って12/1、絞り込みは次回にまわします。".to_owned(),
                    thread: None,
                }),
                TimelineEntry::Decision {
                    who: "濱田".to_owned(),
                    time: "15:12".to_owned(),
                },
                TimelineEntry::PrivateNote {
                    sender: "リク".to_owned(),
                    avatar: AvatarKind::Ai("リ".to_owned()),
                    body: "①B社がCSV出力を要望、納期12/1は固定希望 ②文字コードはBOM付きUTF-8で確定 ③田中さんの案はA（範囲を絞って12/1）とB（12/3に延ばす）の2つです".to_owned(),
                },
                TimelineEntry::ExternalSend(ExternalSendCard {
                    address: "csv-support@b-sha.example".to_owned(),
                    subject: "CSV出力の対応方針について".to_owned(),
                    basis: "◆ 決定 · 濱田 15:12".to_owned(),
                    body: "いつもお世話になっております。ご依頼のCSV出力は、納期12/1のまま対応いたします。".to_owned(),
                    state: ExternalSendState::AwaitingApproval,
                }),
            ],
        },
        right_pane: RightPane::Thread(ThreadPane {
            channel: "# 案件B".to_owned(),
            messages: vec![
                MessageEntry {
                    sender: "リク".to_owned(),
                    avatar: AvatarKind::Ai("リ".to_owned()),
                    body: "調べます。続きはスレッドで進めます。".to_owned(),
                    thread: None,
                },
                MessageEntry {
                    sender: "リク".to_owned(),
                    avatar: AvatarKind::Ai("リ".to_owned()),
                    body: "文字コードの指定が見つかりません。10月の打ち合わせメモに「Excelで開きたい」とあります。B社に確認しますか、社内で決めますか。".to_owned(),
                    thread: None,
                },
                MessageEntry {
                    sender: "田中".to_owned(),
                    avatar: AvatarKind::Human("田".to_owned()),
                    body: "Excelで開けるように、UTF-8のBOM付きで進めて。B社には私から念のため確認します。".to_owned(),
                    thread: None,
                },
            ],
            working: None,
        }),
    }
}
