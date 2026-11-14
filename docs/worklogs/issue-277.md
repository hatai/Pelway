# Issue #277 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: `devin acp` に `--sandbox` を渡す手段。「T3 の acpRegistry インスタンスに `--sandbox` を渡す方法（`launchArgs` 相当の対応）」（Issue #277「未決事項」）は、acpRegistry の `config` が `launchArgs` を持たないため `commandPath` 起動ラッパーで解いた。さらに `devin --sandbox acp`（サブコマンド前のフラグ）は受理されるが ACP セッションの sandbox を起動せず、`devin acp` は `--sandbox` を拒否する。ACP サーバが読むのは `DEVIN_SANDBOX` 環境変数（`devin` バイナリの文字列「ACP: sandbox initialization failed: DEVIN_SANDBOX」で確認）で、値は `true`/`false` のみ（`1` は起動拒否）だった。
2. 曖昧だった文: 「`bwrap` の導入（apt・sudo）は人間の作業。タイミングは hatai が決める」（Issue #277「未決事項」）。実際は linuxbrew の `/home/linuxbrew/.linuxbrew/bin/bwrap`（0.13.1、`--unshare-net` まで動作確認）が既にあり、sudo は要らなかった。
3. 仮定と解釈: レビュー用の設定は `devin --config` でユーザー設定ごと差し替える方式を選んだ。`sandbox` セクションと `permissions` はユーザー設定専用のため、メインの `~/.config/devin/config.json` にレビュー用 deny を混ぜると実装スレッドの `acpRegistry_devin` まで制限される（例: `edit` deny で編集不能になる）。`/home/hatai/.config/devin-review/config.json` に `devin.org_id` を写さないと組織解決が崩れる恐れがあるため複写した。
4. 足りなかった情報・検証手段: `Read(...)` deny に symlink（この環境では `~/.aws` → `/mnt/c/...`）を含めると bwrap がリンク先にマウントできず exec が全滅した（`bwrap: Can't mount on symlink destination`）。deny は実体パスのみに絞った。この挙動は公式ドキュメントに無い。
5. 改善案: `docs/dev-environment.md` の「レビュー担当の Devin CLI」に「`Read(...)` deny に symlink を含めると exec が全滅する」旨の注意を追記済み。
