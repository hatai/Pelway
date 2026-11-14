# Issue #269 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: `scripts/demo-check/loopback.sh` の runtime ループ: `while IFS= read -r _st _rq _sq laddr _peer` の `IFS=` が単語分割を無効化して行全体が `_st` に入り、`laddr` が空のまま "checked 0" になった。`IFS=` を外して修正（1 変数の read とは効きが違う点が盲点だった）
2. 曖昧だった文: 「慣れない機能なら、AGENTS.md の『Prior art first』に従い、方式を人間に選んでもらってから『採用する方式』に書く」（.agents/skills/write-issue/SKILL.md 手順1）— 本 Issue は起票者と実装者が同一セッションのため、方式比較（/proc 直読み vs ss パース）をスレッド上で提示して選択（b: ss パース）を受け、Issue の「採用する方式」に反映した
3. 仮定と解釈: runtime 検査は `DEMO_CHECK_RUNTIME=1` でのみ有効（未設定時は従来どおり PENDING・ACCEPTANCE 時は PENDING 失敗）。デモの起動方法が未決のため、有効化スイッチ方式と解釈した。選ばなかった /proc 直読み（依存ゼロ・テスト容易）も有力だったが、選択結果を優先した
4. 足りなかった情報・検証手段: 実デモの起動方法が決まっていないため、デモ受入時の runtime 有効化をどのタイミングで入れるかは未決（Issue #269 の追跡対象）。実 ss での確認は開発機の常駐ソケットが多いため「違反を列挙して exit 1 になること」の確認にとどまる
5. 改善案: `docs/agents/issue-tracker.md` に「起票者と実装者が同一セッションで方式選択した場合も、比較と選択の経緯を Issue コメントに残す」の一行を足す

## 検証記録（round 1）

- 安全弁（bind scan did not advance）の発火検証: `loopback.sh` の `nxt` 計算を一時的に `nxt="${rest}"` に改変し、bind を含む合成ツリーで実行 → `loopback: bind scan did not advance at crates/x/src/a.rs:1` と exit 2 で即時停止（無限ループ・多重報告なし）。復元後 `git diff`・`git status` とも空、selftest 131 passed, 0 failed（当時の件数）で緑に戻ることを確認
- main 最新（33c7818）での `bash scripts/demo-check/run.sh` → exit 1（4 段階すべて FAIL。ヒット内訳は Issue #269 のコメント参照）
- オフラインビルド: `mise exec -- cargo check/build --offline --workspace --locked` とも 0（mbx キャッシュ、ダウンロード 0B）。`target/debug/` に ime・pelway-agent-watch・pelway-dev-llm-eval・pelway-gateway-echo・perf 生成
- canary 実データ走査: `bash scripts/demo-check/canary.sh` → **scanned 13802 file(s), 0 hit(s), exit 0**（target は mbx キャッシュへのリンクで、17:56 決定の例外が実環境で作動）
- runtime 実装後の selftest → 0（**135 passed, 0 failed**。runtime 4 ケース追加: 違反 exit 1・ループバックのみ exit 0・ACCEPTANCE で PENDING 規則なし・ss 失敗 exit 2。いずれも ss スタブで固定）
- 実 ss（iproute2 6.1.0）での動作確認: `DEMO_CHECK_RUNTIME=1 bash scripts/demo-check/loopback.sh` → 開発機の非ループバック LISTEN（10.255.255.254:53・0.0.0.0:22・tailscale 等）を列挙して exit 1。検出は実環境で機能している
