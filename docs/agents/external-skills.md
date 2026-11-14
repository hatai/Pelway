# External skills: 外部スキルの取り込みと適用

`skills-lock.json` がハッシュ固定で管理する外部リポジトリ由来のスキルと、Pelway 側の適用ルールの置き場所の記録。

## 取り込みの手順

- 取り込みは `npx skills@1.7.6 add <owner/repo> -s <name>... -a universal -a claude-code -y` で行う。
  - `-a universal` が `.agents/skills/<name>` に実体を置き、`-a claude-code` が `.claude/skills/<name>` からシンボリックリンクを張る。`-a claude-code` 単独では `.claude/skills` にコピーされ、リポジトリの配置規約（実体は `.agents/skills` だけ）に合わない。
  - `skills@1.8.x` は `skills-lock.json` を `version: 2` に書き換え、既存全エントリの `computedHash` を再計算する。version 1 の形式を保つため `1.7.6` を使う。
- `skills-lock.json` のハッシュは取り込んだ内容の記録であり、取得元を固定する仕組みではない。版の指定がある取り込みでは、取り込み後に取得内容を指定された版（上流コミット）と突き合わせ、差分があれば止めて再照合する。
- 取り込んだ後は `diff -r .agents/skills .claude/skills` が空であることを確かめる。
- `actionbook/rust-skills`（38 スキル）と `penpot/penpot-ai-kit`（13 スキル）の取り込みの記録は `skills-lock.json` のみにある。上流コミットや適用ルールの記録はこの文書の対象外。

## 取り込みの discipline

- 同梱のスクリプト（`scripts/` など）は実行しない。
- `agents/` のメタデータ（`openai.yaml` など）は使わない。
- 採用していないスキルへの参照は無効とする。
- 輸入品の本文と正本（AGENTS.md・docs/・役割スキル）・適用ルールが衝突した場合は、正本と適用ルールが優先する。
- 自律スレッドは輸入スキルを名前の解決に任せず、役割スキルを入口として repo 内の明示したパスで読む。人間側の同名のユーザーレベル版はチームのスレッドでは読まない。
- 輸入品のファイル自体は書き換えない（`skills-lock.json` が管理するスキルには同梱ヘルパーも足せない。docs/agents/skill-helpers.md）。適用ルールはローカル管理の役割スキルに置く。

## mattpocock/skills（MIT）

上流: <https://github.com/mattpocock/skills>。取り込み時点の上流コミット: `5863806dd4e15c63174bee4e7515a4256792ecfa`（main の head として取得）。

| スキル | 上流のパス | Pelway 側の適用ルールの置き場所 | 実行しない同梱物 |
|---|---|---|---|
| `tdd` | `skills/engineering/tdd` | `.agents/skills/implement-issue/SKILL.md` の「輸入スキル」（手順 4 の方法の詳細） | `agents/openai.yaml` |
| `diagnosing-bugs` | `skills/engineering/diagnosing-bugs` | `.agents/skills/investigate-bug/SKILL.md` の「輸入スキル」（証拠集めと根本原因の方法） | `scripts/hitl-loop.template.sh`、`agents/openai.yaml` |
| `domain-modeling` | `skills/engineering/domain-modeling` | `.agents/skills/command-team/SKILL.md` の「輸入スキルの試行」（設計・ADR と grill 試行の割り当て） | `agents/openai.yaml` |
| `research` | `skills/engineering/research` | `.agents/skills/command-team/SKILL.md` の「輸入スキルの試行」（設計・ADR の割り当て） | `agents/openai.yaml` |
| `grilling` | `skills/productivity/grilling` | `.agents/skills/command-team/SKILL.md` の「grill 試行」 | `agents/openai.yaml` |
| `grill-with-docs` | `skills/engineering/grill-with-docs` | なし（`disable-model-invocation: true` の人間側セッション用の入口。チームのスレッドでは `grilling` と `domain-modeling` を直接読む） | `agents/openai.yaml` |

- 補助文書（実行しない同梱物ではなく、読む文書）: `tdd` の `tests.md` と `mocking.md`、`domain-modeling` の `GLOSSARY-FORMAT.md` と `ADR-FORMAT.md`。
- 適用範囲: 通常運用への移行が決まるまで、試行の対象と指揮役が試行として明示した割り当てに限る。効果の判定と移行は hatai が別に判断する。
- 未採用（輸入スキルの本文が参照しても無効）: `handoff`、`prototype`、`improve-codebase-architecture`、`codebase-design`、`code-review`、`to-spec`、`to-tickets`、`triage`、執筆系スキル、`grill-me`。

## 上流 LICENSE（mattpocock/skills）

`skills-lock.json` には通知の本文が含まれないため、LICENSE 全文と著作権表示をここに転記する。

```
MIT License

Copyright (c) 2026 Matt Pocock

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
