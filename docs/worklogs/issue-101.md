# Issue #101 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: scripts/pgbackrest-verify/Dockerfile: `apk del .build-dependencies` が
   依存もまとめて消し、実行時に必要な `libbz2.so.1`（Alpine 3.24 のパッケージ名
   `libbz2`）が消えて `pgbackrest` が起動しなかった（`Error loading shared
   library libbz2.so.1`）。実行時依存をビルド依存の削除より後の別 `RUN` に
   分離して解決した。
2. 曖昧だった文: 「件数での指定か別の削除処理で、復旧できるバックアップを最大14日に
   保てるか確かめる」（Issue #101 のやること）— 「別の削除処理」が
   pgBackRest のどの操作を指すか（`expire --set` による個別削除か、自前の定期
   処理か）が決まっていない。
3. 仮定と解釈: 仮定: 保持の判定は pgBackRest の `repo-retention-full-type` と
   `repo-retention-full` の組合せで決まるので、上限を守るには日数を明示するより
   件数を指定する方が確実である。解釈: 「別の削除処理」は
   `pgbackrest expire --set=<ラベル>`（pgBackRest が持つ個別削除の操作）と
   した。日数指定（time 型）で残る 14 日超の世代をこれで消せることを実測した。
   選ばなかった: 自前の定期処理（削除の主体が pgBackRest の外に出るのは
   13 章の「残存失敗を隠さない」に反し、pgBackRest の expire と二重に
   管理することになるため採らない）。
4. 足りなかった情報・検証手段: 「何日前のバックアップ」を作る手段。pgBackRest の
   フルバックアップのタイムスタンプは DB サーバの時計由来
   （`src/command/backup/db.c.inc` の `backupTime` はオンライン時 `dbTimeMSec`
   を使う）ので、expire 側（`time(NULL)`）だけ偽っても足りない。postgres 自体を
   libfaketime 付きで起動し直す運用にした。
5. 改善案: docs/research/oss-data-and-collaboration.md の「pgBackRest　判定：MVP前に検証」の
   「Pelway への当てはめ（Pelway 側の評価）」の「時間指定の保持は『14 日を超えない
   こと』ではなく『最低限の復旧範囲を残すこと』を保証する設計です」の記述を、
   実測（time 型は 14 日超の 1 個が残る）に基づいて「実測でも 14 日超の世代が
   残ることを確認した。時間指定は下限（復旧範囲の最低限）を保証する設計で、
   上限は保証しない」に更新する。

## 実装者メモ（round 2）

1. 詰まった箇所: scripts/pgbackrest-verify/verify.sh: container exec と復元手順を
   rootx / postx / clear_pgdata / chown_pgdata / restore_pgdata に共通化したが
   正味 ±0 行（ヘルパー定義の増分が呼出し側の削減と相殺）。pr-size は +935/-0 のまま。
2. 曖昧だった文: 「重複の除去で収まらなければ、その時点で止まる」
   （.agents/skills/implement-issue/SKILL.md「止まる条件」）— 共通化しても
   正味の行数が減らない場合を「収まらない」と判定してよいか決まっていない。
3. 仮定と解釈: 仮定: 「重複の除去」は行数を減らすための手段であり、±0 の整理を
   続けても条件は満たされない。解釈: 共通化を一通り行い実測が変わらない時点で
   「収まらない」と判定して止まる。選ばなかった: 説明やチェックを削って行数を
   合わせる（skill が明示的に禁じる）。
4. 足りなかった情報・検証手段: 「正味の行数」の試算は
   `git diff origin/main...HEAD --stat` で代用した（`mise run pr-size` は
   コミット済み差分だけ数えるため、試行のたびにコミットが要る）。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「止まる条件」の表の
   pr-size の行に「重複の除去は正味の行数が減る範囲で行い、それでも超過なら
   止まる」と判定基準を足す。

## 実装者メモ（round 3）

1. 詰まった箇所: なし。分割は指揮役の表どおりに進んだ。stacked PR の行数は
   `scripts/pr-size.sh --base <下段ブランチ>` で数える（issue-93 のメモで既知）。
2. 曖昧だった文: 「PR 2 は PR 1 のブランチに積み、PR 1 のマージ後に base を
   main に替えます」（Issue #101 の `<!-- split-plan -->` コメント、作成者
   hatai）— 「積む」が PR 2 の base を PR 1 ブランチにする stacked PR を指す
   ことは文脈で分かるが明記はされていない。
3. 仮定と解釈: 仮定: 既存ブランチ agent/101-pgbackrest-retention-restore の
   履歴を保つ必要はなく、2 本の新しいブランチに内容を移してよい。解釈:
   PR 1 = ハーネス、PR 2 = レポート＋作業ログで、PR 2 の base は PR 1 の
   ブランチ。Closes は PR 2 だけに付け、PR 1 は「#101 の分割の 1 本目」と
   参照にとどめる（PR 1 のマージで Issue が閉じると PR 2 が宙に浮くため）。
   選ばなかった: 既存ブランチを reset --hard して force push（push 済み履歴の
   書き換えを避ける）。
4. 足りなかった情報・検証手段: なし。CI の pr-size は github.base_ref を見るため
   stacked でも正しく数えることを .github/workflows/verify.yml の pr-size
   ジョブで確認した。
5. 改善案: .agents/skills/implement-issue/SKILL.md の手順 6 に「stacked PR では
   mise run pr-size ではなく scripts/pr-size.sh --base <下段ブランチ> で数える」
   を足す。
