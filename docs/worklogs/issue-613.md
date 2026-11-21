# Issue #613 の作業ログ

## 実装者メモ（round 1）

対象 head: bf1bb0d11659828ec9b70a37ff0b8f0c84c3e935

1. 詰まった箇所: `scripts/agent-gh` の push 分岐: `PELWAY_AGENT_SKIP_RESIGN=1` の push は汎用の経路に落ちるため、確認を `push && -z SKIP_RESIGN` の分岐の中に置くと素通りする。サブコマンドの判定（`git_sub`）を分岐の前に出し、両方の経路でフックを掛けた。
2. 曖昧だった文: 「確認を外す環境変数・引数が無い」（Issue #613 の受け入れ条件）。pre-push フック方式では `--no-verify` と `-c core.hooksPath=` が事実上「外す引数」になるため、この 2 つ（省略形と `--config-env` を含む）を拒否する形と読んだ。
3. 仮定と解釈: push される ref と sha は引数を解析せず、pre-push フックの stdin から受け取る。フックはリポジトリに置かず、実行時に一時ディレクトリへ作る（新規の実行ファイルは agent-gh-resign で署名できないため）。repo の設定に `alias.x = push` を置くような意図的な回避は対象外とした（Issue の目的は、素の `git commit` による事故の防止）。
4. 足りなかった情報・検証手段: resign 済みのコミットは committer が `GitHub` になる（`git log origin/main --format='%h %an | %cn | %G?'` で確認）。通常は追跡 ref から辿れるので範囲外になるが、追跡 ref が無い状態で再 push すると止まる。これを確かめる試験は無い。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「決まり」の commit と push の行に「`scripts/agent-gh dev git push` は pelway-dev[bot] 以外の名義のコミットを含む push を止める」と足す。

使ったスキル: implement-issue
