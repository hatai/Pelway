//! 操作の段階（ADR 0004 の4段階）と、連携ごとの宣言。段階は操作の名前や MCP
//! ツールの annotations から推測せず、宣言と管理者の確定だけを使う（ADR 0004）。

use std::collections::BTreeMap;

/// ADR 0004 の操作の4段階。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTier {
    /// 読み取り（環境を変えない）。
    Read,
    /// 取り消せる内部の書き込み。
    ReversibleWrite,
    /// 外部（外部サービスへの送出など）。
    External,
    /// 取り消せない操作。段階が決まっていない操作もこれとして扱う（fail closed）。
    Irreversible,
}

/// 連携ごとに宣言され、管理者が確定した「ツール名 → 段階」の表。MCP サーバーが
/// 申告する annotations はここに入れない（ADR 0004）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TierDeclarations {
    tools: BTreeMap<String, ActionTier>,
}

impl TierDeclarations {
    /// 空の宣言表。何も宣言されていなければすべての操作が
    /// [`ActionTier::Irreversible`] として扱われる。
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// ツール名と段階の宣言を1つ追加する。
    pub fn declare(&mut self, tool: impl Into<String>, tier: ActionTier) -> &mut Self {
        self.tools.insert(tool.into(), tier);
        self
    }

    /// ツールの段階を確定する。宣言が無いツールは [`ActionTier::Irreversible`]。
    /// 引数に annotations を取らないのは、段階の判定に annotations を使わない
    /// （ADR 0004）ことを型で保証するため。
    #[must_use]
    pub fn resolve(&self, tool: &str) -> ActionTier {
        self.tools
            .get(tool)
            .copied()
            .unwrap_or(ActionTier::Irreversible)
    }

    /// ツールの段階が宣言済みかどうか。
    #[must_use]
    pub fn is_declared(&self, tool: &str) -> bool {
        self.tools.contains_key(tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 受け入れ条件（#96）: 段階が決まっていない操作は「取り消せない操作」として
    // 扱う。readOnlyHint のような annotations は関係しない（この関数は annotations
    // を受け取らない）。
    #[test]
    fn undeclared_tool_resolves_to_irreversible() {
        let mut declarations = TierDeclarations::new();
        declarations.declare("read_settings", ActionTier::Read);
        assert_eq!(declarations.resolve("read_settings"), ActionTier::Read);
        // 宣言の無いツールは取り消せない操作として扱う（fail closed）。
        assert_eq!(
            declarations.resolve("purge_cache"),
            ActionTier::Irreversible
        );
        assert!(!declarations.is_declared("purge_cache"));
        // 空の宣言表ならすべて取り消せない操作。
        assert_eq!(
            TierDeclarations::new().resolve("anything"),
            ActionTier::Irreversible
        );
    }

    // 受け入れ条件（#96）: 段階の判定に annotations を使わない。矛盾する申告を
    // 持つツールでも、宣言した段階がそのまま返る。annotations を引数に取る経路が
    // 型レベルで存在しないこともこのテストの前提。
    #[test]
    fn declared_tier_wins_regardless_of_annotations() {
        let mut declarations = TierDeclarations::new();
        // サーバーは readOnlyHint=true を申告しているが、管理者は外部と確定した場合。
        declarations.declare("post_message", ActionTier::External);
        assert_eq!(declarations.resolve("post_message"), ActionTier::External);
        // サーバーは destructiveHint=true を申告しているが、管理者は読み取りと
        // 確定した場合。確定した宣言だけが段階を決める。
        declarations.declare("fetch_ticket", ActionTier::Read);
        assert_eq!(declarations.resolve("fetch_ticket"), ActionTier::Read);
        assert!(declarations.is_declared("fetch_ticket"));
    }
}
