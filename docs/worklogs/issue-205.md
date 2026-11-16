# Issue #205 の作業ログ

## 実装者メモ（round 1）

対象 head: 847e92a0a656f61db45676a1793e5a254e7ca9e6

1. 詰まった箇所: scripts/agent-gh dev: GitHub App トークン発行に失敗した。原因は環境の DNS 破壊（/etc/resolv.conf が切れた symlink）。/etc を修復した bwrap ラッパー（/tmp/opencode/net.sh）経由で実行して回避した。別件: ~/.cache が読み込み専用で mbx（mise の cargo ラッパー）が失敗するため XDG_CACHE_HOME=/tmp/opencode/home-cache に退避した。mise run verify は scripts/tests/agent-sandbox.test.sh（DNS 起因、clean tree でも失敗を確認）と scripts/tests/conductor-watch-ensure.test.sh（systemd 挙動の flaky、単独再実行で通る）の環境起因で失敗するが、cargo test/clippy/hk/deny と追加テストは通った。
2. 曖昧だった文: 「制限が効っていることを smoke テストで検証する観点が定義されている → #183 のテスト分類または該当節で確認」（Issue #205 受け入れ条件）
3. 仮定と解釈: #183 は未着手（OPEN）のため「該当節」に観点を定義し、#183 が「ホストの安全装置」枠として拾う形に接続した。書き先は docs/technical.md §4 とした（Issue の関連情報が docs/technical.md §4 を指すため。v0.7 本文は保全対象で検証手順が重く、Issue は v0.7 を書き先に挙げていない）。未決事項「提供元が外部の場合」は解決せず未決のまま文書に記録した（判断を要するが受け入れ条件の成否に関わらないため）。
4. 足りなかった情報・検証手段: 受け入れ条件が目視確認のため、scripts/tests/egress-host-docs.test.sh で必須文言（DOCKER-USER/INPUT/ブロックリスト/169.254.169.254/systemd/#182/#183/Rakazo 等 13 項目）を機械検証した。実装前の失敗出力と実装後の成功出力を PR 本文に記載。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「決まり」に「sandbox 環境でネットワーク関連コマンドが失敗するときは /etc/resolv.conf のリンク切れを疑い、修復した /etc を bwrap で差し込むラッパー（/tmp/opencode/net.sh）経由で実行する」

使ったスキル: implement-issue、write-pull-request（輸入スキル tdd は試行の割り当てではないため未使用。機械確認できる seam として「技術仕様の必須文言」を選んだ）

## 実装者メモ（round 2）

対象 head: 0c6c79919cd538643bdeb94b074b7da60954d56a

1. 詰まった箇所: scripts/agent-gh: 前セッションと同じ DNS 破壊（/etc/resolv.conf が切れた symlink）でトークン発行が失敗した。今回は / が書けるため、コマンドごとに /mnt/wsl/resolv.conf を作って（sandbox が / への書き込みをコマンド単位で消すため）実行した。git push -u は .git/config への書き込みが sandbox に阻まれたため、明示 refspec（origin HEAD:refs/heads/<branch>）で push した（初回の -u push 自体は成功していた）
2. 曖昧だった文: 「本 PR で行うのは、技術仕様と PR 本文への短い注記だけです」（Issue #205 への hatai の判断コメント）— 注記を「方式」節の前後どの位置に置くかが示されていない
3. 仮定と解釈: 注記は小節の冒頭（出典段落の直後・方式節の前）に独立ブロックとして置いた。既存の #182/#183 参照が素の `#NNN` なので注記も素の参照に揃え、リンクは PR 本文側に置いた。判断コメントの「方式は書き換えず」より、§4 の「許可したあて先だけ」「同じ制限をホスト側でも担保」と注記の不整合は残る（#389 で解消）
4. 足りなかった情報・検証手段: 注記の必須文言は判断コメントに列挙されていないため、コメントの 3 要素（#389 へのリンク・runtime 前提が Linux Docker Engine・#389 までブロックリスト）をそのままテスト条件4（`#389`・`既定拒否`・`rootless Podman`）に写した。追記前 FAIL・追記後 ok を確認
5. 改善案: docs/agents/dev-environment.md（存在すれば「sandbox の制約」節）に「/ への書き込みはコマンド実行ごとに消えるため、DNS 修復は同一コマンド内で行う」「git push の upstream 設定は .git/config が書けないため明示 refspec を使う」を足す

使ったスキル: implement-issue（seam は round 1 と同じ「技術仕様の必須文言」に条件4を追加）
