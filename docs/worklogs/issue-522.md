# Issue #522 の作業ログ

## 実装者メモ（round 1）

対象 head: d40d164201246cf50b712e7d686528c5ae336740

1. 詰まった箇所: scripts/agent-gh の app_token: 既存の `trap 'rm -f "${key_file:-}"' EXIT` が gh 隔離用の EXIT trap を上書きしてしまう（bash は EXIT trap を 1 つしか持てない）。cleanup_paths 配列と単一の cleanup ハンドラに統合した。
2. 曖昧だった文: 「`trap` で終了時（正常終了・異常終了とも）に作成した一時ディレクトリを確実に削除する」（Issue #522 本文「やること」）
3. 仮定と解釈: 「異常終了」は exit 2〜5 を指し、トラップされないシグナル終了は含まないと解釈。exec で gh / git に置き換えると EXIT trap が走らないため呼び出しから exec を外して exit $? で終了コードを渡す形にした（stdin と終了コードは子プロセス継承で同等）。「dev・review（および他のロール）」の他のロールには qa を含めた。
4. 足りなかった情報・検証手段: レビュー sandbox（codex_review / claude_review）実機での再確認（本 PR では合成データのオフラインテストで代替。#520 の実測と同じ GH_CONFIG_DIR の空ディレクトリ置き換えが機構）。
5. 改善案: scripts/tests/agent-gh.test.sh の「run()」に「rm -f "$CALL_LOG.token-env"」を足す（他の CALL_LOG 系の消し方に揃える。現在は偽 gh が上書き書き込みするため実害なし）

使ったスキル: implement-issue
