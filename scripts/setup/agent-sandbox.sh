#!/usr/bin/env bash
# agent-sandbox のホスト側インストーラ（Issue #151）。前提: bwrap 導入済み。
# 実行後、表示される差分を ~/.t3/userdata/settings.json に反映し T3 を再起動。
set -euo pipefail

src="$(cd "$(dirname "${BASH_SOURCE[0]}")/../agent-sandbox" && pwd)"

if ! command -v bwrap >/dev/null 2>&1; then
  echo "ERROR: bwrap が見つかりません（先にインストールしてください）" >&2
  exit 1
fi

plugin_dir="$HOME/.config/opencode/plugins/pelway-sandbox"
share_dir="$HOME/.local/share/pelway-sandbox"
config_dir="$HOME/.config/devin-impl"

mkdir -p "$plugin_dir" "$share_dir" "$config_dir" "$HOME/.local/bin"
install -m 0755 "$src/sandbox-shell" "$plugin_dir/sandbox-shell"
install -m 0755 "$src/sandbox-shell" "$share_dir/sandbox-shell"
install -m 0644 "$src/opencode-plugin/index.js" "$plugin_dir/index.js"
install -m 0755 "$src/devin-impl" "$HOME/.local/bin/devin-impl"
install -m 0755 "$src/devin-hook" "$share_dir/devin-hook"
install -m 0644 "$src/devin-config.json" "$config_dir/config.json"

cat <<EOF
インストール完了: $plugin_dir $share_dir $HOME/.local/bin/devin-impl $config_dir

次に ~/.t3/userdata/settings.json の providerInstances.acpRegistry_devin
.config に commandPath を追加し、T3 Code を再起動してください:

  "providerInstances": {
    "acpRegistry_devin": {"config": {"commandPath": "$HOME/.local/bin/devin-impl"}}
  }

確認手順は docs/dev-environment.md「実装担当の sandbox」を参照。
EOF
