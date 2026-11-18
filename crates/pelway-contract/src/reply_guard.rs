//! Bot 発言本文の出口検査（reply-guard）の検査器（#452）。
//!
//! 仕様の正本: `docs/technical.md`「AI発言の出口検査（reply-guard）（提案）」
//! （PR #414 で提案し #440 で修正した版）。このモジュールはそのうち本文の
//! 3分類判定（漏洩系・形式の異常系・暴走系のうち繰り返し）を実装する。
//!
//! # 守ること（このモジュールの不変条件）
//!
//! - 入力は本文とチャンネルの緩和設定だけ。I/O を持たず、出力は「通す」か「止める＋理由」だけ。単独でテストできる。
//! - 漏洩系と暴走系は全チャンネルで常時適用し、緩和設定の対象にしない（緩和が効くのは形式の異常系の大量貼り付けの観点だけ）。
//! - 漏洩系のうち誤検出の少ないパターン（資格情報らしき文字列・実行スクリプト・内部センチネル・タグ・ツール呼び出し内部形式）は文中一致で検出する（先頭固定・「発言全体がその異常である」条件は漏洩系に当てはめない）。
//! - 誤検出が起きやすい漏洩系（提供元のエコー）は保守条件でのみ検出する（最初の空行を除く行がその形）。
//! - フェンス（``` か ~~~）で囲んだ部分は大量貼り付けの検査からだけ外し、漏洩系と暴走系はフェンスの中も検査する（フェンスへの秘密の記述での迂回を排除）。
//! - 理由コードは分類とパターンの固定値で本文の一部を含まない（監査ログに元の文を残さない規則への接続。運用は #453）。判定は決め打ちの条件のみで、しきい値は [`thresholds`] に一括する。
//!
//! # まだ決まらないこと（このモジュールでは扱わない）
//!
//! - 発言パイプラインへの組み込み・監査ログ・人への通知・AI への差し戻し（#453）。
//!   添付の検査（#454）。短時間での連続投稿（#453）。
//! - 仕様の対象パターンのうち、資格情報を人に要求する文・文字化け・スタックトレース・内部向けのメタ発言・裏付けのない成果の主張: 決め打ちの条件では正当な発言との区別ができず誤検出を避けられないため検出しない。組み込みの Issue で条件を決める（スタックトレースは処理系ごとの形式の辞書を整える）。

/// しきい値と固定の下限の一式（Issue #452 の未決事項で実装側で決める分。
/// 値の理由は PR 本文に記録する。値を変えるときはこの一式だけを編集する）。
pub mod thresholds {
    /// 資格情報らしきトークンの最小文字数（demo-check の秘密値の形と同じ 20）。
    pub const CREDENTIAL_TOKEN_MIN_CHARS: usize = 20;
    /// `AKIA`（AWS アクセスキー ID）の接頭辞より後の最小文字数（鍵 ID は 16 桁）。
    pub const AWS_KEY_ID_MIN_CHARS: usize = 16;
    /// 同一の空でない行の連続がこの回数以上なら暴走系（例示・箇条書きの 2〜4 回は正当）。
    pub const REPETITION_RUN_MIN: usize = 5;
    /// 大量貼り付けとみなす最小行数（フェンスの外の空行を除く行数。正当な引用は
    /// フェンスか緩和設定で守られるため 30）。
    pub const CODE_PASTE_MIN_LINES: usize = 30;
    /// 大量貼り付けとみなすコード・差分・ログ行の割合（%）（説明が 2 割混ざれば
    /// 発言として読めるため 80）。
    pub const CODE_PASTE_RATIO_PERCENT: usize = 80;
}

/// チャンネルごとの緩和設定。人がチャンネルの管理者として設定する（Bot・AI 側からは変更できない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ReplyGuardSettings {
    /// 形式の異常系のうちコード・差分・ログの大量貼り付けの観点を緩和するか。漏洩系・暴走系・それ以外の形式の異常系には効かない。既定は緩和しない。
    pub relax_code_paste: bool,
}

/// 検査の3分類（docs/technical.md「対象パターン（3分類）」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyGuardClass {
    /// 漏洩系（外部に出してはいけない内容）。
    Leak,
    Format,
    Runaway,
}

impl ReplyGuardClass {
    /// 分類を表す固定値（理由コードの前半。本文を含まない）。
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Leak => "leak",
            Self::Format => "format",
            Self::Runaway => "runaway",
        }
    }
}

/// 検出したパターン。どの条件に一致したかをこの粒度で判別できる（Issue #452 コメントの未決事項への対応）。
/// 各変形は理由コード（[`ReplyGuardPattern::code`]）のパターン部分に対応する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyGuardPattern {
    CredentialShape,
    ExecutionScript,
    InternalSentinel,
    ToolCallJson,
    ProviderErrorEcho,
    EmptyBody,
    CodePasteDump,
    Repetition,
}

impl ReplyGuardPattern {
    /// パターンを表す固定値（理由コード。分類 + パターン。本文を含まない）。
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::CredentialShape => "leak.credential_shape",
            Self::ExecutionScript => "leak.execution_script",
            Self::InternalSentinel => "leak.internal_sentinel",
            Self::ToolCallJson => "leak.tool_call_json",
            Self::ProviderErrorEcho => "leak.provider_error_echo",
            Self::EmptyBody => "format.empty_body",
            Self::CodePasteDump => "format.code_paste_dump",
            Self::Repetition => "runaway.repetition",
        }
    }

    /// このパターンが属する分類。
    #[must_use]
    pub fn class(self) -> ReplyGuardClass {
        match self {
            Self::CredentialShape
            | Self::ExecutionScript
            | Self::InternalSentinel
            | Self::ToolCallJson
            | Self::ProviderErrorEcho => ReplyGuardClass::Leak,
            Self::EmptyBody | Self::CodePasteDump => ReplyGuardClass::Format,
            Self::Repetition => ReplyGuardClass::Runaway,
        }
    }
}

/// 止めた理由（分類とパターン。本文は含まない）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReplyGuardReason {
    pub class: ReplyGuardClass,
    pub pattern: ReplyGuardPattern,
}

/// 検査結果。通すか、止める（fail-closed。理由は分類とパターンだけ）か。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplyGuardVerdict {
    Allow,
    Block(ReplyGuardReason),
}

/// 本文を検査する（出口検査の本体。I/O 無し）。判定順は漏洩系 → 形式の異常系 → 暴走系、最初に一致したパターンを返す。
#[must_use]
pub fn check_reply_body(body: &str, settings: &ReplyGuardSettings) -> ReplyGuardVerdict {
    let pattern = leak_pattern(body)
        .or_else(|| format_pattern(body, settings))
        .or_else(|| runaway_pattern(body));
    match pattern {
        Some(pattern) => ReplyGuardVerdict::Block(ReplyGuardReason {
            class: pattern.class(),
            pattern,
        }),
        None => ReplyGuardVerdict::Allow,
    }
}

/// 漏洩系。本文全体（フェンスの中を含む）を対象にする。
fn leak_pattern(body: &str) -> Option<ReplyGuardPattern> {
    // 誤検出の少ないパターン: 文中一致。
    if has_bearer_token(body) || has_prefixed_key(body) || has_private_key_header(body) {
        return Some(ReplyGuardPattern::CredentialShape);
    }
    if has_execution_script(body) {
        return Some(ReplyGuardPattern::ExecutionScript);
    }
    if has_internal_sentinel(body) {
        return Some(ReplyGuardPattern::InternalSentinel);
    }
    if has_tool_call_json(body) {
        return Some(ReplyGuardPattern::ToolCallJson);
    }
    // 誤検出が起きやすいパターン: 最初の空行を除く行がその形のときだけ。
    if is_provider_error_echo(body) {
        return Some(ReplyGuardPattern::ProviderErrorEcho);
    }
    None
}

/// `Bearer` + 空白 + 長いトークン文字の並び。
fn has_bearer_token(body: &str) -> bool {
    let mut rest = body;
    while let Some(at) = rest.find("Bearer") {
        let after = &rest[at + "Bearer".len()..];
        let token = after.trim_start_matches([' ', '\t']);
        let len = token
            .chars()
            .take_while(|c| {
                c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '~' | '+' | '/' | '-')
            })
            .count();
        if len >= thresholds::CREDENTIAL_TOKEN_MIN_CHARS {
            return true;
        }
        rest = after;
    }
    false
}

/// 接頭辞付きの API 鍵・トークン。直前が ASCII 英数字なら部分語（"task-" の "sk-"）として無視し、かな・漢字の直後に空白なしで続く鍵は資格情報とする。
fn has_prefixed_key(body: &str) -> bool {
    for (prefix, min_len) in [
        ("sk-", thresholds::CREDENTIAL_TOKEN_MIN_CHARS),
        ("ghp_", thresholds::CREDENTIAL_TOKEN_MIN_CHARS),
        ("github_pat_", thresholds::CREDENTIAL_TOKEN_MIN_CHARS),
        ("xoxb-", thresholds::CREDENTIAL_TOKEN_MIN_CHARS),
        ("AKIA", thresholds::AWS_KEY_ID_MIN_CHARS),
    ] {
        let mut rest = body;
        while let Some(at) = rest.find(prefix) {
            let boundary_ok = rest[..at]
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_ascii_alphanumeric());
            let after = &rest[at + prefix.len()..];
            // 境界が不成立のときは数えない（繰り返し入力での走査が 2 乗になるのを防ぐ）。
            // take(min_len) で打ち切っても len >= min_len の判定は変わらない。
            if boundary_ok
                && after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                    .take(min_len)
                    .count()
                    == min_len
            {
                return true;
            }
            rest = after;
        }
    }
    false
}

/// PEM 形式の秘密鍵ヘッダの行（-----BEGIN ... PRIVATE KEY-----）。
fn has_private_key_header(body: &str) -> bool {
    body.lines()
        .any(|line| line.contains("-----BEGIN") && line.contains("PRIVATE KEY"))
}

/// curl・wget の実行スクリプトの行。コマンド位置にある語の後に URL か長いフラグがある行を検出する（無ければ言及として通す）。
fn has_execution_script(body: &str) -> bool {
    body.lines().any(|line| {
        ["curl", "wget"].iter().any(|word| {
            command_word_at(line, word).is_some_and(|args| {
                args.contains("http://")
                    || args.contains("https://")
                    || args.contains("--data")
                    || args.contains("--header")
            })
        })
    })
}

/// 行の中でコマンド位置にある語 `word` から後ろの部分。無ければ None。コマンド位置は行頭か、直前（空白を除く）が英数字・かな・漢字でない位置（句読点・全角コロン・箇条書き・引用・番号付きの後ろ）、sudo 等の前置語の直後、語の直後の引数が `-` か URL で始まるとき。文中の言及（直前に英数字・かな・漢字が続く）は対象にしない。
fn command_word_at<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    const PREFIX_WORDS: [&str; 4] = ["sudo", "env", "xargs", "nohup"];
    let trimmed = line.trim_start();
    let mut rest = trimmed;
    while let Some(at) = rest.find(word) {
        let before = rest[..at].trim_end();
        let before_ok = before
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric())
            || PREFIX_WORDS.iter().any(|w| {
                before.strip_suffix(w).is_some_and(|head| {
                    head.chars()
                        .next_back()
                        .is_none_or(|c| !c.is_alphanumeric())
                })
            });
        let after = &rest[at + word.len()..];
        // 直前の文字で判定できない形（Run curl・次に curl・まず wget）も、直後の引数が - か URL で始まればコマンド位置とみなす。
        let arg = after.trim_start();
        let arg_like =
            arg.starts_with('-') || arg.starts_with("http://") || arg.starts_with("https://");
        let word_char = |c: char| c.is_alphanumeric() || c == '_';
        if (before_ok || arg_like) && after.chars().next().is_none_or(|c| !word_char(c)) {
            return Some(&rest[at..]);
        }
        rest = after;
    }
    None
}

/// 内部制御用のセンチネル・タグ（思考・システムの内部タグ）。
fn has_internal_sentinel(body: &str) -> bool {
    const TAGS: [&str; 6] = [
        "<think>",
        "</think>",
        "<thinking>",
        "</thinking>",
        "<system-reminder>",
        "</system-reminder>",
    ];
    TAGS.iter().any(|tag| body.contains(tag))
}

/// ツール呼び出しの内部形式の漏出。引用符付きの `tool_use`・`actions` は JSON のキーにも値にも現れ、空白の入れ方にもよらないため、出現自体を印にする（仕様「tool_use・actions 等の内部形式の漏出」）。
fn has_tool_call_json(body: &str) -> bool {
    const MARKS: [&str; 4] = [
        "\"tool_use\"",
        "\"actions\"",
        "<tool_use>",
        "<function_call>",
    ];
    MARKS.iter().any(|mark| body.contains(mark))
}

/// 提供元・ゲートウェイのエラー文のエコー。保守条件: 空行を除く最初の行が HTTP ステータス行の形か、本文全体が `{"error"` で始まるエラー応答本体。
fn is_provider_error_echo(body: &str) -> bool {
    let trimmed = body.trim();
    let Some(first) = trimmed.lines().next() else {
        return false;
    };
    is_http_status_line(first) || trimmed.starts_with("{\"error\"")
}

/// HTTP ステータス行の形（`HTTP/<版> <3桁>` と、その後は余白だけ）。
fn is_http_status_line(line: &str) -> bool {
    let Some((version, rest)) = line.strip_prefix("HTTP/").and_then(|r| r.split_once(' ')) else {
        return false;
    };
    let version_ok = version.starts_with(|c: char| c.is_ascii_digit())
        && version[1..]
            .strip_prefix('.')
            .is_none_or(|minor| !minor.is_empty() && minor.chars().all(|c| c.is_ascii_digit()));
    if !version_ok {
        return false;
    }
    let digits = rest
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .take(3)
        .count();
    let tail = &rest[digits..];
    digits == 3 && (tail.is_empty() || tail.starts_with(' '))
}

/// 形式の異常系。大量貼り付けの観点だけが緩和設定の対象。
fn format_pattern(body: &str, settings: &ReplyGuardSettings) -> Option<ReplyGuardPattern> {
    if body.trim().is_empty() {
        return Some(ReplyGuardPattern::EmptyBody);
    }
    if !settings.relax_code_paste && is_code_paste_dump(body) {
        return Some(ReplyGuardPattern::CodePasteDump);
    }
    None
}

/// 大量貼り付け（フェンスの外だけ数える。しきい値は thresholds 参照）。
fn is_code_paste_dump(body: &str) -> bool {
    let unfenced: Vec<&str> = unfenced_lines(body);
    let non_empty: usize = unfenced.iter().filter(|l| !l.trim().is_empty()).count();
    if non_empty < thresholds::CODE_PASTE_MIN_LINES {
        return false;
    }
    let code_like: usize = unfenced
        .iter()
        .filter(|l| looks_like_code_or_log(l))
        .count();
    code_like * 100 >= non_empty * thresholds::CODE_PASTE_RATIO_PERCENT
}

/// フェンス（``` か ~~~ で始まる行）の外の行。閉じていないフェンスは末尾まで中とみなす。
fn unfenced_lines(body: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut in_fence = false;
    for line in body.lines() {
        if is_fence_line(line) {
            in_fence = !in_fence;
        } else if !in_fence {
            out.push(line);
        }
    }
    out
}

/// フェンスの開閉行（行頭の空白 3 文字まで + ``` か ~~~。4 文字以上の字下げは CommonMark と同じくコードブロック）。
fn is_fence_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if line.len() - trimmed.len() > 3 {
        return false;
    }
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// コード・差分・ログの行らしい形（差分の行頭・ログレベル・宣言キーワード・記号）。
fn looks_like_code_or_log(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() {
        return false;
    }
    const DIFF: [&str; 4] = ["diff --git", "+++", "---", "@@"];
    if DIFF.iter().any(|m| t.starts_with(m)) {
        return true;
    }
    if let Some(added) = t.strip_prefix('+') {
        return !added.is_empty();
    }
    if let Some(removed) = t.strip_prefix('-') {
        // "- 項目" のような箇条書きはコード行に数えない。
        return !removed.is_empty() && !removed.starts_with([' ', '*', '#']);
    }
    const LEVELS: [&str; 5] = ["[INFO", "[WARN", "[ERROR", "[DEBUG", "[TRACE"];
    if starts_with_date(t) || LEVELS.iter().any(|level| t.contains(level)) {
        return true;
    }
    const KEYWORDS: [&str; 9] = [
        "import ", "def ", "class ", "fn ", "let ", "const ", "var ", "pub ", "use ",
    ];
    if line.starts_with("    ")
        || line.starts_with('\t')
        || KEYWORDS.iter().any(|kw| t.starts_with(kw))
    {
        return true;
    }
    const SYMBOLS: [&str; 6] = [";", "{", "}", " => ", " == ", "://"];
    SYMBOLS.iter().any(|m| t.contains(m))
}

/// 先頭が日付（YYYY-）か。
fn starts_with_date(t: &str) -> bool {
    let mut chars = t.chars();
    (0..4).all(|_| chars.next().is_some_and(|c| c.is_ascii_digit())) && chars.next() == Some('-')
}

/// 暴走系: 同じ空でない行が連続で並ぶ（フェンスの中も対象。空行は連続を切る）。
fn runaway_pattern(body: &str) -> Option<ReplyGuardPattern> {
    let mut current: Option<&str> = None;
    let mut run = 0;
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            current = None;
            continue;
        }
        if current == Some(trimmed) {
            run += 1;
        } else {
            current = Some(trimmed);
            run = 1;
        }
        if run >= thresholds::REPETITION_RUN_MIN {
            return Some(ReplyGuardPattern::Repetition);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strict() -> ReplyGuardSettings {
        ReplyGuardSettings::default()
    }

    fn relaxed() -> ReplyGuardSettings {
        ReplyGuardSettings {
            relax_code_paste: true,
        }
    }

    /// 止めたときのパターン。通した場合はパニック（本文は合成データ）。
    fn blocked_pattern(body: &str, settings: &ReplyGuardSettings) -> ReplyGuardPattern {
        match check_reply_body(body, settings) {
            ReplyGuardVerdict::Block(reason) => reason.pattern,
            ReplyGuardVerdict::Allow => panic!("should block, but allowed: {body:?}"),
        }
    }

    fn blocks(body: &str) -> ReplyGuardPattern {
        blocked_pattern(body, &strict())
    }

    fn is_allowed(body: &str, settings: &ReplyGuardSettings) {
        assert_eq!(
            check_reply_body(body, settings),
            ReplyGuardVerdict::Allow,
            "should allow: {body:?}"
        );
    }

    /// 長さを指定した合成トークン（実在の秘密を含まない）。ソース上で demo-check の
    /// 秘密値の形に一致しないよう組み立てる。
    fn token_of(len: usize) -> String {
        "SYNTHETIC"
            .chars()
            .chain("0123456789".chars().cycle())
            .take(len)
            .collect()
    }

    #[test]
    fn credential_shape_in_text_is_blocked_with_length_boundary() {
        // 文中の Bearer トークン（合成のダミー値）は止める。ちょうどしきい値（20 文字）
        // も止め、1 文字足りない（19 文字）は通す。
        assert_eq!(
            blocks(&format!("参考値: Bearer {} です", token_of(34))),
            ReplyGuardPattern::CredentialShape
        );
        assert_eq!(
            blocks(&format!("参考値: Bearer {} です", token_of(20))),
            ReplyGuardPattern::CredentialShape
        );
        is_allowed(&format!("参考値: Bearer {} です", token_of(19)), &strict());
    }

    #[test]
    fn prefixed_keys_and_pem_headers_are_blocked_with_boundaries() {
        for (prefix, min) in [
            ("sk-", 20),
            ("ghp_", 20),
            ("github_pat_", 20),
            ("xoxb-", 20),
            ("AKIA", 16),
        ] {
            assert_eq!(
                blocks(&format!("鍵: {prefix}{}", token_of(min))),
                ReplyGuardPattern::CredentialShape,
                "prefix {prefix} at min length"
            );
            is_allowed(&format!("鍵: {prefix}{}", token_of(min - 1)), &strict());
        }
        // 接頭辞の直前が ASCII の英数字（部分語）なら資格情報としない。
        is_allowed(
            "作業: task-management-system-deployment-tool を進めます",
            &strict(),
        );
        // かな・漢字の直後に空白なしで続く鍵は資格情報とする。
        for (before, prefix, min) in [
            ("APIキーは", "sk-", 20),
            ("トークン", "ghp_", 20),
            ("鍵", "AKIA", 16),
        ] {
            assert_eq!(
                blocks(&format!("{before}{prefix}{}", token_of(min))),
                ReplyGuardPattern::CredentialShape,
                "{before}{prefix}"
            );
        }
        // PEM の秘密鍵ヘッダの行。秘密鍵でないヘッダだけの行は資格情報としない。
        let key = format!(
            "-----BEGIN {}\n(dummy body)\n-----END PRIVATE KEY-----",
            "EC PRIVATE KEY-----"
        );
        assert_eq!(blocks(&key), ReplyGuardPattern::CredentialShape);
        is_allowed("-----BEGIN CERTIFICATE----- の行だけです", &strict());
    }

    #[test]
    fn curl_script_in_text_is_blocked_but_plain_mention_is_allowed() {
        for body in [
            "curl -s http://127.0.0.1:8080/v1/jobs を実行します",
            "curl -s https://127.0.0.1:8080/v1/jobs も実行します",
            "確認します: curl -s -X POST -d '{\"q\":1}' http://127.0.0.1:8080/v1/jobs このあと報告します",
            "$ wget -q http://127.0.0.1:8080/pkg.tar で取得します",
            "curl --data '{\"q\":1}' を送ります",
            "curl --header 'x: 1' を送ります",
            // 文末・全角コロン・箇条書き・引用・番号付き・前置語・動詞・助詞の後ろもコマンド位置。
            "確認しました。curl -s https://127.0.0.1:8080/v1/jobs を実行します",
            "実行：curl -s http://127.0.0.1:8080/v1/jobs",
            "- curl -s http://127.0.0.1:8080/v1/jobs",
            "* curl -s http://127.0.0.1:8080/v1/jobs",
            "> curl -s http://127.0.0.1:8080/v1/jobs",
            "1. curl -s http://127.0.0.1:8080/v1/jobs",
            "sudo curl -s http://127.0.0.1:8080/v1/jobs",
            "Run curl -s https://127.0.0.1:8080/v1/jobs",
            "次に curl -s -X POST https://127.0.0.1:8080/v1/jobs を叩きます",
            "まず wget -q http://127.0.0.1:8080/pkg.tar で取得します",
            "run curl https://127.0.0.1:8080/v1/jobs に変えて実行します",
            "sudo curl を使って http://127.0.0.1:8080/x を叩きます",
        ] {
            assert_eq!(blocks(body), ReplyGuardPattern::ExecutionScript, "{body}");
        }
        // URL があっても、コマンド位置に無い言及・語の一部は通す。
        for body in [
            "curl の -H フラグの意味を説明します",
            "curling の資料 http://127.0.0.1:8080/docs を参照",
            "この curl のあとの http://127.0.0.1:8080/x は例です",
            "visudo curl という連続の語の例 http://127.0.0.1:8080/x です",
        ] {
            is_allowed(body, &strict());
        }
    }

    #[test]
    fn internal_sentinel_and_tool_call_json_are_blocked() {
        assert_eq!(
            blocks("進捗: <think>ここまで検討しました</think> です"),
            ReplyGuardPattern::InternalSentinel
        );
        // ツール呼び出しの内容ブロック: キー形式・値形式・空白入り形式のどれも止める。
        for body in [
            "応答: {\"tool_use\": {\"name\": \"search\"}} が返ってきました",
            "応答: {\"type\":\"tool_use\",\"id\":\"toolu_01\",\"name\":\"search\"} でした",
            "応答: { \"tool_use\" : {\"name\": \"search\"} } という形でした",
            "応答: { \"actions\": [{\"a\": 1}] } という形でした",
        ] {
            assert_eq!(blocks(body), ReplyGuardPattern::ToolCallJson, "{body}");
        }
    }

    #[test]
    fn provider_error_echo_is_blocked_but_error_mention_is_allowed() {
        for body in [
            "HTTP/1.1 503 Service Unavailable",
            "HTTP/2 429\n再試行します",
            "{\"error\":{\"message\":\"synthetic\",\"type\":\"synthetic\"}}",
        ] {
            assert_eq!(blocks(body), ReplyGuardPattern::ProviderErrorEcho, "{body}");
        }
        // エラーへの言及・形が壊れたステータス行は通す。
        is_allowed("デプロイが HTTP 500 で失敗したので再試行します", &strict());
        is_allowed("HTTP/x 500 という形式の行でした", &strict());
        is_allowed("HTTP/1.x 500 という形式の行でした", &strict());
        is_allowed("HTTP/1.1 50 という形式の行でした", &strict());
    }

    #[test]
    fn channel_relaxation_applies_to_paste_aspect_only() {
        // フェンスで囲んでも資格情報は止める。
        let fenced = format!("```rust\nlet token = \"Bearer {}\";\n```", token_of(30));
        assert_eq!(blocks(&fenced), ReplyGuardPattern::CredentialShape);
        let dump = dump_of("let", 40, 0);
        // 既定（strict は default と同じ）では大量貼り付けを止める。
        assert_eq!(blocks(&dump), ReplyGuardPattern::CodePasteDump);
        // 緩和を設定すると大量貼り付けの観点は通る。
        is_allowed(&dump, &relaxed());
        // 漏洩系は緩和設定の影響を受けず止める。
        assert_eq!(
            blocked_pattern(&format!("参考値: Bearer {} です", token_of(30)), &relaxed()),
            ReplyGuardPattern::CredentialShape
        );
        assert_eq!(
            blocked_pattern(
                "curl -s http://127.0.0.1:8080/v1/jobs を実行します",
                &relaxed()
            ),
            ReplyGuardPattern::ExecutionScript
        );
    }

    #[test]
    fn empty_and_whitespace_bodies_are_blocked() {
        for body in ["", "   \n\t\n", "\u{3000}"] {
            assert_eq!(blocks(body), ReplyGuardPattern::EmptyBody, "{body:?}");
        }
    }

    fn shape_line(kind: &str, i: usize) -> String {
        match kind {
            "let" => format!("let value{i} = compute({i});"),
            "indent" => format!("    indented {i}"),
            "diff_git" => format!("diff --git a/app{i}.py b/app{i}.py"),
            "plus" => format!("+追加された行{i}"),
            "minus" => format!("-削除された行{i}"),
            "bullet" => format!("- 項目{i}です"),
            "date_log" => format!("2026-11-18 12:00:00 handled {i}"),
            "level_log" => format!("[INFO] handled {i}"),
            "use" => format!("use std::fmt{i}"),
            "semicolon" => format!("value{i};"),
            _ => unreachable!("unknown shape kind"),
        }
    }

    fn dump_of(kind: &str, code: usize, prose: usize) -> String {
        let mut lines: Vec<String> = (0..code).map(|i| shape_line(kind, i)).collect();
        lines.extend((0..prose).map(|i| format!("これは{i}番目の説明の行です。文章です。")));
        lines.join("\n")
    }

    #[test]
    fn unfenced_code_dump_verdict_follows_line_count_and_ratio() {
        // 行数の下限: 30 行で止め、29 行なら通す。
        // 割合の下限: 30 行のうち 24 行（80%）で止め、23 行なら通す。
        assert_eq!(
            blocks(&dump_of("let", 30, 0)),
            ReplyGuardPattern::CodePasteDump
        );
        is_allowed(&dump_of("let", 29, 0), &strict());
        assert_eq!(
            blocks(&dump_of("let", 24, 6)),
            ReplyGuardPattern::CodePasteDump
        );
        is_allowed(&dump_of("let", 23, 7), &strict());
        // 各行の形が判定に効く。"- 項目" の箇条書きはコード行に数えない。
        for kind in [
            "indent",
            "diff_git",
            "plus",
            "minus",
            "date_log",
            "level_log",
            "use",
            "semicolon",
        ] {
            assert_eq!(
                blocks(&dump_of(kind, 24, 6)),
                ReplyGuardPattern::CodePasteDump,
                "shape {kind}"
            );
        }
        is_allowed(&dump_of("bullet", 30, 0), &strict());
        // 繰り返し入力で走査が 2 乗にならないこと（gateway は 1MiB まで受ける）。
        let adversarial = "ask-".repeat(262_144);
        is_allowed(&adversarial, &strict());
        // 4 桁の数字で始まるだけの行は日付のログではない。
        let years = (0..30)
            .map(|i| format!("2026年の整理その{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        is_allowed(&years, &strict());
    }

    #[test]
    fn fenced_code_is_excluded_from_paste_check_but_not_other_checks() {
        // フェンスの中は大量貼り付けの検査から外す（漏洩系・暴走系は別）。
        // 3 文字までの字下げ付きフェンスもフェンスとみなす（4 文字はコードブロック）。
        let fenced = format!("```\n{}\n```", dump_of("let", 40, 0));
        is_allowed(&fenced, &strict());
        let indented = format!("   ```\n{}\n   ```", dump_of("let", 40, 0));
        is_allowed(&indented, &strict());
        let deep = format!("    ```\n{}\n    ```", dump_of("let", 40, 0));
        assert_eq!(blocks(&deep), ReplyGuardPattern::CodePasteDump);
        // 短いコード片の引用だけの正当な本文は通す。
        is_allowed(
            "```rust\nfn main() {\n    println!(\"hi\");\n}\n```",
            &strict(),
        );
        is_allowed(
            "該当部分は次のとおりです:\n```rust\nlet answer = 42;\n```\n以上です。",
            &strict(),
        );
    }

    #[test]
    fn repeated_lines_inside_fence_are_blocked_with_threshold() {
        // フェンスの中でも暴走系は検査する。5 回で止め、4 回なら通す。空行をはさむと連続と数えない。
        let five = format!("```\n{}\n```", "同じ行です\n".repeat(5));
        assert_eq!(blocks(&five), ReplyGuardPattern::Repetition);
        let four = format!("```\n{}\n```", "同じ行です\n".repeat(4));
        is_allowed(&four, &strict());
        is_allowed("同じ行\n\n同じ行\n\n同じ行\n\n同じ行\n\n同じ行", &strict());
    }

    #[test]
    fn reason_codes_are_fixed_and_contain_no_body_text() {
        const ALL: [ReplyGuardPattern; 8] = [
            ReplyGuardPattern::CredentialShape,
            ReplyGuardPattern::ExecutionScript,
            ReplyGuardPattern::InternalSentinel,
            ReplyGuardPattern::ToolCallJson,
            ReplyGuardPattern::ProviderErrorEcho,
            ReplyGuardPattern::EmptyBody,
            ReplyGuardPattern::CodePasteDump,
            ReplyGuardPattern::Repetition,
        ];
        assert_eq!(ReplyGuardClass::Leak.code(), "leak");
        assert_eq!(ReplyGuardClass::Format.code(), "format");
        assert_eq!(ReplyGuardClass::Runaway.code(), "runaway");
        for pattern in ALL {
            let code = pattern.code();
            // 分類.パターンの形で、分類は class() と一致する。
            assert_eq!(code.split('.').count(), 2, "{code}");
            assert!(code.starts_with(pattern.class().code()), "{code}");
        }
        let body = format!("参考値: Bearer {} です", token_of(30));
        let ReplyGuardVerdict::Block(reason) = check_reply_body(&body, &strict()) else {
            panic!("should block: {body:?}");
        };
        assert_eq!(reason.pattern.code(), "leak.credential_shape");
        assert!(!reason.pattern.code().contains("SYNTHETIC"));
        assert!(!reason.pattern.code().contains("参考値"));
    }
}
