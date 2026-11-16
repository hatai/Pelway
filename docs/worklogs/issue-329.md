# Issue #329 の作業ログ

## 実装者メモ（round 1）

対象 head: `3880967`（分割後の後段 PR-2、ブランチ `agent/329b-runner-http`。前段 PR-1 は `4d6b4d8`・ブランチ `agent/329a-runner-lite`。全体を 1 コミットで作ったあと `pr-size` の +800 上限で停止し、hatai の split-plan 承認（Issue コメント）で 2 本に分解した）

1. 詰まった箇所: 手順 4: `LlmGateway` が `impl Future` を返すため `dyn` 化できず、`chat` を `Pin<Box<dyn Future>>` に包む `DynChatGateway` trait（`LlmGateway` へのブランケット実装つき）を runner 側に置いた。環境の DNS が壊れており `/tmp/connect-proxy.py`（127.0.0.1:19191）+ DoH 経由で GitHub だけ疎通した。cargo は `CARGO_NET_OFFLINE=1` で実行。
2. 曖昧だった文: 「LLM の失敗時は「止まっていることと理由」が投稿され、フォールバック応答へ差し替えられる」（Issue #329 受け入れ条件）
3. 仮定と解釈: 「差し替えられる」は gateway の交換ではなく停止中タスクへの応答の差し込みと解釈し、`Runner::provide_fallback(task_id, reply)` で Stopped → 案投稿 → 判断待ち → AwaitingDecision と進める形にした（gateway 交換＋再試行という解釈は選ばなかった）。判断待ちの `decider` は依頼者ではなく runner 設定の固定 1 人（#337 のデモで山本が決定するため `RunnerConfig.decider`）。「進捗の1行（固定段階）」は受付段階の固定文 `依頼を受け付けました。A/B 案を作成しています。` とした。
4. 足りなかった情報・検証手段: runner-lite の置き場所の指示が Issue に無いため新クレート `pelway-runner` を切り、`pelway-http` が投稿経路から呼ぶ形にした。T3a の「判断待ち一覧に出る」ことは `pending_decisions` で確認したが、画面での表示確認は #331 の UI 側。
5. 改善案: `AGENTS.md` の「Agent workflow」に「`scripts/agent-gh` の argv に非 ASCII を渡せない（`--body-file` / `api --input` を使う）」と「この環境で GitHub に出るには `HTTPS_PROXY=http://127.0.0.1:19191` を付ける」ことを足す。

使ったスキル: implement-issue（`.agents/skills/implement-issue`）

## 実装者メモ（round 2）

対象 head: `3b85542`（PR-1 のレビュー修正コミット、ブランチ `agent/329a-runner-lite`）

1. 詰まった箇所: PR-1 は pr-size +800 ちょうど近く、must-fix の追加行を入れるため既存アサーションの再圧縮（+795 へ）で行数を捻出した。コミット時の hk→mise が rust 1.99.1 を取得しようとして失敗するため `HTTPS_PROXY=http://127.0.0.1:19191` が要る。
2. 曖昧だった文: 受け入れ条件の「止まっていることと理由」で、「理由」に内部エラーをどこまで含めてよいかが Issue に書かれていなかった。
3. 仮定と解釈: Claude レビューの must-fix に従い、外部へ投稿する理由は `LlmError` 変種ごとの固定文（`llm_reason`）とし、`Rejected` は status のみ投稿。HTTP 層が `{"code":"internal"}` だけ返す決まりと揃えた。
4. 足りなかった情報・検証手段: 実提供元（GLM）の 4xx/5xx 応答本文に実際に何が入るかは実接続が無いと確かめられない（レビュー側も未確認）。canary テストで「body が投稿に漏れない」ことだけは検証できる。
5. 改善案: `LlmError` に「外部へ見せてよい理由」を返す `public_reason` のようなメソッドを pelway-llm 側に持たせると、利用側ごとにマッピングを書かずに済む。
