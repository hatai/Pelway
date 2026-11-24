# Issue #689 の作業ログ

## 実装者メモ（round 1）

対象 head: 45da7c8326433674e8aa717e6a60c329e056a8b5

1. 詰まった箇所: 調査自体: `/dev/kvm` は実装担当の sandbox（bwrap）の内側から見えない（sandbox の `/dev` は最小構成で、`/dev/kvm` を含まない）。「`/dev/kvm` が無い＝ホストに無い」と書き違えないよう、`lsmod` の `kvm_amd`（モジュール一覧は名前空間で分かれない）と Issue 本文のホスト側記録（18:57 JST）を併記する形にした。
   - `mise run verify`: `.agents/skills/verify-pelway/verify-run.test.sh` が 6 件失敗した。差分（文書 2 件の追加のみ）を stash しても再現し、CI の main の `mise run verify` は通る（2026-11-24T10:01Z の check_runs）ため環境起因。切り分けると、sandbox 内で `/tmp` の所有者が `nobody`（uid 65534、`stat -c '%u %a' /tmp` → `65534 1777`）に見えるため `dir_chain_safe` の「所有者が自分か root」の条件で落ちる。フォロー Issue #692 を起票した。
2. 曖昧だった文: 「6 つの観点のそれぞれについて、bwrap 方式と `sbx` の両方の欄があり、`sbx` の欄には出典（公式文書の URL）か「minisforum で確認（コマンドと日時 JST）」が付いている」（Issue #689 の受け入れ条件）。bwrap 側の欄の出どころの書き方は指定が無かったため、リポジトリ内のパス（`scripts/agent-sandbox/sandbox-shell`・`docs/dev-environment.md`）を出どころとして書いた。
3. 仮定と解釈: 「T3 の worktree スレッド・opencode・Devin CLI の起動の仕方に乗るか」は、「T3 が ACP でエージェントを起動する構成のまま `sbx` の内側にエージェントを入れられるか」と解釈した。「`sbx run <agent>` で T3 を介さず直接エージェントを動かす運用への置き換え」は対象外と読み、候補にも挙げなかった（Issue の「対象外」に T3 の設定変更があり、運用の置き換えはこの Issue の調査範囲を超えるため）。`sbx` の Windows 版経路（`\\wsl.localhost` のワークスペース）は、sbx-releases #598・#900 の障害報告から候補にしなかった。
4. 足りなかった情報・検証手段: 公式文書に FAQ・credentials・usage の URL が 2 系統（`/ai/sandboxes/faq/` と `/ai/sandboxes/configuration/faq/` 等）に見え、初回の取得は 404 になった。正しい URL は一覧ページから辿って確かめた。git worktree をマウントしたとき共有 git dir まで git が動くか、追加 workspace のマウントをネストできるかは、文書に記載が無く未確認のまま（文書に「未確認」と記入）。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「比較表を求める条件では、各欄の出どころの書き方（URL・コマンドと日時・リポジトリ内のパスのどれを取るか）を条件に書く」を足す案。

使ったスキル: `implement-issue`、`ja-text-communication`

## 受け入れ条件「導入・設定変更・サインインを何もしていない」の確認

2026-11-24 19:07 JST、実装スレッド（agent-sandbox 内）で実行。

```text
$ command -v sbx
（出力なし）
exit=1
$ id -nG
hatai nogroup
exit=0
```

- `command -v sbx` が何も返さず、`id -nG` に `kvm` が無い。導入・グループ変更・サインインは行っていない。
- 補足: 実装担当の sandbox の内側のため、`/dev/kvm` はこの名前空間では見えない（ホスト側の記録では `root:kvm`・`crw-rw----` で存在。Issue #689 本文、2026-11-24 18:57 JST）。
