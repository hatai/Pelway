//! MCP ツールの annotations。未信頼の手がかりとしてそのまま保持する（ADR 0004）。
//! ここにある値は MCP サーバーが自分で申告したもので、検証されていない。管理者の
//! 画面に手がかりとして表示するためだけに使い、段階（[`crate::tier::ActionTier`]）
//! の判定には使わない。この型には段階を計算するメソッドを置かない。

/// MCP の tool annotations を、管理者の画面の手がかりとして保持したもの。
///
/// すべて `Option`。サーバーが申告しなかった項目は `None` のまま返す。`None` も
/// 値も段階の判定には使わない。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnnotationHints {
    /// ツールの人が読める題（申告値。内容の保証はない）。
    pub title: Option<String>,
    /// 環境を変えないという申告（`readOnlyHint`）。
    pub read_only_hint: Option<bool>,
    /// 破壊的な更新をし得るという申告（`destructiveHint`）。
    pub destructive_hint: Option<bool>,
    /// 同じ引数の再呼び出しで追加の効果が無いという申告（`idempotentHint`）。
    pub idempotent_hint: Option<bool>,
    /// 外部の開かれた世界とやり取りし得るという申告（`openWorldHint`）。
    pub open_world_hint: Option<bool>,
}

impl AnnotationHints {
    /// annotations が無いツール用の、すべて `None` の手がかり。
    #[must_use]
    pub fn absent() -> Self {
        Self::default()
    }
}

impl From<&rmcp::model::ToolAnnotations> for AnnotationHints {
    /// rmcp の申告値を手がかりとしてそのまま写す。値の解釈・補完はしない。
    fn from(source: &rmcp::model::ToolAnnotations) -> Self {
        Self {
            title: source.title.clone(),
            read_only_hint: source.read_only_hint,
            destructive_hint: source.destructive_hint,
            idempotent_hint: source.idempotent_hint,
            open_world_hint: source.open_world_hint,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 受け入れ条件（#96）: annotations を手がかりとして読み取る。申告された値は
    // そのまま管理者の画面に見せる形で残る（補完も解釈もしない）。
    #[test]
    fn declared_values_are_carried_verbatim() {
        let declared = rmcp::model::ToolAnnotations::from_raw(
            Some("設定の読み取り".to_owned()),
            Some(true),
            Some(false),
            Some(true),
            Some(false),
        );
        let hints = AnnotationHints::from(&declared);
        assert_eq!(hints.title.as_deref(), Some("設定の読み取り"));
        assert_eq!(hints.read_only_hint, Some(true));
        assert_eq!(hints.destructive_hint, Some(false));
        assert_eq!(hints.idempotent_hint, Some(true));
        assert_eq!(hints.open_world_hint, Some(false));
    }

    // 受け入れ条件（#96）: 申告の一部だけがあるツールでは、無い項目は None のまま。
    // 欠落を既定値で補完しない（補完すると申告していない意味を与えてしまう）。
    // 申告が全く無いときも（absent()）、すべて None になる。
    #[test]
    fn missing_values_stay_none() {
        let partial = rmcp::model::ToolAnnotations::from_raw(None, Some(true), None, None, None);
        let hints = AnnotationHints::from(&partial);
        assert_eq!(hints.title, None);
        assert_eq!(hints.read_only_hint, Some(true));
        assert_eq!(hints.destructive_hint, None);
        assert_eq!(hints.idempotent_hint, None);
        assert_eq!(hints.open_world_hint, None);
        assert_eq!(AnnotationHints::absent(), AnnotationHints::default());
        assert_eq!(AnnotationHints::absent().read_only_hint, None);
    }
}
