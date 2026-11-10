# 作業ログ: write-pull-request スキル作成

- 日時: 2026-11-10
- worktree: demo-t3/write-pull-request (HEAD 7c7a35a043f32ebbb4455781ee0b4b36c822e8a0)

## 方針の決定

- 作業ログの専用ディレクトリ慣習はリポジトリに無し(docs/ は計画書・仕様書のみ)。
  → worktree ルートに本ファイル worklog-write-pull-request.md を置き、コミットに含める。
- SKILL.md の本文言語: このスキルの成果物は日本語 PR のため日本語で書く。
  frontmatter (name, description) は既存 .agents/skills/*/SKILL.md と同じ形式
  (name + description 一行、トリガーキーワード併記)。
- 反面教師 PR #59/#61/#62 の中身は指示書の記載のみで足りる。ネット接続禁止のため
  gh pr view では確認しない(スキル本文にも「実際の diff を読む」ルールとして反映)。
- git log の慣習: スキル追加は "chore: add ..." 形式(7c7a35a に準拠)。

## 実施記録

1. .agents/skills/ と .claude/skills/ の構造確認(全スキルが symlink 形式)。
2. SKILL.md 作成(.agents/skills/write-pull-request/SKILL.md、~100 行)。
3. symlink 作成: .claude/skills/write-pull-request → ../../.agents/skills/write-pull-request
   readlink 結果で相対パスを確認、symlink 経由で SKILL.md が読めることも確認。
   → ハーネス側でも新スキルとして認識された(system-update で出現)。
4. コミット(push なし)。

## 備考

- 製品コードは無変更。追加ファイルは SKILL.md・symlink・本作業ログのみ。
- 指示 8 項目と反面教師 3 PR の教訓はすべて SKILL.md に反映済み。
