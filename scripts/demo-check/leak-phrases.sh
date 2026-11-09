#!/usr/bin/env bash
# demo-check 段階 (b) leak-phrases: ビルド成果物・ログの禁止語検索
# 予定する検査内容（未実装）:
#   - target/ のデモ成果物・デモ実行ログ・fixture を対象に禁止語リストを検索し、1 件でもあれば失敗
#   - 禁止語リストの置き場所と中身は QA と合意してから決める（ここでは定めない）
#   - 秘密値そのものはリストに書かない（ハッシュ照合などの方式は未決）
# 実装するまでは NOT IMPLEMENTED を出して非 0（3）で終わる。
set -u
echo "NOT IMPLEMENTED: demo-check stage leak-phrases (ビルド成果物・ログの禁止語検索)" >&2
exit 3
