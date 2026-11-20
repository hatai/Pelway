# Issue #522 の作業ログ

## 実装者メモ（round 1）

対象 head: d40d164201246cf50b712e7d686528c5ae336740

1. 詰まった箇所: scripts/agent-gh の app_token: 既存の `trap 'rm -f "${key_file:-}"' EXIT` が gh 隔離用の EXIT trap を上書きしてしまう（bash は EXIT trap を 1 つしか持てない）。cleanup_paths 配列と単一の cleanup ハンドラに統合した。
2. 曖昧だった文: 「`trap` で終了時（正常終了・異常終了とも）に作成した一時ディレクトリを確実に削除する」（Issue #522 本文「やること」）
3. 仮定と解釈: 「異常終了」は exit 2〜5 を指し、トラップされないシグナル終了は含まないと解釈。exec で gh / git に置き換えると EXIT trap が走らないため呼び出しから exec を外して exit $? で終了コードを渡す形にした（stdin と終了コードは子プロセス継承で同等）。「dev・review（および他のロール）」の他のロールには qa を含めた。
4. 足りなかった情報・検証手段: レビュー sandbox（codex_review / claude_review）実機での再確認（本 PR では合成データのオフラインテストで代替。#520 の実測と同じ GH_CONFIG_DIR の空ディレクトリ置き換えが機構）。
5. 改善案: scripts/tests/agent-gh.test.sh の「run()」に「rm -f "$CALL_LOG.token-env"」を足す（他の CALL_LOG 系の消し方に揃える。現在は偽 gh が上書き書き込みするため実害なし）

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: f346b530b96d2f548f10c734511e8580f331022e

1. 詰まった箇所: scripts/agent-gh の app_token: レビューの must-fix が指す subshell の問題（親の EXIT trap が届かない・cleanup_paths が親に届かない）は実機で再現・確認できたが、指摘の「直し方」（`trap 'rm -f "${key_file:-}"' EXIT` を subshell 内に残す）だけでは鍵ファイルがまだ残った。EXIT trap の実行時には local 変数のスコープが既に終わっており `${key_file:-}` が空展開されるため（bash で検証）。key_file を local から外して subshell 内の trap からパスが見えるようにした。
2. 曖昧だった文: 「main の版は `trap 'rm -f "${key_file:-}"' EXIT` を subshell の中で設定していたので、この経路でも消えていた（5be66782 で導入）」（指揮役中継のレビュー結果・must-fix）
3. 仮定と解釈: 「この経路でも消えていた」は検証結果と合わない（local 変数は EXIT trap から空展開になり rm が空振り）ため、trap の形は指摘どおりに復しつつ local を外す修正を追加した。なお nit 2 件（シグナル転送の注記、worklog の対象 head と PR head のずれ）は対応していない。対象 head は implement-issue の定義どおりメモコミットの親を指し、PR の head とは異なる値になる。
4. 足りなかった情報・検証手段: レビュー担当が「消えていた」と判断した検証手段。こちらは bash の最小再現（local 変数を trap から参照する実験）で空展開を確認した。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「実装者メモを書く」の「対象 head」の説明に「（メモコミットの親であり、PR の head = メモを含むコミットとは異なる SHA になる）」を足す（レビューの nit 2 件目のような誤読を防ぐ）

使ったスキル: implement-issue（must-fix の検証に diagnosing-bugs 相当の切り分けは自力で実施、スキル本文は未読）
