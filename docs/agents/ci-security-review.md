# CI セキュリティレビューの認可と隔離

GitHub Actions 上で動かすセキュリティレビューについて、起動の認可・実行の隔離・結果の扱いを定める。Buzz（block/buzz）の `codex-security-review` 方式を、Pelway のレビューア運用（`docs/superpowers/specs/agent-team.md`）に合わせて取り込んだ設計。レビューの中身（観点・判定）は skill `security-review` と別の役割で、本書は起動と隔離だけを扱う。

CI ワークフローそのものの実装は本書の範囲外で、workflow ファイルの追加・変更は人間が行う（`pelway-dev` に Workflows 権限を付けない制約。agent-team.md「制約」）。

## 起動の認可

認可の判定は、trusted な準備ジョブだけが行う。準備ジョブは `pull_request_target` のコンテキスト（base リポジトリ側・権限は `contents: read` と `pull-requests: read` のみ）で動き、PR のコードを実行も checkout もしない。

### 自動対象

非 Draft で base が main の PR のうち、作者が次のどちらかを満たすものを自動対象とする。

1. 作者の login が固定許可リストに一致する: `pelway-dev[bot]`、`pelway-review[bot]`、`hatai`
2. 作者がリポジトリに対して write 以上の実権限を持つ。`repos.getCollaboratorPermissionLevel` をその場で呼んで確認する（`author_association` は private な org メンバーを隠すため使わない。`maintain` は `write` と返る）

GitHub App の bot アカウントはコラボレータとして扱われず権限確認では `none` を返すため、許可リストと実権限確認を併用する。権限確認の呼び出しが失敗・404・不明な応答のときは自動対象にしない（fail-closed。#191「検査ゲートの意味論を fail-closed で固定する」と同じ方向）。

### 手動起動

自動対象でない PR は、write 以上の実権限を持つ利用者が、コメント本体が `@pelway-security-review <40 桁の head SHA>` と完全一致するコメントを投稿したときだけ起動できる。App・bot 名義のコメントは受理しない（作者系の名義による自己認可を防ぐ）。コマンドの SHA と現在の PR head が一致しなければ認可しない。認可したコミットだけを審査するため、認可のあと PR が動いた場合は新しい head SHA でやり直す。

### 起動イベントと対象の固定

- 起動イベント: `pull_request_target` の opened / reopened / ready_for_review / synchronize（非 Draft のみ）と、`issue_comment` の created（上のコマンド）
- head リポジトリが同一かフォークかは認可条件にしない。認可した PR の head SHA を `refs/pull/<N>/head` で固定して検査し、checkout 直後と検査開始前の 2 度、SHA の一致を確かめる

## 実行の隔離

- 対象コードは検査ジョブで読み取りだけに使う。リポジトリのスクリプト・ビルド・テスト・パッケージマネージャ・変更されたバイナリを実行しない
- checkout は `persist-credentials: false`。検査ジョブの権限は `contents: read` のみ。checkout 後に git の credential 設定（`http.*.extraheader` 等）が残っていないことを確かめ、残っていればそのジョブを失敗とする
- レビューア（Claude）は、対象の checkout とは別の trusted な作業ディレクトリから起動する。被審査リポジトリの `CLAUDE.md`・`AGENTS.md`・`.agents/skills/`・記憶（メモリ）を、レビューアの指示・スキルとして読み込ませない。対象コードは「審査の入力データ」としてだけ参照する（#193「被審査側の文脈を持ち込まない隔離原則」と同じ方向）
- レビューアのプロセスから runner の GitHub トークン・ランタイムトークン・OIDC 関連の環境変数を外す。レビューアにはネットワークを使わせない。レビューアの API キーは実行 action の背後に留め、サブプロセスの環境に入れない
- 秘密（レビューアの API キー）は GitHub Environment の保護下に置き、検査ジョブ以外に渡さない。秘密の名前と発行方法は実装 Issue で決める（ログイン等の用意は本設計の対象外）

## 結果の扱い

- 結果は助言であり、マージゲートにしない。この workflow のジョブを required status check に設定しない、ブランチ保護の必須チェックに加えない。`pelway-review` の Approve / Request changes の判定とは独立する
- 投稿は PR コメント 1 件とし、`github-actions[bot]` で投稿する。審査した範囲を示すマーカーコメント（`base...head`）を必ず含める
- コメント投稿は `pull-requests: write` を持つ別ジョブだけが行い、検査ジョブに書き込み権限は与えない。投稿前にレビューアの出力をスキーマ検証し、制御文字・任意 Markdown・HTML の混入を無害化してから、信頼側のコードで Markdown を組み立てる
- 読み手は指揮役と人間。所見を must-fix 相当の扱いへ昇格させるかは指揮役が判断し、通常の修正依頼の経路に流す

## stale 管理と再レビュー

- 各レビューコメントは審査した範囲のマーカーを残す。PR への新しい push（synchronize）があったら、以前のレビューコメントに stale 表示を足して「最新の head に対するレビューではない」ことを見えるようにする
- main push 時に過去のレビューを再整理する常時の reconciliation ジョブは設けない。base が動いた PR の再レビューは、次の synchronize か手動コマンドで起きる。必要になった場合は別 Issue で追加する

## 既存の運用との関係

- 本設計が扱うのは CI 上の助言レビューだけ。既存の `pelway-review` による PR レビュー（skill `review-pr` と、発動条件に合ったときの skill `security-review`）の判定・レビュー担当の sandbox（agent-team.md「レビュー担当の権限」、`docs/dev-environment.md`）は変えない
- #191（検査ゲートの fail-closed 意味論）・#193（審査系 AI の隔離原則）は起票済みで未確定のため、本設計は両 Issue 本文の方向と矛盾しない記述に留める。確定後に食い違いが出たら本書を直す
