# Issue #437 の作業ログ

## 実装者メモ（round 1）

対象 head: 5fcdd6309c5624ebaa90e3e3f546ea255bd6b0cd

1. 詰まった箇所: 対象スコープの事前確認で `cargo mutants -f crates/pelway-contract/src/member.rs` を実行したところ、cargo-mutants 27 系ではコマンドラインの `-f` が設定ファイルの `examine_globs` と union になる仕様で全体（406 ミュータント）が実行された。結果として「一度全体を回す」初回計測そのものになった（直列 --jobs 1 で約 31.5 分）。また初回 `mise run verify` の `[test]` タスクだけが rust 同期中の競合で rustc 1.75.0 に解決して失敗し、再実行では通過した（他は `conductor-watch-ensure.test.sh` の環境依存の揺らぎのみ。stash で自分の変更を退避した同一ツリーでも同じ "systemd: foreign watch present" 失敗を確認）。
2. 曖昧だった文: 「最初の対象は、IO がなく速い `pelway-contract` と承認ポリシーに絞る」（Issue #437 本文「やること」）。「承認ポリシー」が pelway-contract の `ApprovalPolicy`（src/approval.rs）だけを指すのか、pelway-client-core の承認カード状態機械（src/approval.rs）も指すのかが書かれていない。
3. 仮定と解釈: 「承認ポリシー」を、承認の振る舞い規則（冪等キーの払い出し・確認の再試行・409/403 で自動再送しない等）を実装する pelway-client-core/src/approval.rs を含むものと解釈した。対象は `examine_globs` で pelway-contract 全体（ApprovalPolicy を内包）＋ client-core の approval.rs とし、どちらの解釈でも条件を満たす範囲にした。client-core の decision.rs・input.rs は対象に入れなかった（「承認ポリシー」に直接対応しないため）。
4. 足りなかった情報・検証手段: cargo-mutants 27.1.0 には「コピーから特定ファイルを除く」オプションが無く（`--build-dir` も無い）、gitignore フィルタが git worktree（`.git` がファイル）で効くかは実測するしかなかった。センチネルの `.env` と `fnox.local.toml` を置いて `--leak-dirs` で残した一時コピーを検査し、両方ともコピーされないこと・`.git` もコピーされないことを確認した。
5. 改善案: `.agents/skills/write-issue/SKILL.md` の「やること」に相当する記述で、対象の範囲を名詞（例: 「承認ポリシー」）で書くときは、複数のファイル・crate に対応しうる場合にファイルパスまたは crate 名を括弧書きで添えるよう求める 1 文を足す。

使ったスキル: implement-issue
