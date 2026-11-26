# Issue #737 の作業ログ

## 実装者メモ（round 1）

対象 head: 20c3296acf292bab2fafc7772f5a969b98e1f47b

1. 曖昧だった文: 「空・相対パスなど不正な値が渡されたときの挙動」は受け入れ条件が「既定に戻す」か「送らない（reason=no-secret）」かの選択を委ねていた。後者を採った（`notify_fnox_config()` が `None` を返し、`send` は秘密が無いときと同じ経路で `mode=not-sent reason=no-secret` になる）。前者は、typo のパスに値を載せたまま送信が続く気づきにくさがあり、退避させる方が安全と判断した。選択は `io.rs` の doc コメントと `docs/dev-environment.md` の両方に書いた。
2. 仮定と解釈: 「絶対パス」の判定は `Path::is_absolute` で行い、値が空文字列・相対パスのときは設定として使わない（既定にも戻さない）。未設定と「セットしたが不正」は区別される（未設定 → 既定、不正 → 送らない）。エージェントが書き換えられる環境変数で相対パスを許すと、起動元の cwd によって指す場所が変わるため。
3. 仮定と解釈: 「通知専用の age 鍵に分ける」は、`review_age`（#449）と同じ形に揃えた。provider 名を `notify_age` とし、`key_file` を明示して `~/.config/fnox` 無しでも復号できる形にする（グローバル設定の `[providers.age]` とのマージで鍵の指定が曖昧にならないため）。dev 用の fnox 設定への通知の公開鍵の写しはやめ、`agent-accounts.sh` は dev の設定には dev 自身の `~/.config/fnox/age.txt` の鍵だけを載せる形にした。
4. 詰まった箇所: `agent-accounts.sh` で、すでに `[providers.age]` 形式の通知設定がある環境を再実行したときの扱い。ウィザードが無断で鍵の形式を書き替えると値が取り出せなくなるので、書き替えず警告だけ出して `docs/dev-environment.md` の移し替え手順に誘導する形にした（同じステージで `providers.notify_age` の有無を見て登録の可否を決める）。
5. 足りなかった情報・検証手段: `fnox set` で `if_missing = "ignore"` のような既存の secret 定義の属性を保ったまま再登録する手段は確認できなかった。`fnox set` は `{ provider, value }` の形で書くため、移し替えの手順でもその形になる。`agent-watch` は `fnox get` で直接読むので `if_missing` の有無は送信の動作に影響しないが、`fnox exec` 経由で読む運用があれば挙動が変わる。
6. 改善案: `notify_fnox_config()` は送信のたびに環境変数を読み直す実装にした。無効値のときに起動時に 1 回だけ警告を出す手もあるが、起動中に直された場合に気づけない。読み直す形なら環境変数の修正が次の送信でそのまま効く。
7. 詰まった箇所: 手順 6: push 後の必須チェックで `scripts/tests/agent-team-config.test.sh` の試験 3（instance names and models match current ops）が落ちた。原因はこの PR の変更ではなく、main の 7c36a927（Update agent-team.toml）が `agent-team.toml` のモデル一覧を変えたのに試験内の pin を更新しなかったため（main 自身の verify も同じ理由で落ちている。PR の CI は merge ref を検査するためこの PR でも落ちる）。main を取り込んだうえで pin を登録済みの新しい値（`zai-coding-plan/glm-5.4`・`swe-2-1-high`・`claude-opus-5-5`/`claude-sonnet-5-5`/`claude-haiku-5-5`）に追従させ、main の赤の直しもこの PR に含めた。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: 5a07965f518587625af02bcd79da37789738a740（マージコミットを含む push のため re-sign 対象外・署名なし）

レビュー（claude/opus/high、run 7188751463、verdict=request-changes、must-fix 2）への対応:

1. must-fix 1（`~/.config/pelway-agent-notify` がどの sandbox の拒否リストにも無い）: 復号鍵と暗号文が同じディレクトリにあり `key_file` が絶対パスのため、読める sandbox から `fnox -c ... get` で復号できてしまう指摘は正しい（review_age 型をなぞったが、それと一対の拒否リストへの追加をなぞっていなかった）。リポジトリ側の 4 箇所（`.devin/config.json`、`scripts/agent-sandbox/devin-config.json`、opencode-plugin の `READ_DENY`、sandbox-shell の `masks`）に `~/.config/pelway-agent-notify` を足した。`masks` への追加は、`.config/fnox`・`.config/pelway-agent` が入っていない点とのずれに見えるが、あちらは sandbox の中の `agent-gh dev` が認証情報の復号に必要で隠せないのに対し、notify のディレクトリを読む必要のあるものはホスト側（agent-watch・`conductor-watch.sh`）だけ（`conductor-thread-id` の読み手を grep で確かめた。`patrol-state.json` は共有 git dir、outbox は `~/.local/state`）なので、ファイルシステム層で隠しても何も壊れない。リポジトリの外（レビュー担当 3 つと QA 担当）は、移し替え手順の手順 0 として「鍵を作る前に拒否を入れる」順序で docs に書いた。
2. must-fix 2（`fnox.toml.bak` を消す段が無い）: 指摘どおりで、残したままだと dev の鍵宛ての暗号文が残り鍵分割が成り立たない。「確かめ方」のあとに .bak を消す段を足し、「戻し方」は .bak を消す前に限ると明記した。
3. nit（`PELWAY_NOTIFY_FNOX_CONFIG` の空文字・相対パス、鍵 dir の `mkdir -p`）: agent-watch と同じく絶対パスのみ受け付ける形に揃え、それ以外は警告して既定の場所に作る。`mkdir -p` は設定ファイルと age 鍵の両方の親に効かせた（REVIEW 側も同じ形だったので同じ直し）。
4. nit（移し替えループの pipefail 不在）: `get` の成否を先に確かめる形に直した（`pipefail` は対話シェルの状態を変えるので、値を一旦変数に受けて失敗時に止める形）。`set` 側の失敗は `set -e` の無い対話シェルでは検知できないままだが、`fnox set` 自体の成否は各イテレーションで目に見える形（終了コード）で残る。
5. 確かめたこと: `masks` に `pelway-agent-notify` を足しても壊れるものが無いことは上記の読み手の確認で裏付けた。`.devin/config.json` など JSON の deny の書式は既存行に揃えた。シェルcheck・cargo test・JSON の構文は下の検証で通す。
6. 詰まった箇所（2 回目の main 衝突）: main の d1c177fa（PR #830）が同じ試験 pin を更新したが、claude のモデル一覧が `"claude-opus-5-5". "claude-sonnet-5-5"` とピリオドで TOML・Python ともに構文が壊れており、main の verify は再び赤。取り込みのコンフリクト解消でカンマのこちら側を採用したため、この PR のマージで main の赤も直る（マージコミット 6bff0875。なおその後 main でも ac5c9fce で同じ直しが入った）。

## 実装者メモ（round 3）

対象 head: 122967dec7384b59fee5bc42666425df4ea31c48（マージコミットを含む push のため re-sign 対象外・署名なし）

レビュー（claude/opus/high、run 7189553436、verdict=request-changes、must-fix 1、文書のずれ nit 2、out-of-scope 1）への対応:

1. must-fix（手順 0 の「実装担当側はリポジトリの変更で済んでいる」が正しくない）: `scripts/agent-sandbox/` の 3 つは `scripts/setup/agent-sandbox.sh` が `install` でホストに写した版（`~/.config/opencode/plugins/pelway-sandbox/{sandbox-shell,index.js}`・`~/.config/devin-impl/config.json`）が使われるため、写し直すまで新しい拒否は効かない。指摘は正しい（devin-impl の `--config` は `~/.config/devin-impl/config.json` を指す）。手順 0 の先頭に「本体 checkout でこの PR を含む main を取り込み、`bash scripts/setup/agent-sandbox.sh` を流し直す」段を足し、鍵を作る前に済ませる順序にした。233 行目も「写し直して初めて効く（`.devin/config.json` だけは worktree の版が読まれる）」に修正。
2. nit[文書のずれ]（「QA 担当の拒否リスト」の行）: QA は判定の投稿に `agent-gh qa` を、読み取りには `dev` 名義を使い、委託は実装担当のインスタンスで動く（command-team の「読み取りの `<role>` は委託先の名義で…それ以外（QA を含む）は `dev`」）ため、実装担当側の拒否で済む。行を消し、その旨をコメントに書いた。
3. nit[文書のずれ]（拒否リストの正本の節に 6 つ目が無い）: 列挙をすべて 6 パスに追従した（`deny_read` の例・「秘密パス 5 件」→ 6 件・claude_review の `Read(...)` 列挙・devin-review の列挙・#449 の節の対象パスの説明）。#449 の節のコードブロックはその手順当時の追加分として残し、6 つ目の書き方と手順 0 への参照を節の末尾に足した。
4. out-of-scope（指揮役の T3 スレッドの sandbox が未対象）: レビュー担当が「この PR 以前からの隙間」と切り分けた。指揮役は旧設定でも dev の鍵で復号できたため今回の分割で悪化していない。対応はこの PR の対象外としてメモだけ残す。

## 実装者メモ（round 4）

対象 head: TBD（round 4 のコード commit の SHA を記録する）

レビュー（claude/opus/high、run 7189912073、verdict=request-changes、must-fix 1）への対応:

1. must-fix（写し直しはディスク上のファイルを変えるだけで、動いているプロセスには効かない）: opencode の `READ_DENY` は plugin の import 時の定数、devin-impl の `--config` も devin_review・codex_review の設定も起動時に読むため、T3 を動かしたまま手順 0 → 手順 1 に進むと、動いているエージェントは古い拒否のまま動く（導入手順自体も T3 の再起動を求めている）。指摘は正しい。手順 0 の末尾に「T3 を再起動し、各担当の新しいスレッドで `ls ~/.config/pelway-agent-notify` で中身が見えないことを確かめてから手順 1 に進む」段を足した（`test -r` は access(2) を見るだけで denyRead 済みでも 0 を返しうるため使わない、とも明記）。233 行目にも「動いているエージェントのプロセスを起動し直して（T3 の再起動で）初めて効く」を足した。
2. 確かめたこと: opencode-plugin/index.js の `READ_DENY` はモジュール定数で `setup` の readDeny 化は読み直しをしない（reviewer の指摘どおり）。sandbox-shell の `masks` はコマンドごとの exec なので写し直しで即効くが、ファイルツール経路は再起動が要る、というレビューの整理とも一致する。
