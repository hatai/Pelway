# Issue #264 の作業ログ

## 実装者メモ（round 1）

対象 head: 65d732dabdf735fdef8f38686b1b549a4f6459f5

1. 詰まった箇所: scripts/agent-gh: この実行環境の /etc/resolv.conf が存在しない /mnt/wsl/resolv.conf を指し DNS が全滅していた（/etc は読み取り専用）。mount namespace で /etc/resolv.conf を差し替えて実行するラッパーで回避。また /tmp/opencode 直下のラッパーが並行セッションに書き換えられたため専用パス（/tmp/opencode/i264/）に再作成した
2. 曖昧だった文: 「発火条件を合成ツリーで再現できるかを検討し、可能なら selftest.sh にケースを追加して固定する。」（Issue #264 本文）。「可能なら」の判定根拠が無く、不可能だった場合の成果物の形が示されていない
3. 仮定と解釈: 検討の結果、BASH_REMATCH[0] は常に rest のリテラル部分文字列であり ${rest#*"${m}"} は入力内容（グロブ文字・バックスラッシュ・マルチバイト・無効 UTF-8 を含む）によらず必ず前進するため、安全弁の exit 2 経路は合成ツリーでは到達不能、と解釈した。到達不能の事実を loopback.sh のコメントに記録し、代わりに安全弁が守る対象（終了性と報告回数）を、tricky 引数（マルチバイト・グロブ・バックスラッシュ・許可+違反の同一行）の新ケースで固定した。安全弁そのものを発火させるテストフックを製品スクリプトに追加する選択は採らなかった（検査専用の経路を検査対象に入れることは誤検出リスクが上回るため）
4. 足りなかった情報・検証手段: mise run verify はこの sandbox では完全緑にできない（読み取り専用ホームのため scripts/tests/agent-sandbox.test.sh が /home/hatai/sbx-wrap-home 等を書けず、conductor-watch-ensure.test.sh が systemd/kill 挙動の差で失敗。いずれも本 diff が触れない領域）。cargo test・clippy・deny-check・hk・fmt は非 root の uid に map して全緑、demo-check selftest は 136/136、わざと長さ基準の前進に壊した RED（tricky ケース 12 回報告）も確認済み
5. 改善案: write-issue skill の「受け入れ条件の書き方」に「条件が『可能なら』のような条件付き目標のときは、不可能だった場合に確かめることを受け入れ条件として並記する」を足す

使ったスキル: implement-issue（tdd は未指定のため未使用。新ケースの RED は実装の意図的破壊（長さ基準の前進への差し替え）で確認）
