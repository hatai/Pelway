# Issue #520 の作業ログ

## 実装者メモ（round 1）

対象 head: 5bf99db055311d64678b15f43ba433f9c8ab0546

1. 詰まった箇所: scripts/agent-gh: `if ! response="$(gh token generate ...)"` の形では `!` 反転後の `$?` が常に 0 になり gh の終了コードを拾えなかったため、`response="$(...)" || rc=$?` の形に変えた。もう 1 件: 初回 `mise run verify` が pelway_http のテストバイナリ欠落（No such file or directory）で失敗したが、単独実行と再実行で通った（ビルド並列時の環境起因。PR 本文に内訳を記録）
2. 曖昧だった文: 「『拡張が必要』という文は、`gh extension list` に gh-token が無いときだけ出す」（Issue #520 受け入れ条件）— `gh extension list` 自体が失敗したときの扱いが書かれていない
3. 仮定と解釈: extension list の失敗時は「gh-token が無い」とは判定できない。判定不能なら案内を出さない（拡張があるのに誤案内する本 Issue の発端が再発するため）と解釈した。採らなかった解釈: list 失敗も「無い」とみなして案内を出す
4. 足りなかった情報・検証手段: Link-/gh-token 拡張が失敗時に出す stderr の実際の書式（Issue コメントにあるのは gh 本体の permission denied の実測のみ）。テストは合成文字列で書いた
5. 改善案: .agents/skills/implement-issue/SKILL.md の「よくある失敗」に「『値が洩れないこと』を確かめる条件は実装前 red にならない性質がある（出す実装と同じ round で初めて意味を持つ）」を足す

使ったスキル: implement-issue、.agents/skills/diagnosing-bugs/SKILL.md（verify 失敗の切り分けに Phase 2 の再現と最小化）

## 実装者メモ（round 2）

対象 head: 53ab69e93a4076c072771eab935377e204a7ed4c

1. 詰まった箇所: scripts/agent-gh: main（PR #524）と #520 が同じ `app_token` を変えていた。main は key_file を local 宣言から外した（subshell の EXIT trap から見えるように）が、私は err_file を local に置いたままだった
2. 曖昧だった文: 「トークン発行そのものが失敗しても、本体の gh は呼ばず exit 3」（scripts/tests/agent-gh.test.sh の main 側テスト no_fallback_generate_fails のコメント）— #520 で追加した診断用の `gh extension list` 呼び出しが「gh を呼ぶ」に当たるか書かれていない
3. 仮定と解釈: 「本体の gh」とは元のコマンド（`gh pr list` 等）の実行のことと解釈した。`gh extension list` は #524 の隔離された空の GH_CONFIG_DIR の中だけで動き hosts.yml を読まないため fail-closed の意図（人間のトークンで続けない）と矛盾せず、診断として残した。merge 後に #520・#522 両側のテストが全て通ることを実測で確認した
4. 足りなかった情報・検証手段: merge コミットが署名付け替え（agent-gh-resign）できないことを示す表示の意味。AGENTS.md の「API で表現できない変更（…マージ…）を含む push は署名なしのままになる」で既定の挙動と確認した
5. 改善案: .agents/skills/implement-issue/SKILL.md の「手順 9 修正依頼」に「merge 後は main 側が追加した同じファイルのテストとの相互作用も通過確認する」を足す

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 356b38aee8d48fa85ad69e691eb186db22df0c04

1. 詰まった箇所: なし。レビュー指摘の `grep -q 'gh-token'` の部分一致（scripts/agent-gh）は、修正前にテスト `token_fail_install_hint_for_lookalike_name` を足して red（案内が出ない）で再現してから直した
2. 曖昧だった文: 「extension 名を行単位で正確に（完全一致で）判定し」（指揮役の must-fix 通知）— 「行単位」の判定対象が行全体か名前フィールドか書かれていない
3. 仮定と解釈: `gh extension list` の出力は「名前 バージョン リポジトリ」で名前は必ず第 1 フィールドと仮定し、`awk '{print $1}'` で名前を抽出して `grep -qx 'gh-token'`（行全体の完全一致）で判定した。採らなかった解釈: 行全体との完全一致（バージョン・リポジトリが常に並ぶ保証が無い）、`^gh-token([[:space:]]|$)` 系のアンカー（文字列先頭一致は今回の見落としと同種の穴を開けやすい）
4. 足りなかった情報・検証手段: 実在する gh-token 似の名前（gh-token-extra 等）の extension が実際に存在するかは未確認（テストは合成のリスト出力で書いた）
5. 改善案: なし（この修正で変えるべき skill・docs の文言は無い）

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: 78df0288166001d9c21e2540997df1570267e8e2

1. 詰まった箇所: scripts/agent-gh: `awk '{print $2}'` でも導入済み判定が通らず、手動再現で原因を特定。実出力の NAME 欄「gh token」にスペースが含まれるため、awk の既定区切り（スペース・タブ両方）では NAME が割れて REPO 欄が第 3 フィールドになる。`awk -F'\t'` でタブに限定して解決
2. 曖昧だった文: 「`gh extension list` の実際の出力形式（`gh <command>` + リポジトリ名の列）を確認した上で」（指揮役の must-fix 通知）— 「確認」の手段が sandbox で使えるかに書かれていない
3. 仮定と解釈: sandbox の `gh extension list` は認証を求められ exit 4 で実測できなかったため、形式は一次情報で確認した: cli/cli の pkg/cmd/extension/command.go（列は NAME・REPO・VERSION、NAME は `gh <コマンド名>`）と go-gh の tableprinter（非 TTY は TSV・ヘッダーなし）。コマンド置換でのキャプチャは常に非 TTY なのでタブ区切りと仮定した。採らなかった解釈: TTY 形式（スペース 2 つ区切り）も判定に入れる（コマンド置換では起こらないため不要）
4. 足りなかった情報・検証手段: 認証不要で `gh extension list` の実物出力を見る手段（hosts.yml の読める環境での実測）。ソース確認で代替した
5. 改善案: .agents/skills/implement-issue/SKILL.md の「手順 4」に「外部コマンドの出力形式に依存する判定は、実行できなくても公式実装の一次情報（ソース）で列構成を確かめる」を足す

使ったスキル: implement-issue

## 実装者メモ（round 5）

対象 head: 8883ad511d69f9ea90143e3c6835ae96622be1b5

1. 詰まった箇所: なし。レビュー指摘のとおり `${line//"$v"/<秘密>}` に複数行 PEM 全体を渡す構造では 1 行の stderr に鍵の一部行が現れたとき伏せられず、複数行ダミー鍵のテスト（token_fail_masks_multiline_key_fragment）を先に足して red で再現してから直した
2. 曖昧だった文: 「空行と `-----BEGIN/END ...-----` 以外の各行を伏せる値として `mask_secret_line` に渡す」（指揮役の must-fix 通知）— BEGIN/END 行自体を伏せない根拠（行の内容は鍵の種別を示すだけで秘密ではない）は通知に書かれていない
3. 仮定と解釈: 指示のとおり BEGIN/END 行と空行は伏せ対象から外した。種別の開示が問題になる運用は本 Issue の対象外（発行者・APP ID は行の別の場所で伏せている）。nit（github_pat_ / JWT の伏せパターン追加）は「対応は任意」との明記により対応しない（implement-issue の決まり: nit は指示があったときだけ）
4. 足りなかった情報・検証手段: fnox に登録された実際の鍵の PEM 形式（PKCS#1 か PKCS#8 か等）の確認手段。BEGIN 行の接頭辞匹配（-----BEGIN*）で両方を cover するため影響なしと判断した
5. 改善案: .agents/skills/write-issue/SKILL.md の「受け入れ条件の書き方」に「秘密を出す条件は、秘密の実際の形（複数行 PEM 等）で確かめるよう指定する」を足す

使ったスキル: implement-issue
