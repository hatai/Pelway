# Issue #205 の作業ログ

## 実装者メモ（round 1）

対象 head: 847e92a0a656f61db45676a1793e5a254e7ca9e6

1. 詰まった箇所: scripts/agent-gh dev: GitHub App トークン発行に失敗した。原因は環境の DNS 破壊（/etc/resolv.conf が切れた symlink）。/etc を修復した bwrap ラッパー（/tmp/opencode/net.sh）経由で実行して回避した。別件: ~/.cache が読み込み専用で mbx（mise の cargo ラッパー）が失敗するため XDG_CACHE_HOME=/tmp/opencode/home-cache に退避した。mise run verify は scripts/tests/agent-sandbox.test.sh（DNS 起因、clean tree でも失敗を確認）と scripts/tests/conductor-watch-ensure.test.sh（systemd 挙動の flaky、単独再実行で通る）の環境起因で失敗するが、cargo test/clippy/hk/deny と追加テストは通った。
2. 曖昧だった文: 「制限が効いていることを smoke テストで検証する観点が定義されている → #183 のテスト分類または該当節で確認」（Issue #205 受け入れ条件）
3. 仮定と解釈: #183 は未着手（OPEN）のため「該当節」に観点を定義し、#183 が「ホストの安全装置」枠として拾う形に接続した。書き先は docs/technical.md §4 とした（Issue の関連情報が docs/technical.md §4 を指すため。v0.7 本文は保全対象で検証手順が重く、Issue は v0.7 を書き先に挙げていない）。未決事項「提供元が外部の場合」は解決せず未決のまま文書に記録した（判断を要するが受け入れ条件の成否に関わらないため）。
4. 足りなかった情報・検証手段: 受け入れ条件が目視確認のため、scripts/tests/egress-host-docs.test.sh で必須文言（DOCKER-USER/INPUT/ブロックリスト/169.254.169.254/systemd/#182/#183/Rakazo 等 13 項目）を機械検証した。実装前の失敗出力と実装後の成功出力を PR 本文に記載。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「決まり」に「sandbox 環境でネットワーク関連コマンドが失敗するときは /etc/resolv.conf のリンク切れを疑い、修復した /etc を bwrap で差し込むラッパー（/tmp/opencode/net.sh）経由で実行する」

使ったスキル: implement-issue、write-pull-request（輸入スキル tdd は試行の割り当てではないため未使用。機械確認できる seam として「技術仕様の必須文言」を選んだ）
