#!/usr/bin/env bash
# agent-sandbox のホスト側インストーラ（Issue #151）。
# リポジトリ内の scripts/agent-sandbox/ を各所に配置する。
#
#   ~/.config/opencode/plugins/pelway-sandbox/index.js      opencode プラグイン
#   ~/.config/opencode/plugins/pelway-sandbox/sandbox-shell bwrap wrapper（共有）
#   ~/.local/share/pelway-sandbox/sandbox-shell             同上（devin-impl 参照用）
#   ~/.local/bin/devin-impl                                 acpRegistry_devin 用 wrapper
#   ~/.config/devin-impl/config.json                        devin-impl の設定
#
# 前提: bwrap がインストール済み（mise/パッケージマネージャで導入）。
# 実行後に表示される settings.json の差分を ~/.t3/userdata/settings.json の
# providerInstances.acpRegistry_devin.config に反映し、T3 Code を再起動する。
set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src="$repo/agent-sandbox"

for cmd in bwrap; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "ERROR: $cmd が見つかりません（先にインストールしてください）" >&2
    exit 1
  fi
done

plugin_dir="$HOME/.config/opencode/plugins/pelway-sandbox"
share_dir="$HOME/.local/share/pelway-sandbox"
config_dir="$HOME/.config/devin-impl"

mkdir -p "$plugin_dir" "$share_dir" "$config_dir" "$HOME/.local/bin"
install -m 0755 "$src/sandbox-shell" "$plugin_dir/sandbox-shell"
install -m 0755 "$src/sandbox-shell" "$share_dir/sandbox-shell"
install -m 0644 "$src/opencode-plugin/index.js" "$plugin_dir/index.js"
install -m 0755 "$src/devin-impl" "$HOME/.local/bin/devin-impl"
install -m 0644 "$src/devin-config.json" "$config_dir/config.json"

cat <<EOF
インストール完了:
  $plugin_dir/index.js
  $plugin_dir/sandbox-shell
  $share_dir/sandbox-shell
  $HOME/.local/bin/devin-impl
  $config_dir/config.json

次に T3 Code の settings.json（~/.t3/userdata/settings.json）で
providerInstances.acpRegistry_devin.config に commandPath を追加:

  "providerInstances": {
    "acpRegistry_devin": {
      "config": {
        "commandPath": "$HOME/.local/bin/devin-impl"
      }
    }
  }

反映後に T3 Code を再起動し、docs/dev-environment.md「実装担当の
sandbox」の確認手順を実行してください。
EOF
