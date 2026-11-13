# Issue #93 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: `scripts/ja-fts-bench/Dockerfile.pgroonga`: `podman build`（rootless ネットワーク既定）内の Groonga バンドル `download_mecab.rb` が packages.groonga.org へ接続タイムアウト。`--network host` で解決。他に `podman rm -tf` が podman 4.9 では `-t` が数値フラグのため失敗する（`rm -f` に修正）、`pgroonga_score(id)` は PGroonga 4 では行型（`pgroonga_score(m)`）が必要、`ALTER SYSTEM SET pgroonga.enable_wal` は PGroonga の .so をロードしたバックエンドでしか GUC が登録されず同一 psql セッション内で検索を1回呼んでから実行する必要があった。
2. 曖昧だった文: 「PGroongaのWALと復元（`pgroonga.enable_wal`、REINDEX）がR15のバックアップ訓練で戻るかを確かめる」（`.github` 上の Issue #93 本文）と「公式の例では『pp』で『Apple』は当たらない」（`docs/research/japanese-full-text-search.md` §5 選択肢A）。
3. 仮定と解釈: R15 の「バックアップ訓練」は日次 snapshot と復元演習（`docs/technical/v0.7/body/13-operational-defaults.md` の R15 行）なので、pg_dump（論理）と pg_basebackup（物理）の復元、kill -9 クラッシュ再起動を訓練相当と解釈した。REINDEX は索引が壊れた場合の復旧手段として、壊れなかった場合も「reindexed=no」と記録して残す。調査ドキュメントの「pp で当たらない」は索引を明示指定（`pgroonga_condition(index_name => ...)`）して検証したところ再現した（両索引が存在する既定ではプランナが SplitSymbolAlpha 側を選びヒットして見える）。
4. 足りなかった情報・検証手段: PGroonga 4.1.0 の GUC 一覧（`pg_settings` に現れない）と Groonga 16 のトークナイザー名（`TokenBigramSplitKeyword` は存在せず `TokenBigramSplitSymbolAlpha` が正しい）。`SELECT pgroonga_command('tokenizer_list');` とセッション内 `SHOW` で確認した。
5. 改善案: `docs/research/japanese-full-text-search.md` の「§5 選択肢A」の「TokenBigramSplit 系か TokenNgram の unify 系オプションを使う」に「Groonga 16.1.3 で実在するのは TokenBigramSplitSymbol / SplitSymbolAlpha / SplitSymbolAlphaDigit（tokenizer_list で確認）」を追記する。

## 実装者メモ（round 2）

1. 詰まった箇所: `mise run pr-size`: 全成果物（+926）が追加 800 行の上限を超えた。検索速度と ACL 込み検索の測定ループを `timed_search` に共通化して削ったが収まらず、分割案を Issue に書いて止まった。
2. 曖昧だった文: 「重複の除去で収まらなければ、その時点で止まる」（`.agents/skills/implement-issue/skill.md` の止まる条件の表）。
3. 仮定と解釈: 止まった後に指揮役から「分割案のとおり 2 本に分けて進める」指示が届いたので、ベンチマーク基盤（PR 1）と計測結果レポート＋作業ログ（PR 2）に分けた。選ばなかった解釈: 1 本の PR にまとめて `pr-size-exception` を求める続行（指揮役の判断を待つ必要があり、指示は分割だった）。
4. 足りなかった情報・検証手段: `scripts/pr-size.sh --base <ref>` を使えば stacked PR（PR 2 の base を PR 1 ブランチにする）の差分行数を正しく数えられることを、分割後に確認した。
5. 改善案: `scripts/pr-size.sh` の「options」の説明に「 stacked PR では --base に下段のブランチを指定する」を追記する。
