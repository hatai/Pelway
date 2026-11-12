# Issue #102 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-audit-checkpoint/Cargo.toml: `rand_core = "0.64"` が解決失敗（`^0.64` は 0.64.x の意味で、実在するのは 0.6.4）。同じく ed25519-dalek 2.2.0 では `rand_core` が既定機能外で `SigningKey::generate` が生えず、features 追加が必要だった。
2. 曖昧だった文: 「チェックポイントの形式（C2SPのsigned-noteの署名形式を参考にする）を決める」（Issue #102 のやること）— 「参考にする」が署名形式だけか、checkpoint 本文の tlog-checkpoint 互換（Merkle 根）まで含むかが決まっていない。
3. 仮定と解釈: 仮定: docs/research/oss-data-and-collaboration.md §6 の「線形鎖の最新値と件数を signed-note で署名する最小形から始めるのが妥当」が hatai の承認済み方向。解釈: 署名形式は signed-note に従い、本文は tlog-checkpoint にならった origin/行数/最新ハッシュの 3 行（Merkle 根ではなく `prev_hash` 鎖の最新値、拡張行なし）とした。選ばなかった: Merkle 木（tlog-tiles 互換）と azul `signed_note` crate 依存（比較は ADR 0007 に記録）。
4. 足りなかった情報・検証手段: `cloudflare/azul` のライセンス（調査書の未確認事項）— LICENSE ファイルを直接確認して BSD-3-Clause＋ISC と判明した。実 PostgreSQL での `audit_events` DDL（04 は表の項目しか定めない）— 下書きクレートは行列を抽象データとして扱い、DB 結合は製品組み込み時に残る。
5. 改善案: docs/research/oss-data-and-collaboration.md の「未確認事項の一覧」の「`cloudflare/azul` のライセンス。」を「`cloudflare/azul` のライセンス: BSD-3-Clause＋ISC（LICENSE ファイル確認済み、ADR 0007）。」に更新する。

## 実装者メモ（round 2）

1. 詰まった箇所: crates/pelway-audit-checkpoint/src/lib.rs: `store` を PR-2 へ分離するとき、tests/tamper.rs の一時ファイルへルパー（`tmp_log`）と ADR 0007 の保管先の節も一緒に外さないと、未使用のモジュール参照が残る。
2. 曖昧だった文: 「PR 2 は PR 1 のブランチに積んで Draft で出す」（指揮役の Issue #102 へのコメント）— 「積む」が同一ブランチへの追いコミットか、PR 1 のブランチを base にした子ブランチか。
3. 仮定と解釈: 仮定: GitHub の PR は head ブランチと base ブランチの組で決まるため、同一ブランチからは 2 本作れない。解釈: PR-2 は PR-1 のブランチから切った子ブランチにコミットを積み、base を PR-1 のブランチにする。選ばなかった: 同一ブランチへの追いコミット（2 本目の PR が作れないため）。
4. 足りなかった情報・検証手段: 積み上げ PR の行数測定 — scripts/pr-size.sh は `--base <ref>` を持つため、PR-2 は PR-1 のブランチを基点に測る（既定の origin/main だと PR-1 の行数も二重に数える）。
5. 改善案: mise.toml の「pr-size」タスクの description に「`--base` で比較基点を変えられる（積み上げ PR は親ブランチを指定する）」を足す。

## 実装者メモ（round 3）

1. 詰まった箇所: docs/adr/0007-signed-audit-checkpoints.md: PR-1 の行数上限のため本文を PR-2 へ移した結果、02-stack-comparison の監査行のリンクが PR-1 では一時的に切れる（#92 の ADR 0004 と同じ扱いにした）。
2. 曖昧だった文: 「PR 1 が追加 800 行を超える場合は、ADR 0007 の本文を PR 2 に移して収める。」（指揮役の Issue #102 へのコメント）— 「本文を移す」が ADR ファイル全体の移動か、決定の節単位の移動か。
3. 仮定と解釈: 仮定: PR-1 を +818 からさらに削ると docs の説明を削ることになる。解釈: ADR ファイル全体を PR-2 に移し、PR-1 は +793 に収めた（リンク切れは PR 本文に明記）。選ばなかった: ADR から形式の節だけ残す部分移動（決定の記録が 2 つの PR に分かれ、ステータスの管理が曖昧になるため）。
4. 足りなかった情報・検証手段: なし（round 1〜2 の情報で足りた。store の RED 出力は round 1 の実行記録に含まれる）。
5. 改善案: .github/pull_request_template.md の「レビューと QA」に「一時的なリンク切れはここに書く（#92 の先行例あり）」を足す。
