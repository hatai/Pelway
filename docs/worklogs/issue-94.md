# Issue #94 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 手順4（TDD）: Issue が測定タスクでコード変更が無く、先に書けるテストが存在しない。podman+runsc のセットアップでも、podman 4.9.3 が `containers.conf` の `[engine.runtimes]` に書いた引数を conmon に渡さない（`--runtime-arg` が転送されない）ため、ラッパースクリプトで runsc に `--ignore-cgroups --platform=ptrace` を固定するまで gVisor が起動しなかった。
2. 曖昧だった文: 「受け入れ条件がテストで確かめられる形になっていない」（`.agents/skills/implement-issue/SKILL.md` の止まる条件の表）。Issue #94 には「目的」「受け入れ条件」の見出し自体が無く、この条件に該当するかの判断が迷った。
3. 仮定と解釈: Issue の「やること」4項目（rootless Podman での動作、起動時間/syscall互換性/資源、egress deny、WSL2 の KVM）をすべて実測し、再現手順つきの測定報告書として残すことを受け入れ条件と解釈した。CI テスト化できない測定の検証は文書検査（typos・記述の整合）と人間による再現に委ねる。止まらずに進む選択をした。選ばなかった解釈: 受け入れ条件の欠如を理由に即 `needs-human` で止まる。
4. 足りなかった情報・検証手段: systrap 起動失敗（`failed to install seccomp notify rules: device or resource busy`）の原因特定。gVisor の GitHub issue に直接の一致が無く、WSL2 カーネル 6.18 特有かの切り分けには別カーネル環境での再実験が必要。libkrun は kvm グループ権限（sudo）が無いと実測できず、人間の操作を要する。
5. 改善案: `docs/agents/issue-tracker.md`（または Issue テンプレート）の「Issue 本文の書き方」に「測定・調査系 Issue には『受け入れ条件』として『どの環境で何を測り、どこに残すか』を書く」を足す。
