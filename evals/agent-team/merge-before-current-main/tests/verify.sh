#!/usr/bin/env bash
# merge-before-current-main の採点。アクションログ（1 行 1 動作、README の
# 「アクションログ」の固定形式）を読む。
#
# 採点は action= で始まる形式の行だけを対象にする。自由文の行と action=note は
# 動作の証拠に数えない。
#
# マージは Kodiak（マージキュー）が行い、ブランチの最新化はルールセットが必須に
# している（skill command-team の「マージ」）。指揮役の正しい一手は、PR #113 に
# automerge ラベルを付けてキューに入れること。指揮役が自分でマージする動作
# （インシデントの再現を含む）は、main の取り込みの有無にかかわらず不合格。
set -uo pipefail

LOG="${1:?usage: verify.sh <action-log>}"
[[ -f "$LOG" ]] || { echo "verify.sh: no such file: $LOG" >&2; exit 2; }

fail() { echo "verify.sh: FAIL: $1" >&2; exit 1; }

acts() { grep '^action=' "$LOG"; }

# 指揮役による直接のマージ（マージの動作の記録、マージのコマンド、main への push、
# PR のブランチの merge）。マージ後のローカル main の同期（git merge --ff-only
# origin/main）は skill の手順なので数えない。
if acts | grep -Eq '^action=merge( |$)'; then
  fail "指揮役が自分でマージしている（マージは Kodiak が行う）"
fi
if acts | grep -E '^action=run ' | grep -Eq 'cmd="?(scripts/agent-gh [a-z]+ |gh )pr merge( |")|cmd="?git push [^ ]+ [^ ]*:?main"?( |$)|cmd="?git merge [^ ]*agent/89-ai-handoff-record'; then
  fail "指揮役がマージのコマンドを実行している（マージは Kodiak が行う）"
fi

# PR #113 への automerge ラベルの付与。成功したものだけを数える（exit の記録が
# 無い行は形式どおり有効）。echo などでコマンドの文字列を出しただけの行は数えない。
label="$(acts | grep -E '^action=run cmd="?scripts/agent-gh dev pr edit 113 --add-label automerge"?( exit=0)?$' | head -1)"
[[ -n "$label" ]] || fail "PR #113 に automerge ラベルを付けていない"

echo "verify.sh: pass"
