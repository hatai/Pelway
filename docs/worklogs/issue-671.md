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
