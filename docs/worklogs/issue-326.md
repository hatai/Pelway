# Issue #326 の作業ログ

## 実装者メモ（round 1）

対象 head: 35a3ffd599dfc194e6fda806dd1408c8c0883864

1. 詰まった箇所: 手順 1・6: sandbox 内で `/etc/resolv.conf` が壊れた symlink（リンク先 `/mnt/wsl/resolv.conf` がマウント名前空間に無い）で `scripts/agent-gh` と mise の rustup 同期が DNS 失敗。`mkdir -p /mnt/wsl && printf 'nameserver 127.0.0.53\n' > /mnt/wsl/resolv.conf` を各 exec の先頭で実行して回避（worklogs issue-301・303 と同じ手口）。mbx が `~/.cache` に書けず `cargo` が失敗 → `XDG_CACHE_HOME=/tmp/xdg-cache` で回避（issue-256 と同じ）。`scripts/tests/agent-sandbox.test.sh` は変更前から環境起因で失敗（issue-301 記載どおり `~/.t3/worktrees/` への mkdir が拒否）。
2. 曖昧だった文: 「起動時に最小 seed（チャンネル、メンバー、過去の会話、P03 根拠）が用意される」（Issue #326「受け入れ条件」）
3. 仮定と解釈: 「起動時」は `DataStore::new_demo_seeded(now)` が起動経路（T1b・#336）から呼ばれる形と解釈。`serve` 内蔵の無条件 seed は選ばなかった（gdpr02 の仮経路が空の store を前提にする挙動を変えるため）。チャンネル名は `#` 無しの `案件B`（Issue の表記はチャンネル呼称で、name フィールドは `#` を持たない既存値「合成チャンネル」に合わせた）。「P03 根拠の登録の形は既存の仕組みに合わせる」は、`create_basis_record` 経由で作った記録を seed 内でだけ status=Active・effective_at=now に確定する形と解釈（Active を作る公開経路は増やさない）。
4. 足りなかった情報・検証手段: `client_msg_id` の重複キーは Issue に無いため、04 §2 の `unique(workspace,author,client_msg_id)` を採用（別 author の同 id は別投稿）。冪等再送の判定は要求の内容（チャンネル・目的・本文・スレッド）が全て同じとき同じ結果、違えば `StoreError::Conflict`（04 §2 の 409 相当）。
5. 改善案: `docs/dev-environment.md` の「実装スレッドの sandbox（Issue #151）」に「sandbox 内で DNS を使うコマンド（agent-gh・mise）は /mnt/wsl/resolv.conf の用意が要る。mbx のキャッシュは XDG_CACHE_HOME を /tmp 配下へ退避する。scripts/tests/agent-sandbox.test.sh はネスト sandbox 内では環境起因で失敗する」旨を足す。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）
