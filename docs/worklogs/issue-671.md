# Issue #671 の作業ログ

## 実装者メモ（round 1）

対象 head: 47c7a89099bcb48da3739338178b1b2065ac682d

1. 詰まった箇所: 手順 4: `conductor_thread_id` は patrol-state.json には実在するが runbook の項目表に無く、pelway-agent-watch も Issue #555 以降は宛先として読まない。書き手（指揮役の記録）・読み手（agent-watch の inbox.rs）の実態を worklog とコードで確かめてから、持ち主の記録として採用した
1. 詰まった箇所: 手順 6: 検証の途中で割り当ての worktree（conductor-rebuild-20261-81a85fdf7efbc410）が別スレッド（agent/669-pending-sends-watch）に reset・checkout され、未コミットの変更は a669e147（"issue-671 leftover"）に退避されていた。それを /tmp の新規 worktree に stash apply で復元して続行した
2. 曖昧だった文: 「自分が `conductor_thread_id` と一致しないときは上書きしないか、不一致を記録する振る舞いが定義され実装される」（Issue #671 本文「受け入れ条件」）
3. 仮定と解釈: 「上書きしないか、不一致を記録する」を選言と読み、拒否と `anomalies` 記録の両方を行う解釈を選んだ（拒否だけでは現指揮役に試みが見えないため）。オフセット巻き戻しも拒否を既定とし、意図した巻き戻しは `--allow-offset-regression` で記録して継続する道を残した。conductor-watch.sh はスレッドではないため `--writer conductor-watch` でスレッド照合の対象外とし、merge patch への置き換えで「読み込み後に他の書き手が足した項目を消さない」形にした
4. 足りなかった情報・検証手段: 旧指揮役スレッドがどの版の手順で patrol-state.json を直書きしていたかの記録。Issue の未決事項どおり起動自体は止められないため、保護は「この手順を通る書き手への強制」（conductor_thread_id 照合）と「手順を通らなくても効く事故止め」（オフセットの単調増加の検査）の両方で設計した。生きた 2 つの指揮役スレッドの同時書き込みは試験では再現できず、共通手順の試験とロック付き読み替えで担保した
5. 改善案: `.agents/skills/command-team/SKILL.md` の「決まり」に「patrol-state.json の書き込みは `scripts/patrol-state-update` にだけ出す（直書きしない）」の 1 行を足すと、runbook を読み直さない指揮役にも届く

使ったスキル: implement-issue
