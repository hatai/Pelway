# Issue #265 の作業ログ

## 実装者メモ（round 2）

対象 head: c02861081ac69230f8c467b0fba76bb19f0dc475

1. 詰まった箇所: scripts/demo-check/lib.sh ほか: コマンドと severity が固定のため、SC1091（source 追跡不可）×4・SC2154（files 未代入）×3・SC2034・SC2016 をフラグ無しで消す手段が .shellcheckrc と disable コメントのどちらかになった
2. 曖昧だった文: 「`shellcheck scripts/demo-check/*.sh` が指摘 0 件で exit 0（severity は既定のまま。個別の抑制は `disable` コメントに理由を添えて許可）」（Issue #265 本文の受け入れ条件）
3. 仮定と解釈: （仮定）「severity は既定のまま」は -x 等のフラグ追加も不可の意味。（解釈）SC1091/SC2034/SC2016 は理由付き disable、SC2154 は files=() の実初期化（dc_load_files が埋める契約の明示）で解消。選ばなかった解釈: .shellcheckrc の external-sources=true（source= の相対解決が cwd 依存になり、実行場所で壊れるため検証済み）
4. 足りなかった情報・検証手段: 開発機で scripts/tests/conductor-watch-ensure.test.sh が本 diff と無関係に落ちることの切り分け手段（git stash -u した素の main でも再現、失敗箇所は実行ごとに変動 = 負荷依存の race。CI の GitHub runner が正の検証）
5. 改善案: docs/dev-environment.md の「CI（GitHub Actions）」に「scripts/tests/conductor-watch-ensure.test.sh は開発機の高負荷で落ちることがある（失敗したら単体再実行と素 main での再現で切り分ける）」を追記する

使ったスキル: implement-issue
