# Issue #723 の作業ログ

## 実装者メモ（round 1）

対象 head: 78ae63f4000eb2bd406e569e11cf1b95f5eccc53

1. 詰まった箇所: 手順 6 の push: `scripts/agent-gh-resign` の `createCommitOnBranch` が HTTP 502 を 4 回返し、実装コミットは署名なしのまま remote に載った（push 自体は成功）。同じ時間帯に App トークンの発行（500）と `pr create`（GraphQL エラー、502）も失敗し、`pr create` は再試行で通った。
2. 曖昧だった文: 「正本の書き方は、v0.7 の本文を消さずに、上書きされた箇所へ「ADR 0014 で置き換え」の注記とリンクを入れる。前例は Octane の採用（8d48a677）。」（Issue #723「採用する方式」）。8d48a677 は 08 の A1・A2 の行を書き換えていて、注記を足す形ではない。
3. 仮定と解釈: hatai がこのスレッドの grilling で選んだ 5 つを決定として扱った（日付と t コードを ADR から外す／openbot の要約を ADR に入れない／運用の仕組みを「決定」と「土台の Issue で決める案」に分ける／v0.7 は元の文を残して注記／GLOSSARY は変えず「デスクトップアプリ」「ブラウザ版」に揃える）。Issue に `ready-for-agent` は無いが、hatai の直接の指示で着手した。v0.7 以外の 7 ファイルも、元の文を残して注記を足す形にそろえた。
4. 足りなかった情報・検証手段: 受け入れ条件の「表の行数が 20 であることを目で確かめる」に、公式の項目の一覧の取り方が無かった。electron/electron の `docs/tutorial/security.md` を `scripts/agent-gh dev api repos/electron/electron/contents/...` で読んで突き合わせた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「6. 検査してコミットし、行数を確かめてから push する」に「`agent-gh-resign` が GitHub の 5xx で失敗したときは、push は成功しているので先へ進み、次の push で付け替えを再試行する。署名なしのまま残ったら PR 本文に書く」を足す。

使ったスキル: grilling、domain-modeling、implement-issue、write-pull-request

## 実装者メモ（round 2）

対象 head: 7ea367f61e1b43fb0dabad46dfb046d1662aad94

1. 詰まった箇所: 手順 6 の push: `scripts/agent-gh-resign` が `createCommitOnBranch` で毎回 HTTP 502 になり、そのたびに PR #726 が自動で閉じた（付け替えの途中で ref が base まで巻き戻るため）。REST の `PATCH pulls/726 {"state":"open"}` で 2 回開き直し、以後の push は `PELWAY_AGENT_SKIP_RESIGN=1` にした。コミットは 3 つとも署名なし。
2. 曖昧だった文: 「それ以外は 1 回だけ再委託する（2 回連続で止まる条件）。」（`.agents/skills/implement-issue/SKILL.md` 手順 9）。claude の `failed` は投稿が GitHub の障害で通らなかったもので、must-fix 1 件の中身は報告に書かれていた。結果としては採用せず、指摘だけを直してから再委託した。
3. 仮定と解釈: claude の must-fix「ホストが同じ Release の `SHA256SUMS` だけで確かめるので、hatai のタグの push に結び付かない」を正しい指摘と判断した。artifact attestation（`desktop-release.yml` が `refs/tags/desktop-v*` から作ったこと）を確かめ、`desktop-v*` のタグの作成を hatai だけに限ることを、配布経路の「決定」に書き足した。署名を入れない決定とは両立する（鍵も費用も増えない）と解釈した。codex の `failed` は使用量の上限によるもので、上限が解ける時刻のあとに再委託する。
4. 足りなかった情報・検証手段: fine-grained トークンで attestation を読めるかを確かめる手段が手元に無い。ADR では「取りに行く仕組みの Issue で確かめる。書き込みの権限は足さない」とした。
5. 改善案: `scripts/agent-gh-resign` の「ブランチを base に巻き戻し」の段は、途中で失敗すると PR が閉じる。`.agents/skills/implement-issue/SKILL.md` の手順 6 に「付け替えの失敗のあとは `scripts/agent-gh dev pr view <M> --json state` で PR が開いているかを確かめ、閉じていれば `api --method PATCH pulls/<M>` で開き直す」を足す。巻き戻さずに付け替える方式も検討したい。

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 107a288e5cc6c27a1e185fde237d4de5e4308db4

1. 詰まった箇所: `docs/adr/0014-desktop-client-electron.md` の配布経路: round 2 で本体だけを attestation に結び付け、同じ Release から取る `latest*.yml` を照合していなかった（claude の must-fix。Release の資産を書き換えられる者が `version` を偽って古い版へ巻き戻せる）。
2. 曖昧だった文: 「sha256 を確かめてから入れ替える。」（Issue #723「未決事項」の 8）。何に対して確かめるか（同じ Release の `SHA256SUMS` か、タグに結び付いた値か）が書かれていない。
3. 仮定と解釈: ホストが `latest*.yml` を (a) タグの版と `version` の一致、(b) 本体の `sha512`・`size` の自前の計算との一致、(c) 今の版より新しいこと、で照合する形を「決定」に足した。`stagingPercentage` の改ざんは公開の速さと停止だけに効くので許すと書いた（`desktop:rollout` の案と両立させるため、`latest*.yml` 自体への attestation は選ばなかった）。直したのは must-fix と `[文書のずれ]` の nit（「v0.7 との関係」の列挙）だけで、ほかの nit 3 件（fine-grained トークンは Contents read になる、Linux の safeStorage の平文、人の発言のリンク）は手順どおり直していない。
4. 足りなかった情報・検証手段: electron-updater が `latest.yml` のどの項目で更新を判断するかは実行して確かめていない（レビューの読みと公開の仕様に拠った）。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の受け入れ条件の書き方に「配布物の完全性を決める Issue では、何に対して確かめるか（信頼の根）を書く」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 4）

対象 head: 5ecfc13fa88a60303f78658cc4fe3cd1c1de3db6

1. 詰まった箇所: `docs/adr/0014-desktop-client-electron.md` の配布経路: codex の must-fix 2 件（タグは作成だけでなく付け替え・削除も守り、attestation をタグが今指すコミットに照合する／Release の `latest*.yml` をそのまま置くと国内ホストの外の URL を指せる）。最初の codex の委託（run 7167538818）は使用量の上限で `failed` を返したあと、上限の解除で自動で再開し、head 9d84a7e をレビューした。予約していた再委託は消した。
2. 曖昧だった文: 「round 3 のレビュー結果に must-fix が残っていたら、4 回目の修正には入らず、止まる条件の形でエスカレーションする」（`.agents/skills/implement-issue/SKILL.md` 手順 9）。指揮役が伝えた hatai の判断（t629u：ラウンドの上限は気にしない、その場合のマージは hatai の admin マージ）に従い、止まらずに直した。
3. 仮定と解釈: 公開の判断はタグの push とみなし、取り下げはタグの削除か新しい版で行う形を選んだ（ホストで Release の公開者を確かめる形は選ばなかった。確かめる手段が API の読み取りの権限に依るため）。ホストは Release の `latest*.yml` を配らず、検証した値だけで作り直し、取得先は Caddy の下の相対の名前だけにした。claude の nit（`releaseNotes` などの照合しない項目、`files[].url` の絶対 URL）もこの形で同時に閉じる。safeStorage の nit は `[文書のずれ]` ではないので直していない。
4. 足りなかった情報・検証手段: electron-updater が作り直した manifest の相対の名前とブロックマップをどう解決するかは実行して確かめていない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 に「使用量の上限で `failed` を返した委託は、上限の解除で自動で再開し投稿することがある。再委託の予約は、元の委託の実行の投稿が無いことを確かめてから出す」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 5）

対象 head: b990a85814e4c3195e16d887ae8ff7324f5908fa

1. 詰まった箇所: `docs/adr/0014-desktop-client-electron.md` の配布経路: round 4 の書き換えで、新しさの判定が Release の `latest*.yml` の版に残った（codex と claude が同じ must-fix を出した）。codex はもう 1 件、取り込んだあとのタグの削除が配布に効かないことを must-fix にした（claude は同じ点を nit にした）。
2. 曖昧だった文: 「公開を取り下げるときは、hatai がタグを消すか、より新しい版を出す。」（round 4 で自分が書いた ADR の文）。取り込んだあとの版に効くかを書いていなかった。
3. 仮定と解釈: 版の比較もタグの版で行い、Release の `latest*.yml` からは `stagingPercentage` だけを読む形にした。取り下げは、GitHub が「タグは無い」と答えたときだけとみなし、取得の失敗は取り下げとみなさない。取り下げたら一つ前の版の manifest に戻す。直したのは must-fix と、codex の `[文書のずれ]` の nit（ADR 0012 の外向きの通信の 2 経路との関係）だけで、claude の nit 3 件（ブロックマップの出どころ、公開の日時の出どころ、トークンは Contents read になる）は直していない。
4. 足りなかった情報・検証手段: GitHub の API でタグが「無い」ことと障害を区別する応答（404 と 5xx）の扱いは、取りに行く仕組みの Issue で実装と試験で確かめる必要がある。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の手順 9 の「Request changes のとき」に「配布や認可の決定を書き換えるときは、前の round で塞いだ経路の条件を消していないかを、前の round のレビューの must-fix と突き合わせてから push する」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 6）

対象 head: a67c50202cfca8669584887cd6f79cb55e43934f

1. 詰まった箇所: `docs/adr/0014-desktop-client-electron.md` の配布経路: codex の must-fix 1 件（取り下げの照合が配布中の版だけで、Caddy に残る古い版と戻し先の版に当たっていない）。claude は approve で、`[文書のずれ]` の nit 1 件（ADR 0012 の外向きの通信の行に ADR 0014 の注記が無い）があった。
2. 曖昧だった文: 「タグが残っている一つ前の版の `latest*.yml` に戻す。」（round 5 で自分が書いた ADR の文）。戻し先の版にも来歴とタグのコミットの照合を当てるかを書いていなかった。
3. 仮定と解釈: Caddy から取れるすべての版を毎回照合し、タグが消えたか別のコミットを指す版は本体を外す。`latest*.yml` は照合を通る最も新しい版で作り直し、通る版が無ければ置かない形にした。ADR 0012 の 39 行目に、ADR 0014 が足した取得の経路への注記を入れた。claude の `[文書のずれ]` でない nit 4 件（比較に使うタグの明記、404 の区別、ブロックマップと日時の出どころ、照合の失敗の通知）は直していない。
4. 足りなかった情報・検証手段: 全部の ref が 404 になる場合（リポジトリの非公開化やトークンの失効）とタグだけが無い場合の区別は、取りに行く仕組みの Issue の実装と試験で確かめる必要がある。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「受け入れ条件の書き方」に「ADR が別の ADR の決定を広げるときは、元の ADR の該当の行にも注記を入れることを条件にする」を足す。

使ったスキル: implement-issue
