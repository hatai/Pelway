#!/usr/bin/env bash
# demo-check 段階 (a) loopback: デモの待受が 127.0.0.1 だけであることの検査
# 予定する検査内容（未実装）:
#   - デモ起動中に待受ソケットを列挙し、127.0.0.1 以外（0.0.0.0・::・LAN アドレス）があれば失敗
#   - 設定値 PELWAY_BIND_ADDR が 127.0.0.1 で始まることの確認
#   - トンネル類（cloudflared 等）をデモプロセスが起動していないことの確認
# 実装するまでは NOT IMPLEMENTED を出して非 0（3）で終わる。
set -u
echo "NOT IMPLEMENTED: demo-check stage loopback (デモの待受が 127.0.0.1 だけであることの検査)" >&2
exit 3
