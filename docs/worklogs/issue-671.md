# Issue #671 の作業ログ
## 実装者メモ（round 1）
対象 head: 47c7a89099bcb48da3739338178b1b2065ac682d
1. 詰まった箇所: 手順 4: `conductor_thread_id` は patrol-state.json には実在するが runbook の項目表に無く、pelway-agent-watch も Issue #555 以降は宛先として読まない。書き手（指揮役の記録）・読み手（agent-watch の inbox.rs）の実態を worklog とコードで確かめてから、持ち主の記録として採用した
1. 詰まった箇所: 手順 6: 検証の途中で割り当ての worktree（conductor-rebuild-20261-81a85fdf7efbc410）が別スレッド（agent/669-pending-sends-watch）に reset・checkout され、未コミットの変更は a669e147（"issue-671 leftover"）に退避されていた。それを /tmp の新規 worktree に stash apply で復元して続行した
2. 曖昧だった文: 「自分が `conductor_thread_id` と一致しないときは上書きしないか、不一致を記録する振る舞いが定義され実装される」（Issue #671 本文「受け入れ条件」）
3. 仮定と解釈: 「上書きしないか、不一致を記録する」を選言と読み、拒否と `anomalies` 記録の両方を行う解釈を選んだ（拒否だけでは現指揮役に試みが見えないため）。オフセット巻き戻しも拒否を既定とし、意図した巻き戻しは `--allow-offset-regression` で記録して継続する道を残した。conductor-watch.sh はスレッドではないため `--writer conductor-watch` でスレッド照合の対象外とし、merge patch への置き換えで「読み込み後に他の書き手が足した項目を消さない」形にした
4. 足りなかった情報・検証手段: 旧指揮役スレッドがどの版の手順で patrol-state.json を直書きしていたかの記録。Issue の未決事項どおり起動自体は止められないため、保護は「この手順を通る書き手への強制」として設計した（conductor_thread_id 照合とオフセットの単調増加の検査の両方を手順の内側で行う。手順を通らない直書きの書き手には及ばない）。生きた 2 つの指揮役スレッドの同時書き込みは試験では再現できず、共通手順の試験とロック付き読み替えで担保した
5. 改善案: `.agents/skills/command-team/SKILL.md` の「決まり」に「patrol-state.json の書き込みは `scripts/patrol-state-update` にだけ出す（直書きしない）」の 1 行を足すと、runbook を読み直さない指揮役にも届く

使ったスキル: implement-issue
## 実装者メモ（round 2）
対象 head: 8bf29ff5e9294ba964a76366bf183af449d3689b
1. 詰まった箇所: must-fix「unreadable を空として上書きする fail-open」の修正で、拒否時に `{anomalies}` だけの最小状態を書き戻すと、次の書き込みは「読めるが持ち主もオフセットも無い」状態になり拒否が 1 回で抜けることに試験を足す過程で気づいた。回復印 `needs_conductor_claim` を残し、`--claim-conductor` の回復書き込みが来るまで拒否が続く形にした
2. 曖昧だった文: 「`--claim-conductor` 付きの書き込み（または明示の回復フラグ）が来るまで、`--writer-thread` の書き込みとオフセットを書く書き込みを拒否する」（pelway-review の must-fix）。「書き込みを拒否する」がその試行だけか、回復完了までの全試行かが読み取れなかった
3. 仮定と解釈: 「来るまで」を期間と読み、回復完了まで拒否が続く解釈を選んだ（その場限りの拒否では fail-open の穴が残るため）。回復印は patch からは操作できず、`--claim-conductor` だけが下ろす。unreadable 時に `--writer`（conductor-watch）がオフセットを書かない書き込みは、レビューの「オフセットを書く書き込みを拒否する」の文言どおり適用を許した。印の無い nit（O_NOFOLLOW、`--writer` の conductor_thread_id 制限、conductor-mismatch 後の指針、presence check の名前、`-I` フラグ）は決まりどおり直していない
4. 足りなかった情報・検証手段: unreadable からの回復運用（退避ファイルから現指揮役が手で戻す手順）は runbook に回復の材料としての言及までで、完全な手順は現指揮役の判断に委ねた
5. 改善案: `scripts/patrol-state-update` の `--help` 出力か runbook に「unreadable を見つけた現指揮役は、退避ファイルの中身を確かめて `--claim-conductor` で回復する」までの回復手順を足すと、異常時の判断が現指揮役ごとにばらつかない

使ったスキル: implement-issue
## 実装者メモ（round 3）
対象 head: 1c29fd897bfc56acd04494e776577c1724aeee4a
1. 詰まった箇所: must-fix 1（退避直後の中断で初回作成扱い）の再現試験。実行中の中断を試験する経路がなく、`os.replace` の 2 回目（退避コピー設置の次、回復状態の書き戻し）を SourceFileLoader + monkeypatch で失敗させる形で決定的に再現した
2. 曖昧だった文: 「有効な登録を更新する操作は明示的な `--claim-conductor` に限り、通常の patch による削除・無効化・別名義からの変更を拒否してください」（pelway-review の must-fix）
3. 仮定と解釈: 「更新する操作」を変更一般と読み、登録の有無・書き手の種類（`--writer`・`--writer-thread`）にかかわらず、通常の patch で `conductor_thread_id` が変わる書き込みは全て拒否する解釈を選んだ（登録前ファイルの patch での登録も拒否し、登録は `--claim-conductor` だけに揃えた。同じ値の書き戻しは変更にならないため許す）。must-fix 1 はレビュー提示の「退避コピーを保存してから正規パスを原子的に置き換える」を採り、退避を `os.replace` の移動から原子的なコピーに変えた
4. 足りなかった情報・検証手段: 中断の再現にはプロセス内の失敗注入が要り、シェル試験に埋め込む Python（`os.replace` を 2 回目で失敗させる）で済ませた。外部から kill する実時間の中断試験はタイミング依存で書けなかった
5. 改善案: `scripts/patrol-state-update` の「保護」の項に「通常の patch での変更・削除・無効化は拒否」の記述を足したので、`.agents/skills/command-team/SKILL.md` の patrol-state 記述にも同じ 1 行があるとスキルだけ読む指揮役にも届く

使ったスキル: implement-issue
## 実装者メモ（round 4）
対象 head: 20f56c235c189ff48c897127e5ed8d58744ba01c
1. 詰まった箇所: symlink・FIFO の再現試験で、旧実装のままだと FIFO の正規パスを open した時点で試験自体が止まり、赤確認が取れなかった。upd・pget・anoms の各ヘルパに timeout を入れ、ブロックする実装を試験の停止ではなく失敗として扱う形にした
2. 曖昧だった文: 「不適合は中身を読まず写さず拒否側へ」（pelway-review/claude の must-fix）
3. 仮定と解釈: 「拒否側へ」を unreadable と同じ照合不能の扱い（`--claim-conductor` 以外の `--writer-thread` の書き込みとオフセットを書く書き込みの拒否）と読み、`--writer` のオフセットを書かない書き込みは unreadable と同じく適用を許す解釈を選んだ（不適合なパスを回復状態で置き替えるだけで、中身の漏洩にはならないため）。anomalies は手順だけが書く項目として、base の記録に今回の記録を追記したものだけを結果に入れ、patch が anomalies に書いたものは採らない形にした。fstat の検査項目（通常ファイル・自己所有・リンク数 1・256KiB 上限）はレビュー文の指定をそのまま採用し、読み取りは検査した fd からの 1 回だけにして解析と退避で共用した
4. 足りなかった情報・検証手段: レビュー結果の通算数。この round で pelway-review のレビュー結果は 4 件目（codex@9d8579c3 CR・claude@c7344eab APPROVED・codex@c7344eab CR・claude@06dcc874 CR）。「3 回目のレビュー結果に must-fix が残ったら 4 回目の修正には入らない」の数え方（delegation 単位か結果単位か）は指揮役の指示で進めた
5. 改善案: state-untrusted は unreadable と違い退避コピーが残らないため、回復の材料は anomalies の reason（symlink・not-regular・not-owned・hardlinked・too-large・unreadable-path）だけになる。runbook の回復説明に reason の見方を足すと、差し替えの手口に応じた対処（リンク先の所在確認など）がしやすい

使ったスキル: implement-issue
## 実装者メモ（round 5・[文書のずれ] の nit のみ）
対象 head: 74ed4d69599963c56f7cf825b7a97fb2be1fb925
1. 詰まった箇所: なし（runbook:410 の移行の記述のみの修正）
2. 曖昧だった文: 「1 回きり `cp` で移す」— claude の approve の [文書のずれ] nit が、旧ファイルが残るため正規パスに通常ファイルが無い起動のたびに写し直す点と、確認と `cp` が `<state>.lock` の外で行われる点を指摘した
3. 仮定と解釈: 印の nit にはコードか文書で対応する決まりのうち、実際の条件を書く側（文書）を選んだ（挙動の変更はレビューの対象を広げるため）。印の無い nit（untrusted の open・fstat・read の失敗をデータ消失の危うさとして分ける、pelway 親ディレクトリの symlink、hardlinked・too-large の試験追加、ロック不適合の戻し方の記述）は決まりどおり対象外
4. 足りなかった情報・検証手段: なし
5. 改善案: 旧置き場のファイルを移行後に改名するか消して本当に 1 回きりにする挙動側の直しは、フォロー Issue 候補（レビュー文でも代替案として示されていた）

使ったスキル: implement-issue
## implementer memo — round 6（claude approve の [文書のずれ] nit 対応）
- head: 本コミット（9a030a05 の approve review の印付き nit 1 件への対応）
- 直し: runbook:382 の「初回起動時に引き継ぐ」と conductor-watch.sh:32 の「1 回きりの移行」を、実際の条件（旧ファイルは残り、正規パスに通常ファイルが無い起動のたびに写し直す）に合わせた。pr-size 対策で試験ファイルの空行を 10 行削った（意味のある行は不変）
- 結果: mise run verify 実施・pr-size +797/-13
## 実装者メモ（round 8。round 7 は停止時のメモで Issue #671 のコメントにのみ記録）
対象 head: 006749b0d4900bdadfb1ae5801850f6783518416
1. 詰まった箇所: 手順 9「修正依頼」: scripts/conductor-watch.sh の書き戻しで、main の #699（PR #705）の検査とこの PR の共通手順が同じ箇所を変えていた。#699 の write_ps を検査だけの check_ps にし、通った回だけ scripts/patrol-state-update を呼ぶ形で併合した
2. 曖昧だった文: 「両立する変更は両側残す。片方を採るときは、落とした側と理由を round サマリーに書く」（.agents/skills/implement-issue/SKILL.md の手順 9「修正依頼」）。どちらも書き戻しを守る変更で、両立できる部分と排他的な部分の区別が読み取れなかった
3. 仮定と解釈: 巡回の読み取り上限（4 MiB）と手順の読み取り上限（256KiB）の差は意図の食い違いと読み、手順の STATE_MAX_BYTES を 4 MiB に揃えた。手順を 256KiB のままにする解釈では、巡回が読める 256KiB 超 4 MiB 以下の記録が手順に置き換えられるため。手順が親ディレクトリを検査しない点は変えず、巡回が dir-invalid の回に手順を呼ばないことで済ませた
4. 足りなかった情報・検証手段: `.agents/skills/verify-pelway/verify-run.test.sh` がこの sandbox では /tmp の所有者 uid 65534 のために落ちる。`scripts/red-check origin/main -- bash .agents/skills/verify-pelway/verify-run.test.sh` で同じ 6 項目の失敗を再現し、環境起因と切り分けた
5. 改善案: .agents/skills/implement-issue/SKILL.md の「止まる条件」の pr-size の行に「衝突の解消で行数が増えたときは、空行・継続行を詰めた先例（548faa59）がある」と書くと、解消のたびに停止にならない

使ったスキル: implement-issue（初回の解消で読んだユーザーレベルの fix-merge-conflicts は、手順は implement-issue の手順 9 に合わせた）
## 実装者メモ（round 9）
対象 head: 0f0ca8d9c546d72fea9c95070ce65b23c9dc6702
1. 詰まった箇所: 手順 6: レビュー round 7（codex・claude とも request-changes、must-fix 3 + 1）への修正で pr-size が +978 / -56 になり（上限を 178 行超える）、push が `scripts/agent-gh` の pre-push ゲートで止まった。分割では収まらない（手順とその試験だけで +845 で、巡回の書き戻しを別 PR に分けても 1 本目が上限を超える。手順の守りはどれも前の round までの must-fix）。hatai の承認コメント https://github.com/hatai/Pelway/issues/671#issuecomment-7167069771 と PR の `pr-size-exception` ラベルで例外が認められ、0f0ca8d9 を push した
2. 曖昧だった文: 「追加 800 行または削除 2000 行の上限を超えたら止まる。push は原則しない（例外は次の項の相談のための push だけ）。」（.agents/skills/implement-issue/SKILL.md の手順 6）。相談のための push は例外として認められて読めるが、pre-push ゲートは例外のラベルと目印コメントが無い push を止める
3. 仮定と解釈: 巡回側の書き込み前の検査（`check_ps`）と手順内の上限の検査を両方残す案と、手順だけにまとめる案があり、保護が 1 箇所になる後者を選んだ（Issue #671 の受け入れ条件 4）。巡回がどの回も手順を呼ぶようになるため、main の試験 15〜17 の「一時ファイルが残らない」は、試験 10 と同じく手順が残すロックファイルを除外して数える形にした
4. 足りなかった情報・検証手段: 例外が認められるまで、手元の修正コミットを PR に載せる方法が無い。初回の push は GitHub 側の Internal Server Error で拒まれ、送り直したら通った
5. 改善案: .agents/skills/implement-issue/SKILL.md の手順 6 の pr-size の段落に「pre-push ゲートがあるため、上限を超えた head は push できない。手元のコミットの SHA と行数を Issue に書いて止める」と書くと、相談のための push を試みずに済む

使ったスキル: implement-issue
