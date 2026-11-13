# 2026-11-13 platform-pr-ci-octocov-datastores-r2

- ブランチ: demo-t3/platform-pr-ci-octocov-datastores-r2（origin/main 322b7cf8d7d3c374984dcd337c6bd02355917d11 から新規作成）
- worktree: /home/hatai/.t3/worktrees/Pelway/demo-t3-platform-write--5d2b13fe1a3cd733
- 依頼: 濱田さん（grilling で合意、2026-11-13）。octocov のカバレッジ増減表示を datastores 形式で実現する。Issue #266（write-pull-request の Draft ライフサイクル論点）は別件として同日先に起票済み。

## 背景（r1 からの引き継ぎ）

- 同じ設計の r1（ブランチ demo-t3/platform-pr-ci-octocov-datastores-r1 系、先端 d8d57c4）は未 push のまま main が前進し（verify.yml の並列ジョブ化・mise-action v5・coverage 除外等）、再利用より作り直しが速いと判断された。
- 設計自体は r1 で Codex 読み取り専用レビュー4回分の吟味を経たもの（当時の経緯は未 push のため参照できないブランチ側の作業ログにのみ残る）。

## 変更内容

- `.octocov.yml`: `diff.storage.artifacts`（`if-no-artifact: pass`）を削除し、公式 v0.83.5 の `report.datastores` / `diff.datastores`（ともに `artifact://${GITHUB_REPOSITORY}`）に置換。`coverage:`・`comment:` は変更なし。
- `.github/workflows/verify.yml`:
  - `on:` に `push: branches: [main]` を追加。
  - `coverage` ジョブの `permissions:` に `actions: read` を追加（既存の `contents: read`・`pull-requests: write` はそのまま）。
  - `Run octocov` step 直前のコメントと workflow 先頭コメントの trigger 記載を現状に合わせて修正。
- `docs/dev-environment.md`: CI 節の3文を更新（main push でも workflow が走る旨、権限がジョブ単位である旨、`report.datastores` / `diff.datastores` と増減が出始める条件）。
- 本作業ログ。

## 根拠（octocov README v0.83.5。写し: /mnt/c/Users/htaig/Downloads/pelway-review/octocov-README-v0.83.5.md、読み取りのみで再確認）

- L256: デフォルトブランチのレポートは「リポジトリ自体の下」に保存され、PR のレポートとして保存されるのは pull request イベントの run のみ。つまり main のレポートは main での run でしか保存されない → `push: branches: [main]` が必要。
- L264: `diff.datastores:` は PR で base ブランチのレポートを読む。base ブランチのレポートがどの datastore にも無い場合はデフォルトブランチのレポートを読む。
- L189-193: 計測と PR への書き込みは best-effort（`comment:` 等の失敗は run を落とさない）。一方「storing the report to `report.path:` or `report.datastores:`」は失敗すると run を落とす（次の run が比較のために読む物を書くため）。
- L1121-1142（GitHub Actions Artifacts 節）: scheme は `artifact://[owner]/[repo]/[artifactName]`、artifactName 省略時の既定は `octocov-report`。必須環境変数は `GITHUB_TOKEN` と `GITHUB_REPOSITORY`。必要権限の明記はこの節に無いが、README の workflow 例は `actions: read` を付与する → `actions: read` を追加し、保存側の可否は最初の main run のログで確認する。

## 検証（実行したコマンドと出力）

- `python3 -c "import yaml; yaml.safe_load(open('.octocov.yml')); yaml.safe_load(open('.github/workflows/verify.yml'))"` → `YAML OK`。
- `mise exec -- jactionlint .github/workflows/verify.yml` → exit 0。`note: shellcheck was not found, so it did not check 24 shell scripts.` の表示あり（shellcheck 未導入のため `run:` の shellcheck 検査はスキップ。jactionlint 自体の指摘はなし）。
- `git diff --stat` → 3ファイル +25/−10（pr-size 上限 +800/−2000 の範囲内）。

## 未検証

- GitHub 上での実機: main の run によるレポート artifact の保存、`actions: read` で保存・読み出しが通ること、保存後の PR コメントでの増減表示、`updatePrevious` の更新動作。いずれも最初の main run のログと次の PR で確認するまで未検証。
- 保存（`report.datastores`）が 403 等で失敗した場合は run が落ちる（README L193）。その場合は `actions: write` の付与を検討する。

## コミット

- `ci: store octocov reports via datastores and run verify on main`（.octocov.yml・.github/workflows/verify.yml・docs/dev-environment.md・本作業ログ）。

## 修正1: coverage ジョブの octocov 保存失敗（PR #268 の CI、5d879e6）

### 失敗の内容

- PR #268 の最初の CI で `octocov (coverage diff)` のみ失敗（ほかの必須チェックは合格）。ログ: `Storing report...` のあと `Error: env ACTIONS_RUNTIME_TOKEN is only available from the context of an action`（exit 1）。
- 原因: artifact へのレポート保存には ACTIONS_RUNTIME_TOKEN が必要だが、これは `run:` ステップには渡されず action（`uses:`）のコンテキストでしか取れない。octocov リポジトリ自身の CI にも同じ趣旨のコメントがある（`.github/workflows/ci.yml`「Uploading the page needs ACTIONS_RUNTIME_TOKEN, which a run step is not given」）。README v0.83.5 の Artifacts の節（L1121-1142）にこの記載は無い。

### 直し（公式の octocov-action 経由に変更）

- `.github/workflows/verify.yml`: `Run octocov` を `run: mise exec -- octocov` から `uses: k1LoW/octocov-action@v1.5.2`（`version: 0.83.5` + `checksum` 付き）に変更。action は内部で gh-setup が octocov 本体を導入し actions/github-script（action コンテキスト＝ランタイムトークンあり）から実行する。
- チェックサムの根拠: v0.83.5 の linux_amd64 アセットは `octocov_v0.83.5_linux_amd64.tar.gz`（asset id 711698412）1つだけ（zip は darwin/windows のみ。deb/rpm/apk は gh-setup の content-type 対応外で除外される）。mise.lock の linux-x64 エントリと同一アセットのため、同じ sha256（`3aa6973a…555d8e9c`）を流用。gh-setup の checksum は `sha256:<hex>` 形式に対応、`application/x-gtar` は対応 content-type に含まれる（いずれも k1LoW/gh-setup の gh/gh.go で確認）。
- 同ワークフローの coverage ジョブ権限: `actions: read` → `actions: write`。octocov-action の公式例が「過去レポートの取得と default ブランチの現行レポート artifact の削除」に `actions: write` を付与するため。
- `.octocov.yml`: `report:` に `if: is_default_branch` を追加（octocov-action の公式例と同じ形。diff が読むのは base ブランチのレポートのため、PR の run は保存しない）。
- `.github/workflows/actions.lock`: `scripts/agent-gh dev actions-lock` で再生成（octocov-action@v1.5.2 と、その複合アクション内の依存 k1Low/gh-setup@v1.11.9・actions/github-script@v9.0.0 が sha 固定で追加）。
- `docs/dev-environment.md`: CI 節の2文を上記に合わせて更新（actions: write、octocov-action 経由の理由、version+checksum 固定）。

### 検証（修正1）

- `python3 -c "import yaml; …"` → `YAML OK`。
- `mise exec -- jactionlint .github/workflows/verify.yml` → exit 0（shellcheck 未導入の note のみ）。
- `scripts/agent-gh dev actions-lock` → 上記の追記で完了（エラーなし）。
- 実機（PR の再 run で `Run octocov` が action 経由で成功すること）は未検証。push は workflow ファイル変更のため担当が行う。

### コミット（修正1）

- `ci: run octocov via official action for artifact datastore`（.octocov.yml・.github/workflows/verify.yml・.github/workflows/actions.lock・docs/dev-environment.md・本作業ログ）。
