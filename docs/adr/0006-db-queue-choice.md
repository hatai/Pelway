---
status: accepted
---

# DBキューは自作の`FOR UPDATE SKIP LOCKED`+leaseとし、Riverは将来の候補に置く

非同期実行の基盤として、自作の`FOR UPDATE SKIP LOCKED`+leaseによるPostgreSQL上のキューを採る。River（Rust実装のcrate `riverqueue` 0.6.0）は、同一トランザクション投入などPelwayの必須条件を満たすが、Rust実装の成熟度がまだ低いため、採用せず「将来の候補」として置く。検証はIssue #99として、`crates/dev-queue-compare`の結合テスト（PostgreSQL 16.15、rootless Podman上）で行った。`LISTEN/NOTIFY`は起こす合図に限り、正本はジョブ表の行とする方針（02 §調査を受けた採否）が前提。

## 比較の結果

- **同一トランザクション投入**（状態・監査・outboxと同じトランザクションでジョブを登録できるか）: 両者とも可能。自作は業務トランザクション内の`INSERT`だけで、rollbackすればジョブも業務行も消え、commit前に他の接続からは見えないことを試験で確認した。Riverも`client.insert().tx()`が同じ性質を持つことを同じ形の試験で確認した。差は無い。
- **lease・再実行・停止**: 自作は約100行のSQLで、SKIP LOCKEDによる排他（並行2 workerで各ジョブをちょうど1回だけ処理）、lease期限切れの回収、`max_attempts`での打ち切り（Requeued/Dead）、停止要求（未実行は即時cancel、実行中は`cancel_requested`を副作用の前に拾う）を試験で確認した。Riverはこれに加えてretryポリシーの上書き、Snooze、一意ジョブ、定期ジョブ、soft stopを持つ。ただし実行中ジョブの外部cancelは、通常はNOTIFYで約1秒でworkerのcancellation tokenに届くが、試験中に1回、合図が届かずジョブが実行し続けた（lost notificationの定期補償は既定で1分間隔）。この観測を含め、Riverの停止の扱いは「隅まで理解するコスト」が自作より高いと判断した。
- **`LISTEN/NOTIFY`の取りこぼし**: 自作キューで、合図はcommitと同時にLISTENしている接続へ届き、LISTENしている者を消しても（=取りこぼしても）定期の追いつき検索（`dequeue`のポーリング）だけでジョブを処理できることを試験で確認した。02のserver間通知の行と整合する。
- **成熟度**: crates.ioの`riverqueue`は0.6.0、累計ダウンロード2,754件（直近は1,270件）。sqlx ^0.9専用（0.8系とは共存できない）、MSRVは1.95。Go本体（v0.51.2、2026-11-09リリース）は成熟しているが、Rust実装は2026年時点でまだ若い。MPL-2.0はファイル単位のコピーレフトで、改変しない限りPelway本体への影響は小さい。

## Consequences

- 製品のジョブキューは自作のSKIP LOCKED+leaseで実装する。`crates/dev-queue-compare`の`devq_`スキーマと関数群は検証用のたたき台であり、製品には製品のマイグレーションと命名で組み込む。
- 定期追いつき検索の間隔、leaseの長さ、再実行のバックオフの値は製品設計で決める。取りこぼし補償の前提上、合図を欠落しても正しさは失われない。
- Riverは次の条件が揃った時点で再評価する: Rust実装の利用実績の増加、sqlx 0.8/0.9の両対応またはエコシステムの0.9移行完了、実行中cancelの通知取りこぼしの扱いの改善。再評価の土台として、この検証ハーネスと試験を残す。
