# 2026-11-10 platform-pr-ci-sysdeps-r1

- 開始: 2026-11-10 17:56 JST
- ブランチ: `demo-t3/platform-pr-ci-sysdeps-r1`
- worktree: `/home/hatai/.t3/worktrees/Pelway/demo-t3-platform-pr-ci-sysdeps-r1`
- 親: `105a9ac7cfc9618285a003995b4d085c6616148a`（#63 の main マージ）

## 変更

- `.github/workflows/verify.yml` の verify / coverage 両ジョブへ、クライアントビルド用の `libfontconfig1-dev` 導入ステップを追加した。
- `docs/dev-environment.md` の CI 節を、上記 apt 導入と「`mise bootstrap`（podman）は使わない」方針が両立する書き方に更新した。
- 製品コードは変更していない。

## 入れなかったもの

- `docs/dev-environment.md` に Linux 向け apt として書いてあるのは `[bootstrap.packages]` の `apt:podman`（rootless 一式）のみ。CI 方針どおり podman は入れていない。
- 文書にクライアント用の他の `*-dev` パッケージ一覧は無かったため、追加していない。

## 検証

- このブランチ単体はクライアントクレートを含まないため、`yeslogic-fontconfig-sys` のビルド成功は未検証。
