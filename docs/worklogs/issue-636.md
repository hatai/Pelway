# Issue #636 の作業ログ

## 失敗の筋（実装前）

- 失敗の筋: `write-issue` にマイルストーン紐づけの決まりが無く、起票者がマイルストーンを付けずに起票する
  確かめ方: `.agents/skills/write-issue/SKILL.md` の「決まり（毎回守る）」と起票手順に、紐づけ・選び方・作成・読み返しの文言があることを確認する
- 失敗の筋: 適切なマイルストーンが無いときの作成条件・命名・失敗時の扱いが無く、起票者が近いマイルストーンへ仮に付けたり空のまま進めたりする
  確かめ方: `.agents/skills/write-issue/SKILL.md` に、作成条件・初期値・作成順序・通知・作成失敗時の `needs-human` の文言があることを確認する
- 失敗の筋: CLI / REST API の起票例にマイルストーン指定が無く、特に非 ASCII タイトルの API 起票でマイルストーンが付かない
  確かめ方: `.agents/skills/write-issue/SKILL.md` の起票例に `--milestone` と REST API の `milestone` 番号があることを確認する
- 失敗の筋: Issue Form の利用者がフォームではマイルストーンを設定できないことを知らず、作成直後の手動設定をしない
  確かめ方: `.github/ISSUE_TEMPLATE/agent-task.yml` の上部の案内に作成直後の設定と、適切なものが無いときの作成手順への言及があることを確認する
- 失敗の筋: `docs/agents/issue-tracker.md` に常設ルールが無く、手順書を読む担当がマイルストーン紐づけと作成の正本に辿り着けない
  確かめ方: `docs/agents/issue-tracker.md` に常設ルールと `write-issue` を正本とする文言があることを確認する
- 失敗の筋: 実装役・レビュー役が自分の skill だけを読み、Issue 起票時に `write-issue` を開かない
  確かめ方: `.agents/skills/implement-issue/SKILL.md` と `.agents/skills/review-pr/SKILL.md` に `write-issue` に従う文言があることを確認する

## 失敗の筋（最終）

- 失敗の筋: `write-issue` の決まりだけを読んだ起票者が、既存マイルストーンの選択と新規作成のどちらを行うかを取り違える
  確かめ方: `.agents/skills/write-issue/SKILL.md` の「決まり（毎回守る）」と「マイルストーンに紐づける」に、選択順・作成条件・作成順序が併記されていることを確認する
- 失敗の筋: 日本語のマイルストーン名を CLI 引数で渡そうとして agent-gh に拒否され、起票が進まない
  確かめ方: `.agents/skills/write-issue/SKILL.md` に非 ASCII の制約と REST API の `milestone` 番号による起票例があることを確認する
- 失敗の筋: 新規マイルストーン作成後に Issue への設定や読み返しを省略し、マイルストーン無しの Issue が残る
  確かめ方: `.agents/skills/write-issue/SKILL.md` の「マイルストーンに紐づける」の手順 3〜4 に PATCH と読み返しがあることを確認する
- 失敗の筋: Issue Form の利用者がマイルストーンを自動設定できると思い込み、作成直後の手動設定をしない
  確かめ方: `.github/ISSUE_TEMPLATE/agent-task.yml` の上部の案内に、自動設定されないことと作成直後の設定が書かれていることを確認する
- 失敗の筋: `docs/agents/issue-tracker.md` だけを読んだ担当が常設ルールと正本の場所に辿り着けない
  確かめ方: `docs/agents/issue-tracker.md` の `Milestones` に常設ルールと `write-issue` への参照があることを確認する
- 失敗の筋: 実装役・レビュー役が Issue 起票時に `write-issue` を開かず、マイルストーンの手順を適用しない
  確かめ方: `.agents/skills/implement-issue/SKILL.md` と `.agents/skills/review-pr/SKILL.md` の「決まり（毎回守る）」に `write-issue` に従う文言があることを確認する

## 実装者メモ（round 1）

対象 head: 81ba9802914a494d2d33637be59c726fa7d6adb1

1. 詰まった箇所: `mise run verify` は最初、この sandbox の `/tmp`・`/home` が uid 65534 所有で `.agents/skills/verify-pelway/verify-run.test.sh` の `dir_chain_safe` に落ちた。`docs/worklogs/issue-674.md` の前例どおり、自分所有のチェーンになる `TMPDIR=/dev/shm/p636` で再実行した
2. 曖昧だった文: 受け入れ条件は CLI の `--milestone` と REST API の `milestone` 番号の両方を求めるが、このリポジトリのマイルストーン名は日本語で、agent-gh は非 ASCII を含む argv を拒否する。この 2 つをどう両立させるかは本文に書かれていなかった
3. 仮定と解釈: CLI の `--milestone` は ASCII の題名を持つマイルストーンに限定し、このリポジトリでは通常 REST API の `milestone` 番号で起票すると解釈した。あわせて、起票コマンドが `issue create` に限定されないよう、`write-issue` 冒頭の既存の一文を `scripts/agent-gh dev` 経由と直した
4. 足りなかった情報・検証手段: `pelway-dev` にマイルストーン作成権限があるかは、不要なマイルストーンを実際に作る確認になるため検証していない。作成失敗時は近いマイルストーンへ仮で付けず、`needs-human` で止まる手順を書いた
5. 改善案: マイルストーン作成から Issue への PATCH と読み返しまでを行う検査済みの helper を `write-issue` に同梱すると、JSON を起票者ごとに組み立てる誤りを減らせる

使ったスキル: implement-issue、writing-for-agents、write-pull-request
