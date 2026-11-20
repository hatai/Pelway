# Issue #520 の作業ログ

## 実装者メモ（round 1）

対象 head: 5bf99db055311d64678b15f43ba433f9c8ab0546

1. 詰まった箇所: scripts/agent-gh: `if ! response="$(gh token generate ...)"` の形では `!` 反転後の `$?` が常に 0 になり gh の終了コードを拾えなかったため、`response="$(...)" || rc=$?` の形に変えた。もう 1 件: 初回 `mise run verify` が pelway_http のテストバイナリ欠落（No such file or directory）で失敗したが、単独実行と再実行で通った（ビルド並列時の環境起因。PR 本文に内訳を記録）
2. 曖昧だった文: 「『拡張が必要』という文は、`gh extension list` に gh-token が無いときだけ出す」（Issue #520 受け入れ条件）— `gh extension list` 自体が失敗したときの扱いが書かれていない
3. 仮定と解釈: extension list の失敗時は「gh-token が無い」とは判定できない。判定不能なら案内を出さない（拡張があるのに誤案内する本 Issue の発端が再発するため）と解釈した。採らなかった解釈: list 失敗も「無い」とみなして案内を出す
4. 足りなかった情報・検証手段: Link-/gh-token 拡張が失敗時に出す stderr の実際の書式（Issue コメントにあるのは gh 本体の permission denied の実測のみ）。テストは合成文字列で書いた
5. 改善案: .agents/skills/implement-issue/SKILL.md の「よくある失敗」に「『値が洩れないこと』を確かめる条件は実装前 red にならない性質がある（出す実装と同じ round で初めて意味を持つ）」を足す

使ったスキル: implement-issue、.agents/skills/diagnosing-bugs/SKILL.md（verify 失敗の切り分けに Phase 2 の再現と最小化）

## 実装者メモ（round 2）

対象 head: 53ab69e93a4076c072771eab935377e204a7ed4c

1. 詰まった箇所: scripts/agent-gh: main（PR #524）と #520 が同じ `app_token` を変えていた。main は key_file を local 宣言から外した（subshell の EXIT trap から見えるように）が、私は err_file を local に置いたままだった
2. 曖昧だった文: 「トークン発行そのものが失敗しても、本体の gh は呼ばず exit 3」（scripts/tests/agent-gh.test.sh の main 側テスト no_fallback_generate_fails のコメント）— #520 で追加した診断用の `gh extension list` 呼び出しが「gh を呼ぶ」に当たるか書かれていない
3. 仮定と解釈: 「本体の gh」とは元のコマンド（`gh pr list` 等）の実行のことと解釈した。`gh extension list` は #524 の隔離された空の GH_CONFIG_DIR の中だけで動き hosts.yml を読まないため fail-closed の意図（人間のトークンで続けない）と矛盾せず、診断として残した。merge 後に #520・#522 両側のテストが全て通ることを実測で確認した
4. 足りなかった情報・検証手段: merge コミットが署名付け替え（agent-gh-resign）できないことを示す表示の意味。AGENTS.md の「API で表現できない変更（…マージ…）を含む push は署名なしのままになる」で既定の挙動と確認した
5. 改善案: .agents/skills/implement-issue/SKILL.md の「手順 9 修正依頼」に「merge 後は main 側が追加した同じファイルのテストとの相互作用も通過確認する」を足す

使ったスキル: implement-issue
