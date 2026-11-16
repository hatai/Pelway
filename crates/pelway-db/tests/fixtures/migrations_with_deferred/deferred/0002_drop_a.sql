-- deferred/ の中は sqlx の移行ソースが読まない（ディレクトリは無視される）。
-- これが MIGRATIONS.md の「deferred の印」の仕組み。
DROP TABLE a;
