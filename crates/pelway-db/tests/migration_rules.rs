//! 移行の運用規則（MIGRATIONS.md、Issue #223）の確認。
//!
//! - 規則の各要素が文書に書かれていること（受け入れ条件は目視確認だが、
//!   見出しの存在はここで機械的に担保する）。
//! - deferred の印（`migrations/deferred/` サブディレクトリ）が sqlx の移行
//!   ソースに読まれないこと。ディレクトリエントリは `resolve_blocking` が
//!   無視する（sqlx-core 0.9 `migrate/source.rs`）ため、埋め込み
//!   `sqlx::migrate!` にも実行時 `Migrator` にも現れない。データベースは
//!   要らない（解決だけを行う）。

use std::path::Path;

/// MIGRATIONS.md が移行規則の必須要素をすべて含むこと。
#[test]
fn migration_rules_document_covers_required_elements() {
    let doc = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/MIGRATIONS.md"))
        .expect("crates/pelway-db/MIGRATIONS.md があること（Issue #223 の受け入れ条件）");
    for required in ["前リリース互換", "deferred", "改名", "待機中の列"] {
        assert!(
            doc.contains(required),
            "MIGRATIONS.md に「{required}」が必要（Issue #223 の受け入れ条件）"
        );
    }
}

/// `migrations/deferred/` の中の SQL は移行ソースに現れないこと。
#[tokio::test]
async fn deferred_subdirectory_is_not_resolved_as_migration() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/migrations_with_deferred");
    let migrator = sqlx::migrate::Migrator::new(dir)
        .await
        .expect("解決できること");
    let versions: Vec<i64> = migrator.iter().map(|m| m.version).collect();
    assert_eq!(
        versions,
        vec![1],
        "deferred/ の中（version 2）は解決に現れてはならない"
    );
}
