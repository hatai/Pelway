# Issue #223 の作業ログ

## 実装者メモ（round 1）

対象 head: 34cdc411e93b0b1c20c5cfa784dae666891ff1ae

1. 詰まった箇所: scripts/agent-gh dev: GitHub App トークン発行に失敗した。原因は環境の DNS 破壊（/etc/resolv.conf が /mnt/wsl/resolv.conf への切れたシンボリックリンク）。sudo も不可（no_new_privileges）。127.0.0.53（systemd-resolved）で名前解決する自作 CONNECT プロキシを立て HTTPS_PROXY で回避した。別件: ~/.cache が読み込み専用で mbx（mise の cargo ラッパー）が失敗するため XDG_CACHE_HOME を worktree 外の許可済み一時領域へ向けた。
2. 曖昧だった文: 「移行規則（前リリース互換の原則、deferred 削除の印と適用の仕組み、改名の手順、待機中の列の制約）が文書化されている → pelway-db の文書または AGENTS.md の該当箇所を目視で確認」（Issue #223 受け入れ条件）
3. 仮定と解釈: 「pelway-db の文書」= crates/pelway-db 配下の文書と解釈し、MIGRATIONS.md を新設して lib.rs のドキュメントから参照した（AGENTS.md はリポジトリー全体の規則を置く場所で、スキーマ移行の技術規則は crate に寄せる方が配置として自然なため。docs/adr/ は意思決定記録で性格が違う）。未決事項「deferred の印の具体的な形式」は、移行ファイルを migrations/deferred/ サブディレクトリに置く方式に決めた: sqlx の resolve_blocking がディレクトリエントリを無視するため印がそのまま適用遅延の仕組みになる（注記方式は運用の紳士協定になり sqlx が強制しないため選ばなかった）。
4. 足りなかった情報・検証手段: acceptance が目視確認のため機械検証がなく、代わりに tests/migration_rules.rs で（a）文書の必須要素の存在（b）deferred/ が移行ソースに現れないこと（sqlx の挙動、DB 不要）を確認した。mise run verify は scripts/tests/agent-sandbox.test.sh のみ失敗（この sandbox の読み込み専用 $HOME で sbx-plug-test ディレクトリが作れない環境起因。本変更は pelway-db の文書とテストのみで関連しない）。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「よくある失敗」に「agent-gh が exit 3 で失敗するときは DNS（/etc/resolv.conf のリンク切れ）と ~/.cache の読み込み専用を疑う。HTTPS_PROXY のユーザーランドプロキシと XDG_CACHE_HOME の退避で回避できる」

使ったスキル: なし（輸入スキル tdd は試行の割り当てではないため未使用。受け入れ条件の目視確認に対し、機械確認できる seam として「文書の必須要素」と「sqlx の移行ソースの解決結果」の 2 点を選んだ）
