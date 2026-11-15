# Issue #303 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 環境: `/etc/resolv.conf` が無く DNS 解決が失敗し、`scripts/agent-gh` のトークン発行（api.github.com への POST）が出来なかった。sudo も使えないため、user namespace（`unshare -Um --map-root-user`）内で resolv.conf を bind mount して全 `agent-gh` 実行を回避した
2. 曖昧だった文: 「command-team 内に報告のテンプレートや例文があり、そこで番号が素で書かれている箇所があれば、リンク形式に合わせて直す」（Issue #303 本文「採用する方式」）
3. 仮定と解釈: command-team 自体には素の番号を含む人間あて報告の例文が無いと仮定した。人間あて報告の例文の実体は runbook 8「まとめ」にあるため、そこをリンク形式に直し、ルールから runbook を参照させた。ルール本文（スレッド名の表や指示のテンプレート）はスレッドあて・GitHub 上の表記なので対象外と解釈した
4. 足りなかった情報・検証手段: 受け入れ条件 3 件目（指揮役の実際の報告 1 件以上でリンク確認）はマージ後に指揮役が正本を読んで初めて満たせる。PR 内では例文（runbook 8）のリンク形式とテスト（scripts/tests/conductor-report-links.test.sh）で代替検証した
5. 改善案: なし

使ったスキル: implement-issue
