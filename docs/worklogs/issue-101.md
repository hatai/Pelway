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
