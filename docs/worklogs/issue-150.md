# Issue #150 の作業ログ

## 実装者メモ（round 1）

対象 head: 07b9c04ee1ca53d7a6133a7d505b6de3fd5f0859（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: scripts/pgbackrest-verify/verify.sh phase_expire_count:
   差分を全日分のフルの後に作ると、postgres の時計を -17 日に偽っていても
   差分はリポジトリ内の最新フル（当日分）に紐づき、expire --set で古い
   フルを消しても差分が残って expire-count-dependent が失敗した。差分を
   対象フルの直後（次のフルができる前）に挟んで解決。
2. 曖昧だった文: 「ハーネスに、件数指定のリポジトリで日次バックアップが
   数日欠けた状況を作り、`expire --set` で 14 日を超えた世代を消す段階を
   足す」（Issue #150「受け入れ条件」）— 「数日欠けた状況」の作り方
   （欠落の位置・世代数）が曖昧で、フルちょうど 14 世代（件数上限）で
   7 日分の欠落という境界ケースを選んだ。
3. 仮定と解釈: 仮定: expire-count の追加位置は `all` の retention-time の
   後（保持系の検証をまとめる）。latest-label-repo1.txt を終端で最新残存
   ラベルに更新すれば下流フェーズ（delete-reapply / restore-checksum）は
   壊れないと解釈し、`all` を実際に回して確かめた。選ばなかった: 末尾に
   置く（下流との結合は避けられるが保持検証が分断される）。
4. 足りなかった情報・検証手段: pgBackRest が差分の基にするフルの選び方
   （取得時点でリポジトリに存在する最新フル）。ハーネスの失敗で実測し、
   docs/research/pgbackrest-retention-restore-verification.md §2.5 に書いた。
5. 改善案: .agents/skills/write-issue/SKILL.md の受け入れ条件の書き方に
   「検証系 Issue では再現したい状態のパラメータ（欠落の長さ・件数）を
   書く」を足すとシナリオ設計の解釈の幅が狭まる。

使ったスキル: implement-issue
