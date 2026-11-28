# Issue #816 の作業ログ

## 実装者メモ（round 1）

対象 head: 244ff1e8d2d044b0721ed7c03c93ca66ab67bf53

1. 詰まった箇所: 手順 6 の `mise run verify`: この sandbox の `/tmp` は所有者が uid 65534 で、`.agents/skills/verify-pelway/verify-run.test.sh` が `make_run_dir` の所有者検査で落ちるため、`TMPDIR=/dev/shm/p816` で再実行した。
2. 曖昧だった文: 「行ごとの転写はせず、分類ごとの対象パス群または代表例と残す理由を書く」（Issue #816「受け入れ条件」）。分類表の粒度が行単位でないことを、この文から分類ごとの代表例でよいと読んだ。
3. 仮定と解釈: v0.7 body に残る Slint の比較本文は、冒頭の置き換え注記で過去の検討と分かる限り現行方針と矛盾しないとした。注記だけで過去と分かる形を選び、比較本文や構造を再構成する案は選ばなかった。
4. 足りなかった情報・検証手段: GUI を出せる環境が無いため、`mise run client:dev` の手動起動確認は未実施。文書は「verify-run の合否に入れない補助手順」として書いた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「検査してコミットし、行数を確かめてから push する」に「Linux sandbox で `/tmp` の所有者が uid 65534 のときは `TMPDIR=/dev/shm/<issue>` を使う」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 9aed4e57dbc61343c49aa7e5780b9c35830b17dc

1. 詰まった箇所: `docs/technical/v0.7/body/08-decisions-open.md`: A2 で元の決定文を残しつつ、置き換え後の共通核を注記で示す形を確認した。
2. 曖昧だった文: 「[技術仕様 v0.7](../technical/v0.7/README.md) の本文は消さず、この ADR で置き換えた箇所（08 の U05・A1・A2、02 の冒頭・「ネイティブUI」・クライアントの行・「Discordの事例」、07 の比較とメモリ目標）に注記を入れる。」（ADR 0014「v0.7 との関係」）
3. 仮定と解釈: Issue #816 はスキル改善ではなく Electron 決定への文書同期なので、`improve-skills` 手順 7 の引き渡し記録は付けない。改訂日と決定日の記述は AGENTS.md の「Facts over history」より Issue の受け入れ条件を優先する。
4. 足りなかった情報・検証手段: 範囲限定 grep の残存行の分類は、行ごとの表か分類ごとの代表例かが Issue 本文だけでは読み分けにくい。分類ごとに対象行を示す表を PR 本文へ足す。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「残存行の分類を求める条件では、行ごとか分類ごとかを指定する」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 3）

対象 head: b59cd141d9b37aa9a930d27ac940cb3dfe6777b8

1. 詰まった箇所: `docs/technical/v0.7/verification/verify.py`: 本文の注記をリンクなしの文にしたとき、見出し・アンカーは変わらなくても相対リンクの件数が変わり、生成物（relative-links.json・static-checks.json）の再生成が要ることがレビューで初めて分かった。
2. 曖昧だった文: 「v0.7 本文の見出し・明示アンカー・リンク構造が保たれている」（Issue #816 の受け入れ条件）
3. 仮定と解釈: 「リンク構造」を見出し間の導線の骨格と読み、注記内のリンク増減は構造の変更に当たらないと解釈していた。受け入れ条件は相対リンクの件数まで含めており、変わった場合は verify.py を実行して生成物を同じ PR で更新する。README 系の「Rust デスクトップ」の主文は受け入れ条件外の文書でプロダクト判断を含むため別 Issue に回し、`docs/agents/external-skills.md` の参照は #826 でそろえる。
4. 足りなかった情報・検証手段: v0.7 の body を変える PR で verify.py の再生成が要るかを、コミットの前に気づく検査が無い。nightly の docs-v07 は生成物を書き出すだけで、コミット済みの JSON と比べない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 6 に「`docs/technical/v0.7/body` を変える差分では `python3 docs/technical/v0.7/verification/verify.py` を実行し、生成物の差分を同じ PR に入れる」を足す。

使ったスキル: implement-issue、write-pull-request

## 実装者メモ（round 4）

対象 head: db252cb02fc330a019894dab54c62524deba666d

1. 詰まった箇所: `docs/technical/v0.7/body/09-sources.md`: wasm-bindgen は「不採用の調査対象」だが `dev-client-probe` の直接依存として残っており、台帳の列の意味（配布物に入るか）と字義（依存の有無）がずれていた。
2. 曖昧だった文: 「ブラウザ版を先行し、将来はRust製デスクトップアプリを想定します」（README など 7 ファイル）
3. 仮定と解釈: 公開・計画の現行文書は ADR 0014 の決定（D1 から Electron、ブラウザ版は時期を別に決める）に合わせて主文を直す。`docs/planning/input-ui.md`・`docs/planning/input/technical-input.md` は入力担当が出した記録で、後から決定に合わせて書き換えるものではないと解釈して残した。
4. 足りなかった情報・検証手段: 台帳の行と実際の依存（Cargo.toml）の突き合わせは人手で、依存を消すタイミングで台帳を直す検査は無い。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の受け入れ条件の書き方に「依存の台帳の記述は Cargo.toml と突き合わせる」を足す。

使ったスキル: implement-issue、write-pull-request
