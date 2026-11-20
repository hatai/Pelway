# Issue #489 の作業ログ

## 実装者メモ（round 1）

対象 head: 7e1a7e1b5a427ebd596015fe430b15d12b63583e

1. 詰まった箇所: verification/section-correspondence.json: 新設節に対応する元文書が無く、既存161行の source は全て /workspace パスだった。Issue #489 の URL を source、archive=null として登録した。
2. 曖昧だった文: 「紛らわしい語（「AI」を含まない人名に「AI」を付けるなど）の扱い：UI 担当の案を受けて hatai が決める。決まるまでは「バッジは表示名と別に必ず出す」だけを書く。」（Issue #489「未決事項」）
3. 仮定と解釈: 「だけを書く」は節の中に未決の存在自体を残すことを禁じないと解釈し、他章の未決は 08 へ集約する本文規則に従って U08 を登録し 06 の節から参照した。ZWJ は「絵文字の並びだけ許す／全面禁止」の二者から、絵文字シーケンス判定規則の新設を避ける全面禁止を選んだ。
4. 足りなかった情報・検証手段: 受け入れ条件は全て「目視で確認」で機械試験が無いため、節の必須記述を検査する python スクリプトを /tmp に書いて手順4の失敗→成功の証拠に使った。Switch の agent_display_name.py / names.py には Issue 引用のリンク情報から設計思想だけを参照し、コードは読み込みも写しもしていない。
5. 改善案: docs/technical/v0.7/README.md に「本文へ新節を追加したときは verification/current-sections.json・section-correspondence.json と source-correspondence.md へ登録し、verify.py を再実行して報告へ記録する」を足す。

使ったスキル: implement-issue（.agents/skills/implement-issue/SKILL.md）、write-pull-request（.agents/skills/write-pull-request/SKILL.md）
