# Issue #548 の作業ログ

## 実装者メモ（round 1）

対象 head: a0ec119116f12fed6a8b2a1aef5c9c5c556341af

1. 詰まった箇所: `.agents/skills/implement-issue/SKILL.md` の新しい節の置き場所: 手順 8・10・止まる条件の 3 か所から使うため、手順の番号の中に入れず、手順 10 の後に独立した節「ターンの終わりの状態の行」と「巡回から定型の指示を受けたとき」として置き、手順 8・10 から参照した。
2. 曖昧だった文: 「どの終わり方でも、最後の返答の全体を `キー: 値` の行だけにし、その中に `state:` の行を入れる」（Issue #548 の受け入れ条件）— 説明の文をどこに書くかが書かれていないため、「最後の返答より前のメッセージに書く」と望ましい行動の形で足した。
3. 仮定と解釈: 起こしてもらう仕組みを仕掛けられないときは `waiting-*` でなく `working` にする、と解釈した（巡回は `working` をすぐ指示の対象にするため、待ちの取りこぼしを巡回が拾える）。巡回の指示を受けたときの再開先は、GitHub の実状態ごとの表にした。
4. 足りなかった情報・検証手段: 実装担当（GLM・SWE-2）が新しい形を守るかは、この PR では確かめられない（効果測定は command-team 手順 4 で、モデルごとの「状態の行の無い終わり方」の割合で測る）。
5. 改善案: `docs/agents/notify-payloads.md` の「例」の JSON の `report`（4 行の古い形）を、今の報告の形に揃える（この Issue の範囲外として残した）。

使ったスキル: implement-issue、improve-skills（`.agents/skills/improve-skills/SKILL.md`）、writing-for-agents（`.agents/skills/writing-for-agents/SKILL.md`）、write-pull-request

### improve-skills 手順 7 の引き渡し記録

- 候補の出どころ:
  - `docs/worklogs/2026-11-13-conductor-substitute.md:70`（#251）
  - `docs/worklogs/2026-11-14-conductor-substitute.md:261`（#256）
  - `docs/worklogs/2026-11-18-conductor-substitute.md:253`（#381）
  - Issue #504 の背景（#488・#480・#490）
  - `docs/worklogs/issue-219.md:9`
- 対象の改善点: implement-issue。手順の欠落（待ち・途中で終わるときに残す印が無い。巡回の指示を受けたときの手順が無い）
- 期待する変化:
  - 待ちで終わったまま起きない実装スレッドを、巡回が次の巡回（約 8 分）で拾えるようになり、放置の時間が減る（指揮役の側で測る）。
  - 状態の行の無い終わり方の割合が、実装担当のモデルごとに 0 に近づく（実装担当の側で測る）。
