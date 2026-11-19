//! Fluent による文言の取得の検査（窓は開かない）。
//!
//! - 用意している言語（ja・en）の bundle が作れること。他の言語は拒むこと。
//! - 日英それぞれで、`USED_KEYS` のすべてのキーに文言があり、Fluent の誤りが
//!   出ないこと（`message` が `Some` を返すこと）。
//! - 文言の中身（本文）はこのテストでは出力しない（空でないことだけを確かめる）。
//! - OS の言語設定から初期言語を決めること（日本語は ja、それ以外は en）。
//! - 読んだ範囲の終わり（時刻）と社外送信の宛先が別の Fluent 変数であること。

use pelway_client_ui::i18n::{FluentVars, LANGS, Messages, USED_KEYS, initial_lang};

#[test]
fn bundles_build_for_both_languages_and_reject_others() {
    for lang in LANGS {
        assert!(
            Messages::new(lang).is_some(),
            "{lang} の bundle が作れること"
        );
    }
    assert!(
        Messages::new("fr").is_none(),
        "用意していない言語は拒むこと"
    );
    assert!(Messages::new("").is_none(), "空の言語名は拒むこと");
}

#[test]
fn every_used_key_formats_without_errors_in_both_languages() {
    let vars = FluentVars::sample();
    for lang in LANGS {
        let messages =
            Messages::new(lang).unwrap_or_else(|| panic!("{lang} の bundle が作れること"));
        let mut count = 0usize;
        for key in USED_KEYS {
            let text = messages.message(key, &vars).unwrap_or_else(|| {
                panic!("{lang} の {key} が引けない（キー不足か、変数代入の誤り）")
            });
            assert!(!text.is_empty(), "{lang} の {key} が空の文言");
            count += 1;
        }
        assert_eq!(count, USED_KEYS.len(), "{lang} で調べたキーの件数");
    }
}

/// Issue 323: AI の呼称は「AI Bot」（半角スペース、B は大文字）に統一する。
/// ja の文言に旧称「AI社員」（スペースありを含む）と表記ゆれが出ないこと。
#[test]
fn ja_labels_call_the_ai_ai_bot() {
    let vars = FluentVars::sample();
    let ja = Messages::new("ja").expect("ja の bundle が作れること");
    assert_eq!(
        ja.message("rail-dm-ai", &vars).as_deref(),
        Some("DM・AI Bot"),
        "rail の AI グループの見出しが「DM・AI Bot」であること"
    );
    for key in USED_KEYS {
        let text = ja
            .message(key, &vars)
            .unwrap_or_else(|| panic!("ja の {key} が引けない（キー不足か、変数代入の誤り）"));
        assert!(
            !text.contains("AI社員") && !text.contains("AI 社員"),
            "旧称の「AI社員」が残っている: {key} = {text}"
        );
        assert!(
            !text.contains("AIボット")
                && !text.contains("AI ボット")
                && !text.contains("AIbot")
                && !text.contains("AIBot")
                && !text.contains("AI bot"),
            "表記ゆれ（AIボット・AIbot・AI bot 等）がある: {key} = {text}"
        );
    }
}

#[test]
fn unknown_keys_return_none() {
    let vars = FluentVars::sample();
    for lang in LANGS {
        let messages = Messages::new(lang).expect("bundle が作れること");
        assert!(
            messages.message("no-such-key", &vars).is_none(),
            "{lang} に無いキーは None であること"
        );
    }
}

/// Issue 384: 起動時の初期言語。OS の言語が日本語なら ja、それ以外は en。
/// OS の値は引数で与える（この関数は文字列を受け取るだけ。OS からの取得は
/// 呼び出し側の役）。未知の言語・空文字列でも ja/en のどちらかを返すため、
/// 翻訳キーの生文字列を画面へ出す経路が無い。
#[test]
fn initial_lang_picks_ja_for_japanese_os_and_en_otherwise() {
    // 日本語の OS 言語（BCP-47 相当）は ja。
    for os_lang in ["ja", "ja-JP", "ja_jp", "JA"] {
        assert_eq!(
            initial_lang(os_lang),
            "ja",
            "OS の言語 {os_lang} の初期言語は ja であること"
        );
    }
    // それ以外（未知・空を含む）は en。
    for os_lang in ["en", "en-US", "fr-FR", "zh_CN", "klingon", ""] {
        assert_eq!(
            initial_lang(os_lang),
            "en",
            "OS の言語 {os_lang} の初期言語は en であること"
        );
    }
}

/// Issue 384: 初期言語は常に用意している言語（ja・en）のどれかを返すこと。
/// 未知の OS 言語でも `Messages::new` が作れる言語だけを返すため、翻訳の
/// 無いキーを画面へ出さない。
#[test]
fn initial_lang_always_returns_a_lang_with_a_bundle() {
    for os_lang in ["ja-JP", "en-US", "fr-FR", "zh", ""] {
        let lang = initial_lang(os_lang);
        assert!(
            LANGS.contains(&lang),
            "OS の言語 {os_lang} から返る {lang} は LANGS に入っていること"
        );
        assert!(
            Messages::new(lang).is_some(),
            "OS の言語 {os_lang} から返る {lang} の bundle が作れること"
        );
    }
}

/// Issue 384: `.ftl` の `$to`（読んだ範囲の終わり＝時刻）と `$address`
/// （社外送信の宛先）は別の変数であること。1 組の vars（`to` に時刻、
/// `address` に宛先）で両方のキーが同時に正しく整形できること。
#[test]
fn to_and_address_are_separate_variables() {
    // Fluent は変数の値の前後に bidi isolate（U+2068/U+2069）を付ける
    // （fluent-bundle の既定）。期待値はこの挙動に合わせて組み立てる。
    fn iso(value: &str) -> String {
        format!("\u{2068}{value}\u{2069}")
    }
    // to（時刻）と address（宛先）に別々の値を入れた 1 組の vars。
    let from = "昨日18:40";
    let to = "今日9:30";
    let n = "5";
    let address = "csv-support@b-sha.example";
    let vars = FluentVars {
        from: from.to_owned(),
        to: to.to_owned(),
        n: n.to_owned(),
        address: address.to_owned(),
        ..FluentVars::sample()
    };
    let ja = Messages::new("ja").expect("ja の bundle が作れること");
    assert_eq!(
        ja.message("mention-read-range", &vars),
        Some(format!(
            "{}〜{}の{}件を読みました",
            iso(from),
            iso(to),
            iso(n)
        )),
        "ja の mention-read-range に to（時刻）が入ること"
    );
    assert_eq!(
        ja.message("ext-card-question", &vars),
        Some(format!("{}にメールを送りますか", iso(address))),
        "ja の ext-card-question に address（宛先）が入ること（to の時刻は入らない）"
    );
    let en = Messages::new("en").expect("en の bundle が作れること");
    assert_eq!(
        en.message("mention-read-range", &vars),
        Some(format!(
            "Read {} messages from {} to {}",
            iso(n),
            iso(from),
            iso(to)
        )),
        "en の mention-read-range に to（時刻）が入ること"
    );
    assert_eq!(
        en.message("ext-card-question", &vars),
        Some(format!("Send an email to {}?", iso(address))),
        "en の ext-card-question に address（宛先）が入ること（to の時刻は入らない）"
    );
}
