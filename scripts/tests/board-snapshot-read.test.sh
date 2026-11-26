#!/usr/bin/env bash
# scripts/board-snapshot-read のテスト（Issue #814）。
# ボード読み取りスナップショット（ホスト側の conductor-watch.sh が書き出す JSON）を
# 指揮役が読む手順の 4 件の扱いを合成データで確かめる:
#   正常 / 古い書き出し（stale）/ 壊れた書き出し（broken）/ 書き出しが無い（missing）
# 本物のボードには触れない。時刻は PELWAY_BOARD_SNAPSHOT_NOW で固定する。
set -uo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ROOT="$(mktemp -d)"
trap 'rm -rf "$ROOT"' EXIT
failures=()

READER="$REPO/scripts/board-snapshot-read"
SNAP="$ROOT/snapshot.json"
NOW=1700000000  # 固定の「今」（2023-11-14T22:13:20Z 相当。値自体に意味は無い）

check() {
  local name="$1"; shift
  if "$@"; then printf 'ok   %s\n' "$name"; else printf 'FAIL %s\n' "$name"; failures+=("$name"); fi
}

# run_read: 環境を固定して reader を実行し、stdout を $ROOT/out.json、終了コードを rc に残す
run_read() {
  PELWAY_BOARD_SNAPSHOT="$SNAP" PELWAY_BOARD_SNAPSHOT_NOW="$NOW" \
    python3 "$READER" > "$ROOT/out.json" 2> "$ROOT/err.txt"
  rc=$?
}

jqv() { python3 -c 'import json,sys; print(eval(sys.argv[1], {"d": json.load(open(sys.argv[2]))}))' "$1" "$ROOT/out.json" 2>/dev/null; }

# 正常な書き出し（Todo が 2 件・Working が 1 件。順序は item-list の返却順）
write_ok_snapshot() {
  python3 -I - "$SNAP" "$1" <<'PY'
import json, sys
path, fetched = sys.argv[1], float(sys.argv[2])
items = [
  {"id": "PVTI_a", "type": "Issue", "number": 810, "repository": "hatai/Pelway",
   "status": "Todo", "labels": ["ready-for-agent"], "assignees": []},
  {"id": "PVTI_b", "type": "Issue", "number": 811, "repository": "hatai/Pelway",
   "status": "Todo", "labels": ["ready-for-agent", "P0"], "assignees": ["hatai"]},
  {"id": "PVTI_c", "type": "PullRequest", "number": 812, "repository": "hatai/Pelway",
   "status": "In review", "labels": [], "assignees": []},
]
json.dump({"version": 1, "fetched_at": fetched, "items": items}, open(path, "w"))
PY
}

# 1. 正常: 終了コード 0、ok=true、並び順と Status・Issue 番号・ラベルが読める
write_ok_snapshot $((NOW - 60))
run_read
check "ok: exit 0" test "$rc" -eq 0
check "ok: ok=true" test "$(jqv 'd["ok"]')" = "True"
check "ok: order preserved (810,811,812)" test "$(jqv '[i["number"] for i in d["items"]]')" = "[810, 811, 812]"
check "ok: status readable" test "$(jqv 'd["items"][1]["status"]')" = "Todo"
check "ok: labels readable" test "$(jqv 'd["items"][1]["labels"]')" = "['ready-for-agent', 'P0']"
check "ok: assignees readable" test "$(jqv 'd["items"][1]["assignees"]')" = "['hatai']"
check "ok: type readable" test "$(jqv 'd["items"][2]["type"]')" = "PullRequest"
check "ok: fetched_at echoed" test "$(jqv 'd["fetched_at"]')" = "1699999940.0"

# 2. 古い書き出し（巡回間隔 8 分の 2 倍 = 960 秒を超える）: ok=false・reason=stale・非 0 終了。
# 「0 件」として扱えないよう items を出さない
write_ok_snapshot $((NOW - 961))
run_read
check "stale: exit nonzero" test "$rc" -ne 0
check "stale: ok=false" test "$(jqv 'd["ok"]')" = "False"
check "stale: reason is stale" test "$(jqv 'd["reason"]')" = "stale"
check "stale: items not exposed" test "$(jqv '"items" not in d')" = "True"

# 2b. 境界: ちょうど 960 秒はまだ新しい（超えたときだけ古い）
write_ok_snapshot $((NOW - 960))
run_read
check "boundary 960s is fresh" test "$rc" -eq 0

# 3. 壊れた書き出し: JSON でない・形が違う・version が違う・未来の時刻、のいずれも
# ok=false・reason=broken・非 0 終了で、「0 件」とは扱わない
printf 'not json at all' > "$SNAP"
run_read
check "broken(not json): exit nonzero" test "$rc" -ne 0
check "broken(not json): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

echo '{"version": 1, "fetched_at": 0, "items": "not-an-array"}' > "$SNAP"
run_read
check "broken(bad shape): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

echo '{"version": 2, "fetched_at": '"$((NOW - 10))"', "items": []}' > "$SNAP"
run_read
check "broken(bad version): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

write_ok_snapshot $((NOW + 120))
run_read
check "broken(future fetched_at): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

echo '{"version": 1, "fetched_at": '"$((NOW - 10))"', "items": [{"id": "X", "type": "Issue"}]}' > "$SNAP"
run_read
check "broken(item without number): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

# 3b. 型の読めない項目・有限でない時刻も、例外を出さず契約どおり ok=false・reason=broken の
# JSON を返す（type が配列・オブジェクトだと集合の照合で、NaN・±Infinity だと時刻の計算で
# 例外になっていた）
python3 -c 'import json,sys; json.dump({"version": 1, "fetched_at": float(sys.argv[2]), "items": [{"id": "x", "type": "Issue", "number": 1}]}, open(sys.argv[1], "w"))' "$SNAP" "$NOW"
python3 -c 'import json; d = json.load(open("'$SNAP'")); d["items"][0]["type"] = ["Issue"]; json.dump(d, open("'$SNAP'", "w"))'
run_read
check "broken(type array): exit nonzero" test "$rc" -ne 0
check "broken(type array): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

python3 -c 'import json,sys; json.dump({"version": 1, "fetched_at": float(sys.argv[2]), "items": [{"id": "x", "type": {"name": "Issue"}, "number": 1}]}, open(sys.argv[1], "w"))' "$SNAP" "$NOW"
run_read
check "broken(type object): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

printf '{"version": 1, "fetched_at": NaN, "items": []}' > "$SNAP"
run_read
check "broken(NaN fetched_at): exit nonzero" test "$rc" -ne 0
check "broken(NaN fetched_at): reason is broken" test "$(jqv 'd["reason"]')" = "broken"
check "broken(NaN fetched_at): no traceback" test ! -s "$ROOT/err.txt"

printf '{"version": 1, "fetched_at": Infinity, "items": []}' > "$SNAP"
run_read
check "broken(Infinity fetched_at): reason is broken" test "$(jqv 'd["reason"]')" = "broken"

printf '{"version": 1, "fetched_at": -Infinity, "items": []}' > "$SNAP"
run_read
check "broken(-Infinity fetched_at): reason is broken" test "$(jqv 'd["reason"]')" = "broken"
check "broken(-Infinity fetched_at): no traceback" test ! -s "$ROOT/err.txt"

# 4. 書き出しが無い: ok=false・reason=missing・非 0 終了
rm -f "$SNAP"
run_read
check "missing: exit nonzero" test "$rc" -ne 0
check "missing: ok=false" test "$(jqv 'd["ok"]')" = "False"
check "missing: reason is missing" test "$(jqv 'd["reason"]')" = "missing"

# 5. symlink・通常でないファイルは broken として扱う（たどって読まない）
write_ok_snapshot $((NOW - 60))
ln -sf "$ROOT/target-outside" "$SNAP.link"
printf 'x' > "$ROOT/target-outside"
SNAP="$SNAP.link" run_read
check "symlink is not followed (missing target ok but link itself rejected or treated safely)" test "$rc" -ne 0

# 6. 空の items は正常（ボードが空なら 0 件は正しい）
echo '{"version": 1, "fetched_at": '"$((NOW - 10))"', "items": []}' > "$SNAP"
run_read
check "empty items is ok" test "$rc" -eq 0
check "empty items: zero entries" test "$(jqv 'len(d["items"])')" = "0"

# 7. DraftIssue は番号を持たないので number 無しでも壊れ扱いしない
python3 -I - "$SNAP" "$NOW" <<'PY'
import json, sys
items = [{"id": "PVTI_d", "type": "DraftIssue", "status": "Todo", "labels": [], "assignees": []}]
json.dump({"version": 1, "fetched_at": float(sys.argv[2]) - 10, "items": items}, open(sys.argv[1], "w"))
PY
run_read
check "draft issue without number is ok" test "$rc" -eq 0

# 8. 所有者が自分でない書き出しは broken（detail: foreign-owner）にする。
# 共有の置き場では別の uid の利用者が見張りより先に書き出しを置け、
# sticky bit で見張りの置き換えが効かなくなる。所有者を確かめないと、
# 偽の列（着手の合図の Todo・停止の Backlog）を本物と読んでしまう。
# 比べる uid は PELWAY_BOARD_SNAPSHOT_EXPECT_UID で差し替える（テスト用）。
write_ok_snapshot $((NOW - 60))
export PELWAY_BOARD_SNAPSHOT_EXPECT_UID=99999
run_read
check "foreign-owner: exit nonzero" test "$rc" -ne 0
check "foreign-owner: reason is broken" test "$(jqv 'd["reason"]')" = "broken"
check "foreign-owner: detail is foreign-owner" test "$(jqv 'd.get("detail")')" = "foreign-owner"
check "foreign-owner: items not exposed" test "$(jqv '"items" not in d')" = "True"
own_uid="$(id -u)"
export PELWAY_BOARD_SNAPSHOT_EXPECT_UID="$own_uid"
run_read
check "owner match: exit 0" test "$rc" -eq 0
unset PELWAY_BOARD_SNAPSHOT_EXPECT_UID

# 9. 単独のサロゲートを含む文字列が書き出しにあっても、出力は JSON 1 件に保つ。
# Python の JSON 読み取りと文字列の型検査は通るが UTF-8 の標準出力には書けないため、
# 成功の出力の途中に失敗の JSON が続くと、出力が JSON 1 件にならない
python3 -I - "$SNAP" "$NOW" <<'PY'
import json, sys
items = [{"id": "PVTI_s", "type": "Issue", "number": 1, "status": "Todo\ud800",
          "labels": [], "assignees": []}]
json.dump({"version": 1, "fetched_at": float(sys.argv[2]) - 60, "items": items}, open(sys.argv[1], "w"))
PY
run_read
check "surrogate: stdout is a single valid json" python3 -c 'import json,sys; json.loads(open(sys.argv[1], encoding="utf-8").read())' "$ROOT/out.json"
check "surrogate: no traceback" test ! -s "$ROOT/err.txt"

printf '\n'
if ((${#failures[@]})); then
  printf 'failed: %s\n' "${failures[@]}" >&2
  exit 1
fi
printf 'all tests passed\n'
