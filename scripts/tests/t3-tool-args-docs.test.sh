#!/usr/bin/env bash
# t3_thread_launch / t3_thread_send の引数の例と、Failed to connect の確かめ方の
# 必須文言と JSON 例の形を固定するテスト（wording pin、Issue #688）。
# 位置づけ: 「文言と例の形が文書に存在する」という観測可能な事実だけを確かめる
# 決定的な検査であり、例が指揮役の実際の運用に合うかの意味評価はしない
# （AGENTS.md「LLM judges」）。例や文言を変えるときはこのテストと同じ PR で直す。
set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RUNBOOK="$ROOT/docs/agents/grokbot-runbook.md"
COMMAND_TEAM="$ROOT/.agents/skills/command-team/SKILL.md"
IMPLEMENT_ISSUE="$ROOT/.agents/skills/implement-issue/SKILL.md"
failures=()
got="" want="" smode=""

fail() { failures+=("$1"); }

# check_in <name> <入力（標準入力相当の文字列）> <pattern...>: すべての pattern があれば合格。
check_in() {
  local name="$1" text="$2"
  shift 2
  local p
  for p in "$@"; do
    if ! grep -qF "$p" <<<"$text"; then
      fail "$name: 無い: $p"
    fi
  done
}

# --- 節の切り出し ---
sec4="$(awk '/^## 4\./{f=1} /^## 5\./{f=0} f' "$RUNBOOK")"
sec7="$(awk '/^## 7\./{f=1} /^## 8\./{f=0} f' "$RUNBOOK")"
if [[ -z "$sec4" || -z "$sec7" ]]; then
  fail "runbook の節切り出しに失敗（## 4. / ## 7. が見つからない）"
fi

# --- 条件1: §4 に t3_thread_launch の JSON 例があり、最上位キーが 7 つ ---
# ```json フェンスの中身を順に取り出し、t3_thread_launch の例（title+modelSelection
# を持つもの）と t3_thread_send の例（threadId を持つもの）を選ぶ。
launch_json="" send_json=""
while IFS= read -r -d '' block; do
  if jq -e 'has("modelSelection") and has("title")' >/dev/null 2>&1 <<<"$block"; then
    [[ -z "$launch_json" ]] && launch_json="$block"
  elif jq -e 'has("threadId")' >/dev/null 2>&1 <<<"$block"; then
    [[ -z "$send_json" ]] && send_json="$block"
  fi
done < <(awk '/^```json/{f=1;b="";next} /^```/{if(f){printf "%s%c", b, 0; f=0} next} f{b=b $0 "\n"}' <<<"$sec4")

if [[ -z "$launch_json" ]]; then
  fail "cond_launch_json: §4 に t3_thread_launch の JSON 例（title と modelSelection を持つ json フェンスブロック）が無い"
else
  got="$(jq -r 'keys | sort | join(",")' <<<"$launch_json")"
  want="clientRequestId,message,modelSelection,projectId,runtimeMode,title,workspaceStrategy"
  if [[ "$got" != "$want" ]]; then
    fail "cond_launch_keys: t3_thread_launch 例の最上位キーが '$got'（期待: '$want'）"
  fi
  # modelSelection のキー
  ms="$(jq -r '.modelSelection | keys | sort | join(",")' <<<"$launch_json")"
  if ! jq -e '.modelSelection | has("instanceId") and has("model")' >/dev/null <<<"$launch_json"; then
    fail "cond_launch_ms_keys: modelSelection に instanceId と model が無い（keys: $ms）"
  fi
  if jq -e '.modelSelection | keys - ["instanceId","model","options"] | length > 0' >/dev/null <<<"$launch_json"; then
    fail "cond_launch_ms_keys: modelSelection に余分なキーがある（keys: $ms）"
  fi
  # 例の値がツール定義で受け付けられる名前
  inst="$(jq -r '.modelSelection.instanceId' <<<"$launch_json")"
  case "$inst" in
    opencode|acpRegistry_devin|codex_review|claude_review|devin_review) : ;;
    *) fail "cond_launch_instanceId: modelSelection.instanceId が '$inst'（runbook の provider 名でない）" ;;
  esac
  rm_mode="$(jq -r '.runtimeMode' <<<"$launch_json")"
  case "$rm_mode" in
    approval-required|auto-accept-edits|auto|full-access) : ;;
    *) fail "cond_launch_runtimeMode: runtimeMode が '$rm_mode'（ツール定義の値でない）" ;;
  esac
  wstype="$(jq -r '.workspaceStrategy.type' <<<"$launch_json")"
  case "$wstype" in
    root|existing_worktree|worktree) : ;;
    *) fail "cond_launch_wstype: workspaceStrategy.type が '$wstype'（ツール定義の値でない）" ;;
  esac
fi

# --- 条件2: title が必須であることが書かれている ---
# launch の例と同じ段落群に「title」「必須」を含む記述があること（同一行内）。
if ! grep -n 'title' <<<"$sec4" | grep -q '必須'; then
  fail "cond_title_required: §4 に t3_thread_launch の title が必須である旨の記述が無い"
fi

# --- 条件3: t3_thread_send の JSON 例（キー 4 つ） ---
if [[ -z "$send_json" ]]; then
  fail "cond_send_json: §4 に t3_thread_send の JSON 例（threadId を持つ json フェンスブロック）が無い"
else
  got="$(jq -r 'keys | sort | join(",")' <<<"$send_json")"
  want="clientRequestId,message,mode,threadId"
  if [[ "$got" != "$want" ]]; then
    fail "cond_send_keys: t3_thread_send 例のキーが '$got'（期待: '$want'）"
  fi
  smode="$(jq -r '.mode' <<<"$send_json")"
  case "$smode" in
    auto|queue|steer|restart) : ;;
    *) fail "cond_send_mode: t3_thread_send 例の mode が '$smode'（ツール定義の値でない）" ;;
  esac
fi

# --- 条件4: delegate_task の形を流用しない旨の注意 ---
check_in "cond_no_delegate_shape" "$sec4" \
  'delegate_task` の `task`・`target` の形を `t3_thread_launch` に、`text` を `t3_thread_send` に使わない'

# --- 条件5: §7 に Failed to connect の確かめ方 ---
# shellcheck disable=SC2088 # パターンは展開せず文書の字面と照合する
check_in "cond_failed_connect" "$sec7" \
  "Failed to connect to MCP server 't3-code'" \
  '~/.local/share/devin/cli/logs/' \
  'Invalid parameters for tool'

# --- 条件6: command-team に runbook の例の節への参照が 1 か所 ---
launch_refs="$(grep -n 't3_thread_launch' "$COMMAND_TEAM" | grep -c 'grokbot-runbook.md' || true)"
if [[ "$launch_refs" -ne 1 ]]; then
  fail "cond_command_team_ref: command-team に t3_thread_launch を含む grokbot-runbook.md への参照が ${launch_refs} か所（1 か所のはず）"
fi

# --- 条件7: implement-issue の delegate_task ひな形の近くに専用の注意 ---
if ! grep -n 'delegate_task` 専用で、`t3_thread_launch` には使わない' "$IMPLEMENT_ISSUE" >/dev/null; then
  fail "cond_impl_issue_note: implement-issue に「この形は delegate_task 専用で t3_thread_launch には使わない」の 1 行が無い"
fi

if ((${#failures[@]})); then
  printf 'FAIL %s\n' "${failures[@]}" >&2
  exit 1
fi
echo "PASS t3-tool-args-docs"
