//! Fluent による文言の取得の検査（窓は開かない）。
//!
//! - 用意している言語（ja・en）の bundle が作れること。他の言語は拒むこと。
//! - 日英それぞれで、`USED_KEYS` のすべてのキーに文言があり、Fluent の誤りが
//!   出ないこと（`message` が `Some` を返すこと）。
//! - 文言の中身（本文）はこのテストでは出力しない（空でないことだけを確かめる）。

use pelway_client_ui::i18n::{FluentVars, LANGS, Messages, USED_KEYS};

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
