#!/usr/bin/env bash
# demo-check 段階 (c) external-refs: デモ資産の外部参照検査
# 予定する検査内容（未実装）:
#   - デモ資産（HTML/CSS/WASM/フォント指定/設定）から http(s):// の外部 URL・外部ホストを抽出し、許可リスト外があれば失敗
#   - Web フォントのホスト参照も対象（fixture で確認する予定）
#   - 許可リストは空から始める（127.0.0.1 のみ）
# 実装するまでは NOT IMPLEMENTED を出して非 0（3）で終わる。
set -u
echo "NOT IMPLEMENTED: demo-check stage external-refs (デモ資産の外部参照検査)" >&2
exit 3
