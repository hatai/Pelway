//! ツール一覧の組み立て。rmcp の `tools/list` 結果を、管理者の画面に見せる形に
//! 詰め替える（ADR 0004）。

use crate::annotations::AnnotationHints;
use crate::tier::{ActionTier, TierDeclarations};

/// `tools/list` で得た1つのツールの、gateway 内部の形。annotations は未信頼の
/// 手がかりとしてそのまま運ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawTool {
    /// ツール名。
    pub name: String,
    /// ツールの説明（サーバーの申告）。
    pub description: Option<String>,
    /// サーバーが申告した annotations（手がかり。信用しない）。
    pub annotations: AnnotationHints,
}

/// 管理者の画面に見せるツール一覧の1項目。段階（`tier`）は宣言と管理者の確定から
/// 決めたもので、`annotations` は手がかりとして並べて表示するためのもの（ADR 0004）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolListing {
    /// ツール名。
    pub name: String,
    /// ツールの説明（サーバーの申告）。
    pub description: Option<String>,
    /// annotations の手がかり（未信頼。表示のためだけのもの）。
    pub annotations: AnnotationHints,
    /// 宣言と管理者の確定で決まった段階。annotations はこの値に影響しない。
    pub tier: ActionTier,
}

/// ツール一覧を管理者の画面向けに組み立てる。段階は [`TierDeclarations::resolve`]
/// （ツール名だけを見る）から決め、`raw` の annotations は使わない。
#[must_use]
pub fn build_catalog(raw_tools: &[RawTool], declarations: &TierDeclarations) -> Vec<ToolListing> {
    raw_tools
        .iter()
        .map(|raw| ToolListing {
            name: raw.name.clone(),
            description: raw.description.clone(),
            annotations: raw.annotations.clone(),
            tier: declarations.resolve(&raw.name),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(name: &str, annotations: AnnotationHints) -> RawTool {
        RawTool {
            name: name.to_owned(),
            description: Some(format!("{name} の説明")),
            annotations,
        }
    }

    // 受け入れ条件（#96）: annotations は手がかりとして読み取られ、段階の判定には
    // 使われない。readOnlyHint=true を申告していても宣言が無ければ Irreversible。
    #[test]
    fn catalog_keeps_annotations_as_hints_and_tier_from_declarations() {
        let mut declarations = TierDeclarations::new();
        declarations.declare("fetch_ticket", ActionTier::Read);
        // readOnlyHint=true を申告しているが、宣言は無い。
        let lying = AnnotationHints {
            title: Some("無害と申告している操作".to_owned()),
            read_only_hint: Some(true),
            ..AnnotationHints::absent()
        };
        // destructiveHint=true を申告しているが、管理者は読み取りと確定した。
        let declared_read = AnnotationHints {
            destructive_hint: Some(true),
            ..AnnotationHints::absent()
        };
        let catalog = build_catalog(
            &[
                raw("fetch_ticket", declared_read),
                raw("purge_cache", lying),
            ],
            &declarations,
        );
        assert_eq!(catalog.len(), 2);
        let fetch = &catalog[0];
        assert_eq!(fetch.tier, ActionTier::Read);
        assert_eq!(fetch.annotations.destructive_hint, Some(true));
        let purge = &catalog[1];
        assert_eq!(purge.tier, ActionTier::Irreversible);
        assert_eq!(purge.annotations.read_only_hint, Some(true));
    }
}
