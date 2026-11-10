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

## 2026-11-10 追記: スキル自身の手順で PR #64 を作成

- 手順1: HEAD sha を取り直した → 3d3fdeaa952bbdc7f0a267e65f31200c1da7714f。
  diff(main...HEAD)も読み直した(3 ファイル・128 行追加、コミット 1 個)。
- 手順2: origin/demo-t3/write-pull-request = 3d3fdea をローカルの remote-tracking
  記録で確認(fetch なし)。担当の push 済みとも一致。
- 手順3: 依頼の許可ネットは gh pr create / gh pr view のみだったため、
  スキルの `gh pr list --head` の代わりに `gh pr view`(カレントブランチ)で
  既存 PR の有無を確認 → なし(「no pull requests found」)→ gh pr create で新規作成。
- 手順4〜6: タイトルは変更の中身主体(回次・段階名・チケットID 不使用)。
  本文は推奨構成 6 セクション。QA は実行済みコマンドと出力のみ、未検証は
  「テスト実行はしていません」と明記。
- 手順7: PR URL・タイトル・本文をそのまま報告。
- 結果: https://github.com/hatai/Pelway/pull/64
- 本追記はコミットしない(コミットすると HEAD sha が変わり、PR 本文の
  「対象 HEAD」とずれるため。#59 の反面教師と同じ過ちを避ける)。
  よって working tree はこのファイルのみ dirty のまま担当へ引き継ぐ。

## 2026-11-10 追記 2: SKILL.md に定期ルール 1〜5 を追加

- ユーザー指示により「## 定期ルール(毎回守る)」セクションを新設し、
  既存の「マージはしない。git push / git fetch もしない」を含む形で 6 項目にまとめた:
  1. マージ・push・fetch 禁止(既存)
  2. worktree 外への書き込み禁止(許可が出ても)
  3. rm -rf 禁止(許可が出ても)
  4. ネット/gh は gh pr list / view / create / edit のみ
  5. 迷ったら作業ログに書いて進める
  6. PR 作業中は製品コード・他ブランチ・他 worktree・無関係 PR を変えない
- 手順3の gh pr list --head はそのまま(前回依頼で list を禁じたのは誤りで、
  今回スキル側で list を許可することが確定したため)。
- 既存の手順(事実収集・日本語タイトル・本文構成・誠実さルール・反面教師)は無変更。
- 前回「コミットしない」とした worklog を今回の指示でコミットする
  (ユーザーの明示指示。push はしないので PR #64 の head は変わらない)。

## 2026-11-10 追記 3: PR テンプレート追加とスキル更新

- ユーザー指示により `.github/pull_request_template.md` を新設。
  セクション構成は SKILL.md 手順5の推奨構成(概要/変更内容/確認方法・検証/
  レビューと QA/対象外/対象)をそのまま雛形化し、各セクションに
  HTML コメントで記入指針を添えた。
- SKILL.md 手順5を更新: 本文はテンプレートの構成に従う旨を明記し、
  `gh pr create` / `gh pr edit` がテンプレートを自動適用しないため
  自分で読んで埋める手順にした。従来の推奨構成リストは「構成の要点
  (詳細はテンプレート側が正)」として残し、テンプレートを正とする
  単一情報源の関係にした。
- SKILL.md 手順4を更新: 「タイトルは必ず日本語で書く。簡潔で
  分かりやすいものにする」を明示的な指示として追加(従来は
  「変更の中身が分かる日本語で書く」のみ)。
- 補足: gh 2.103.3 の `gh pr create` には `--template` フラグがあるが、
  雛形をそのまま本文にするだけで各セクションを埋めないため、
  スキルでは heredoc + `--body-file -` で完成形を渡す方式を維持した。
- コミットはローカルのみ(push なし)。

## 2026-11-10 追記 4: 時系列情報の分離と worklog の移動

- ユーザー指示により「スキルには事実のみを書き、時系列情報は書かない」
  方針を適用。SKILL.md から時系列的な記述を除去した:
  - 手順1の「(main 取り込みで head が進み、本文の sha が実態とずれた
    PR が実際にある)」という実例の括弧書きを削除(ルール自体は維持)。
  - 末尾の「## 反面教師(このリポジトリの実際の失敗)」セクションを削除。
    #59/#61/#62 の経緯は git の履歴と本 worklog に残るため、
    スキル本文にはルールのみを残す形にした。
- AGENTS.md に「## Facts over history」セクションを追加:
  スキルとドキュメントには事実(ルール・手順・定義・現在の状態)のみを書き、
  時系列情報(日付・経緯・何がいつ起きたか)は書かない。時系列情報は
  git log と、時系列自体が目的であるドキュメント(docs/worklogs/ の
  worklog など)に残す。
- 本ファイルを worktree ルートから docs/worklogs/2026-11-10-write-pull-request-skill.md
  に移動。docs/worklogs/ の既存命名規則(YYYY-MM-DD-<slug>.md)に合わせた。
  ルート置きは docs/dev-environment.md 等の他 worklog 参照と不整合だったため。
- SKILL.md の「作業ログに書いて進める」に docs/worklogs/ のパス規則を追記し、
  今後の worklog の置き場所を明示した。
- なお本ファイル自体は時系列情報が本質のドキュメント(worklog)なので、
  「Facts over history」の例外に該当する。
