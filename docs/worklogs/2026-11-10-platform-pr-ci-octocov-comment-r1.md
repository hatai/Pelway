# 2026-11-10 platform-pr-ci-octocov-comment-r1

- 開始: 18:29 JST（Asia/Tokyo）
- ブランチ: demo-t3/platform-pr-ci-octocov-comment-r1
- worktree: /home/hatai/.t3/worktrees/Pelway/demo-t3-platform-pr-ci-octocov-comment-r1
- 親: d67fc7c6798b48ac04209f04c0274056614c8b31（現 main。#60 マージ後）
- 依頼: 濱田さん（エンジニアチーム部屋）。octocov のカバレッジ変化を PR コメントまたは本文へ出す機能を使っているか、使っていなければ有効化してほしい。開発リードからも、fontconfig 修正とは別ブランチで進めること、実際の PR コメント有無を先に確認することを指示された。

## 調査

- coverage ジョブ（octocov (coverage diff)）自体は動いている。ジョブ権限の pull-requests: write と GITHUB_TOKEN も既にある。
- 一方 .octocov.yml には公式の comment: も body: も summary: も無かった。
- GitHub の PR #63、#61、#60 の issue コメントと review コメントはいずれも 0 件だった（カバレッジ報告なし）。
- 公式 v0.83.5 README では、PR コメントは comment:、Job Summary は summary:、PR 本文への挿入は body:。

## 変更

- .octocov.yml に comment.if: is_pull_request と comment.updatePrevious: true を追加した。
- body: は追加していない（PR 本文を書き換えない）。
- report: と公式の diff.datastores: は、今回は触っていない。既存の diff.storage.artifacts はそのまま。履歴との増減表示は、base（main）側に公式形式のレポートが貯まるまでは出ない可能性がある。PR コメント自体と、変更行の patch coverage 列は comment: だけで出る。
- 製品コードは変更していない。workflow の権限も変更していない。fontconfig 修正ブランチ（demo-t3/platform-pr-ci-sysdeps-r1）には混ぜていない。
- push していない。

## 未検証

- GitHub 上で実際に PR コメントが付くこと。
- updatePrevious: true の更新動作。
- 履歴レポートとの増減表示。
