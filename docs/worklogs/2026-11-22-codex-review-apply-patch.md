# レビュー担当の Codex の apply_patch の書き込み範囲（2026-11-22）

`docs/dev-environment.md` の「専用 home での確認」の `~/.codex-review` への書き込みの項目を、`apply_patch` で確かめようとした記録。版は `codex-cli 0.171.0`、T3 `0.1.10-nightly.20261115.4736`。

`main` のコミット（`383ab2ca`）から委託した `codex_review` の確認用の子タスク（`runtimeMode: "auto-accept-edits"`、gpt-6.2-sol）に、`apply_patch` で `/home/hatai/.codex-review/probe-apply-patch.txt` と `/home/hatai/probe-apply-patch-outside.txt` を作らせようとした。

- 子タスクは「書き込みが許可された範囲の外で、`apply_patch` には承認を求める機能がない」として、どちらも試さなかった。ツールからの拒否も承認要求も観測していない。
- ホストで確かめると、どちらのファイルもできていない。
- 結果: 未確認。`apply_patch` が範囲の外への書き込みを sandbox で止めるかは、まだ実測できていない。
