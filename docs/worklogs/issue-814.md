# Issue #814 の作業ログ

## 実装者メモ（round 1）

対象 head: 54f8372417c8b297615f3a48cb8f0e94c0c00364

1. 詰まった箇所: 手順 4: 受け入れ条件の「合成データのテストで 4 件の扱いを確かめる」の主語が曖昧で、見張り（書き出し側）と指揮役（読み取り側）のどちらを試験するかで迷った。読み取り側は LLM エージェントの手順なので決定的に試験できず、`scripts/board-snapshot-read` という検証用の読み取り手順を新設してそこに 4 件の分岐（正常・stale・broken・missing）を持たせる設計にした。
2. 曖昧だった文: 「巡回はボードの Todo を `gh project item-list` の返却順（全件取得、上限で打ち切らない）で読み、`ready-for-agent` 付きを並び順に割り当てる。」（docs/agents/grokbot-runbook.md「2. ボードと巡回」）
3. 仮定と解釈: 「全件取得、上限で打ち切らない」はスナップショットの取得側の要件でもあると解釈し、`item-list --limit 1000` の打ち切りを検知（`totalCount` が件数を超える）したら公開しない fail-closed にした。案 A を選んだ理由: 書き込みキューと同じ中継の型で秘密を指揮役の環境に入れず、案 B と違い読み取りの鮮度が巡回ごとに自立的に更新され、案 C と違いトークンの発行・保管（人間の作業）を要しない。採らなかったもの: 案 B（送り手のルーティンが別途要る）、案 C（secret の sandbox 持ち込み）。
4. 足りなかった情報・検証手段: `gh project item-list --format json` の実際のキー（`labels`・`assignees` が文字列の配列かオブジェクトか）は、人間名義の gh 認証がこの環境に無く実測できなかった。cli/cli の利用例から文字列の配列と読み、オブジェクト形も正規化で受け付ける防御的な書き方にした。
5. 改善案: docs/agents/grokbot-runbook.md の「ボード読み取りスナップショット」に「`labels`・`assignees` の生の形が gh の版で変わったときは、見張りの正規化（names）とこの節の項目の説明を両方直す」を足すとよい。

使ったスキル: implement-issue。輸入スキルはなし（試行の割り当てではない）

push 後の修正: スナップショットの鮮度判定が `now - fetched < interval` だけで、未来の fetched_at を「新しい」と扱ってしまい、reader が broken と読む書き出しを見張りが更新しなくなる問題に気づき、回帰テスト（future fetched_at is refetched。実装なしで FAIL、実装で ok）を足して `0 <=` の条件に直した（54f8372）。
