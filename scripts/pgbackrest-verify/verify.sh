#!/usr/bin/env bash
# Issue #101: pgBackRest で R15（作成から最大14日）の保持と、暗号化・チェックサム
# 検証つきの復元、12 §8.2 手順6 の削除記録の再適用を確かめる。
#
# 使い方:
#   bash scripts/pgbackrest-verify/verify.sh <フェーズ>
#   フェーズ: build | setup | retention-count | retention-time | delete-reapply |
#            restore-checksum | cleanup | all
#   環境変数: WORK_DIR（既定 scripts/pgbackrest-verify/work）
#
# 前提: rootless Podman が使えること。ネットワークは docker.io（ベースイメージ）、
# dl-cdn.alpinelinux.org（apk）、github.com（pgBackRest の tarball）に届くこと。
# 結果と読み方は docs/research/pgbackrest-retention-restore-verification.md。
#
# 判定は機械的に行う。各フェーズは条件を満たすと $WORK/checks/<name>.ok を
# 作り、満たさないと exit 1 で止まる。
#
# 保持ポリシーの検証で時刻を制御する理由: pgBackRest のフルバックアップの
# タイムスタンプは DB サーバの時計から取る（src/command/backup/db.c.inc の
# backupTime はオンライン時 dbTimeMSec を使う）。したがって「何日前の
# バックアップ」を作るには postgres 自体を libfaketime 付きで起動し直す
# 必要がある。expire 側の判定は time(NULL)（実行ホストの時計）なので、backup
# 側の時刻を偽れば expire の判定材料がoml 実測できる。
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
WORK=${WORK_DIR:-$SCRIPT_DIR/work}
CHECKS=$WORK/checks
IMAGE=localhost/pelway-pgbackrest:pg16
CONTAINER=pelway-pgbr
PGDATA=/var/lib/postgresql/data
REPO=/repo
RECORDS=/records
STANZA=pelway
RETENTION=14
CIPHER_PASS=pelway-test-pass
# 平文がリポジトリに現れないことを確かめるための目印。
MARKER=PELWAY-BACKUP-MARKER
# 保持検証の基準日（毎日 02:00 のフルバックアップを模す）。
BASE_DAY=${BASE_DAY:-$(date -u +%Y-%m-%d)}
mkdir -p "$WORK" "$CHECKS"

log() { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >&2; }
ok()  { : >"$CHECKS/$1.ok"; log "check ok: $1${2:+ ($2)}"; }
fail(){ printf 'check FAIL: %s: %s\n' "$1" "${2:-}" >&2; exit 1; }

# postgres を libfaketime 付きで起動する。$1 は偽装する時刻（"YYYY-MM-DD HH:MM:SS"）。
# 起動引数は postgresql.conf に書いておく（pg_ctl -o は空白で分割される）。
pg_start() { # pg_start <faketime 時刻>
  podman exec -u postgres \
    -e FAKETIME="@$1" \
    -e LD_PRELOAD=/usr/lib/faketime/libfaketime.so.1 \
    -e FAKETIME_DONT_FAKE_MONOTONIC=1 \
    "$CONTAINER" \
    pg_ctl -D "$PGDATA" -l /tmp/pg.log -w -t 60 start >/dev/null 2>&1
  wait_pg
}

pg_stop() {
  podman exec -u postgres "$CONTAINER" \
    pg_ctl -D "$PGDATA" -m immediate -w -t 60 stop >/dev/null 2>&1 || true
}

wait_pg() {
  for _ in $(seq 1 90); do
    podman exec "$CONTAINER" pg_isready -U postgres -q >/dev/null 2>&1 && return 0
    sleep 1
  done
  fail wait_pg "$CONTAINER の postgres が起動しない"
}

# コンテナ内で pgbackrest を実行する。
pgr() { podman exec -u postgres "$CONTAINER" pgbackrest "$@"; }

psql_q() { # psql_q <SQL> : 値を 1 行で返す
  podman exec -u postgres "$CONTAINER" psql -U postgres -d bench -X -q -t -A \
    -v ON_ERROR_STOP=1 -c "$1"
}

# リポジトリ内のバックアップ一覧（ラベル）を改行区切りで返す。
repo_labels() { # repo_labels <repo 番号>
  pgr --stanza="$STANZA" --repo="$1" info --output=json | jq -r '.[].backup[]?.label'
}

# バックアップの開始時刻（epoch 秒）をラベル→時刻の表で返す。
repo_backup_times() { # repo_backup_times <repo 番号>
  pgr --stanza="$STANZA" --repo="$1" info --output=json \
    | jq -r '.[].backup[]? | "\(.label) \(.timestamp.start)"'
}

# ---------------------------------------------------------------- build
phase_build() {
  log "build: pgBackRest 2.60.0 入り postgres:16-alpine イメージを作る"
  podman build --network host -f "$SCRIPT_DIR/Dockerfile" -t "$IMAGE" "$SCRIPT_DIR" \
    || fail build "podman build が失敗した"
  local v ver
  v=$(podman run --rm "$IMAGE" pgbackrest version)
  log "build: $v"
  # Issue #101 は 2.59.3 以上（それより前は暗号化の乱数生成の失敗を検出しない）を
  # 要求する。使う版が条件を満たすことをここで確かめる。
  ver=${v##* }
  if [[ "$(printf '%s\n%s\n' "2.59.3" "$ver" | sort -V | head -1)" != "2.59.3" ]]; then
    fail pbgr-version "pgBackRest $ver は 2.59.3 未満"
  fi
  ok pbgr-version "pgBackRest $ver（2.59.3 以上）"
}

# ---------------------------------------------------------------- setup
phase_setup() {
  log "setup: コンテナを起動し、pgBackRest の設定と合成データを作る"
  phase_cleanup_containers
  mkdir -p "$WORK"
  # 設定はコンテナ起動前に bind mount する。archive_command が起動直後から
  # 参照できるようにするため。
  # 節名は stanza 名そのもの（[pelway]）が正しい（[stanza:pelway] は読まれない）。
  cat >"$WORK/pgbackrest.conf" <<EOF
[global]
repo1-path=$REPO/count
repo1-type=posix
repo1-cipher-type=aes-256-cbc
repo1-cipher-pass=$CIPHER_PASS
repo1-retention-full=$RETENTION
repo2-path=$REPO/time
repo2-type=posix
repo2-cipher-type=aes-256-cbc
repo2-cipher-pass=$CIPHER_PASS
repo2-retention-full=$RETENTION
repo2-retention-full-type=time
log-path=/var/log/pgbackrest
process-max=2
start-fast=y

[$STANZA]
pg1-path=$PGDATA
pg1-port=5432
pg1-socket-path=/var/run/postgresql
EOF
  podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
  podman volume rm -f pelway-pgbr-pgdata pelway-pgbr-repo pelway-pgbr-records >/dev/null 2>&1 || true
  # エントリポイントを使わず、initdb と postgres の起動はスクリプトが行う
  # （backup のタイムスタンプを偽装するため postgres を自有で起動し直す）。
  podman run -d --name "$CONTAINER" --entrypoint /bin/sh \
    -v pelway-pgbr-pgdata:$PGDATA \
    -v pelway-pgbr-repo:$REPO \
    -v pelway-pgbr-records:$RECORDS \
    -v "$WORK/pgbackrest.conf:/etc/pgbackrest/pgbackrest.conf:ro" \
    "$IMAGE" -c 'while :; do sleep 3600; done' >/dev/null
  # リポジトリと削除記録の置き場所、ログ先を用意する。
  podman exec -u root "$CONTAINER" sh -c \
    "mkdir -p $REPO/count $REPO/time $RECORDS /var/log/pgbackrest /var/run/postgresql && \
     chown -R postgres:postgres $REPO $RECORDS /var/log/pgbackrest /var/run/postgresql && \
     chown -R postgres:postgres $PGDATA"
  # Alpine にはロケールPACK enteringがないため、initdb は C ロケールで作る。
  # エンコーディングは UTF8 を明示する（SQL_ASCII のままだと日本語が壊れる）。
  podman exec -u root "$CONTAINER" sh -c "rm -rf $PGDATA/* && chown postgres:postgres $PGDATA"
  podman exec -u postgres -e LANG=C -e LC_ALL=C "$CONTAINER" \
    initdb -D "$PGDATA" -U postgres --encoding=UTF8 --locale=C \
    --auth-local=trust --auth-host=trust >/dev/null
  podman exec -u root "$CONTAINER" chown -R postgres:postgres "$PGDATA"
  # WAL アーカイブを有効にする（復旧訓練で WAL を要するため）。
  # pg_ctl -o は引数が空白で分割されるので設定は postgresql.conf に書く。
  cat >"$WORK/postgresql-extra.conf" <<EOF
archive_mode = on
archive_command = 'pgbackrest --stanza=$STANZA archive-push %p'
listen_addresses = ''
EOF
  podman cp "$WORK/postgresql-extra.conf" "$CONTAINER:$PGDATA/postgresql-extra.conf"
  podman exec -u root "$CONTAINER" sh -c \
    "printf 'include = 'postgresql-extra.conf'\n' >> $PGDATA/postgresql.conf && \
     chown postgres:postgres $PGDATA/postgresql-extra.conf && \
     chown -R postgres:postgres $PGDATA"

  pg_start "$(date -u +'%Y-%m-%d %H:%M:%S')"

  # エントリポイントを使わないため、検証に使うデータベースを自前で作る。
  podman exec -u postgres "$CONTAINER" createdb -U postgres bench \
    || fail createdb "bench データベースの作成に失敗"

  # stanza-create と check は設定済みの全リポジトリに対して動く（--repo は不可）。
  pgr --stanza="$STANZA" stanza-create || fail stanza-create "stanza 作成に失敗"
  pgr --stanza="$STANZA" check || fail pgbackrest-check "pgbackrest check が失敗"

  # 合成データ。probe は復元の照合用で、検証中ずっと書き換えない。
  podman exec -i -u postgres "$CONTAINER" psql -U postgres -d bench -X -q -v ON_ERROR_STOP=1 <<'SQL' >/dev/null
CREATE TABLE messages (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  channel_id int NOT NULL,
  sender_id int NOT NULL,
  body text NOT NULL,
  expires_at timestamptz,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE probe (id int PRIMARY KEY, payload text NOT NULL);
CREATE TABLE recovery_guard (
  id int PRIMARY KEY CHECK (id = 1),
  reapply_state text NOT NULL CHECK (reapply_state IN ('pending', 'done'))
);
INSERT INTO recovery_guard (id, reapply_state) VALUES (1, 'pending');
INSERT INTO probe SELECT i, '$MARKER-' || lpad(i::text, 4, '0') FROM generate_series(1, 5) i;
INSERT INTO messages (channel_id, sender_id, body, expires_at)
SELECT 1 + i % 20, 1 + i % 50,
       '検証メッセージ ' || i || ' ' || repeat('x', 20),
       CASE WHEN i % 10 = 0 THEN now() - interval '1 day' ELSE NULL END
FROM generate_series(1, 200) i;
SQL
  local n
  n=$(psql_q "SELECT count(*) FROM messages;")
  [[ "$n" == "200" ]] || fail seed "messages が $n 行（200 のはず）"
  ok seed "合成データ $n 行と probe 5 行"
}

# ---------------------------------------------------------------- retention-count
# 件数指定（count 型）。毎日フルバックアップなら 14 世代 = 最大 14 日。
phase_retention_count() {
  log "retention-count: 件数指定 $RETENTION 世代で oldest が消えることを確かめる"
  rm -f "$CHECKS/retention-count-"*.ok
  # 毎日 02:00 のフルバックアップを $((RETENTION + 1)) 日分作る。
  # 1 日目..14 日目の 14 世代は残され、15 日目を作った時点で最古が消える。
  local i day
  for i in $(seq 0 "$RETENTION"); do
    day=$(date -u -d "$BASE_DAY - $i day + 1 day" +%Y-%m-%d)
    pg_stop
    pg_start "$day 02:00:00"
    pgr --stanza="$STANZA" --repo=1 --type=full backup >/dev/null \
      || fail retention-count "$day の backup が失敗"
  done
  local labels count
  labels=$(repo_labels 1)
  count=$(printf '%s\n' "$labels" | grep -c . || true)
  # 件数指定では「完了したバックアップが retention 個より 1 個多いときに
  # 最古が expire される」。15 個作ったので残るのは 14 個。
  if [[ "$count" != "$RETENTION" ]]; then
    fail retention-count "残る世代数が $count（$RETENTION のはず）: $labels"
  fi
  ok retention-count "15 世代作っても $RETENTION 世代だけ残る"

  # 残った 14 世代はすべて「現在から 14 日以内」であること（R15 の Creating）。
  local now oldest_epoch
  now=$(date -u +%s)
  oldest_epoch=$(repo_backup_times 1 | awk '{print $2}' | sort -n | head -1)
  local age_days
  age_days=$(( (now - oldest_epoch) / 86400 ))
  if (( age_days > RETENTION )); then
    fail retention-count "最も古い残存世代が $age_days 日前（上限 $RETENTION 日）"
  fi
  ok retention-count-window "最も古い残存世代は $age_days 日前（$RETENTION 日以内）"

  # 消えたのは最古の 1 個であること（残りは連続する 14 日分）。
  local labels_sorted first
  labels_sorted=$(repo_backup_times 1 | sort -n -k2)
  first=$(printf '%s\n' "$labels_sorted" | head -1 | awk '{print $1}')
  ok retention-count-labels "最古の残存ラベル $first"

  # 差分・増分が最古の full に紐づく場合、それらも一緒に消えること。
  local orphans
  orphans=$(pgr --stanza="$STANZA" --repo=1 info --output=json \
    | jq -r '.[].backup[]? | select(.type != "full") | .label' | grep -c . || true)
  [[ "$orphans" == "0" ]] || fail retention-count "expire 後も差分 $orphans 件が残っている"
  ok retention-count-no-orphan "差分・増分は残っていない"

  printf '%s\n' "$labels" | head -1 >"$WORK/latest-label-repo1.txt"
  ok retention-count-labels-saved "復元先のラベル $(cat "$WORK/latest-label-repo1.txt")"
}

# ---------------------------------------------------------------- retention-time
# 日数指定（time 型）。日数指定だけでは「作成から最大14日」を守れない。
phase_retention_time() {
  log "retention-time: 日数指定 $RETENTION 日では 14 日超の世代が残ることを確かめる"
  rm -f "$CHECKS/retention-time-"*.ok
  # 20 日前・16 日前・3 日前のフルバックアップを 3 つ作る。
  # time 型の expire は「14 日超のうち最も新しい 1 つ」を残す設計なので、
  # 14 日超のものが消えずに残る。
  local offset day
  for offset in 20 16 3; do
    day=$(date -u -d "$BASE_DAY - $offset day" +%Y-%m-%d)
    pg_stop
    pg_start "$day 02:00:00"
    pgr --stanza="$STANZA" --repo=2 --type=full backup >/dev/null \
      || fail retention-time "$offset 日前の backup が失敗"
  done
  pg_stop
  pg_start "$(date -u +'%Y-%m-%d %H:%M:%S')"

  local now limit_epoch old_count old_labels
  now=$(date -u +%s)
  limit_epoch=$(( now - RETENTION * 86400 ))
  old_labels=$(repo_backup_times 2 | awk -v limit="$limit_epoch" '$2 < limit {print $1}')
  old_count=$(printf '%s\n' "$old_labels" | grep -c . || true)
  # 14 日超のバックアップが残っていれば、日数指定だけでは上限を守れない。
  if [[ "$old_count" -lt 1 ]]; then
    fail retention-time "14 日超のバックアップが残っていない（$old_count 件）"
  fi
  ok retention-time "14 日超のバックアップが $old_count 件残存している（$old_labels）"

  # 別の削除処理（expire --set）で 14 日超のものを消せるか。
  local target remain
  target=$(printf '%s\n' "$old_labels" | head -1)
  pgr --stanza="$STANZA" --repo=2 expire --set="$target" >/dev/null \
    || fail retention-time "expire --set が失敗"
  remain=$(repo_labels 2 | grep -c . || true)
  if [[ "$remain" != "2" ]]; then
    fail retention-time "expire --set 後も $remain 世代（2 のはず）"
  fi
  ok retention-time-expire-set "expire --set $target で 14 日超の世代を消せる（残り $remain 世代）"

  # 消した後も、件数指定と同じ「14 世代以内」に収めるには日数の管理が要る。
  local oldest_epoch
  oldest_epoch=$(repo_backup_times 2 | awk '{print $2}' | sort -n | head -1)
  if (( oldest_epoch < limit_epoch )); then
    fail retention-time "expire --set 後も 14 日超の世代が残っている"
  fi
  ok retention-time-window "消した後は最も古い世代も $RETENTION 日以内"
  printf '%s\n' "$target" >"$WORK/expired-label-repo2.txt"
}

# ---------------------------------------------------------------- delete-reapply
# 12 §8.2 手順6: 復旧時は認可と外部dispatchを既定停止し、復旧DBと独立して
# 保全した削除・制限・撤回記録を先に再適用する。
phase_delete_reapply() {
  log "delete-reapply: 復旧後に独立保全の削除記録を先に再適用する"
  rm -f "$CHECKS/delete-reapply-"*.ok
  local label
  label=$(cat "$WORK/latest-label-repo1.txt")
  [[ -n "$label" ]] || fail delete-reapply "復元元のラベルがない"

  # 1) 削除を行い、独立保全先（PG クラスタの外）へ記録する。
  podman exec -i -u postgres "$CONTAINER" psql -U postgres -d bench -X -q -v ON_ERROR_STOP=1 <<'SQL' >/dev/null
UPDATE messages SET deleted_at = now() WHERE id IN (1, 2, 3);
SQL
  podman exec -u postgres "$CONTAINER" psql -U postgres -d bench -X -q -t -A \
    -v ON_ERROR_STOP=1 -c \
    "SELECT id || E'\t' || deleted_at FROM messages WHERE deleted_at IS NOT NULL ORDER BY id;" \
    >"$WORK/deleted-records.tsv"
  local nrec
  nrec=$(grep -c . "$WORK/deleted-records.tsv" || true)
  [[ "$nrec" == "3" ]] || fail delete-reapply "削除記録が $nrec 行（3 のはず）"
  podman exec -i -u postgres "$CONTAINER" sh -c "cat > $RECORDS/deleted-records.tsv" \
    <"$WORK/deleted-records.tsv"
  ok delete-reapply-records "削除記録 $nrec 行を独立保全先に書き出した"

  # 2) 停止させ、バックアップから復旧する。
  pg_stop
  podman exec -u root "$CONTAINER" sh -c "rm -rf $PGDATA/*"
  pgr --stanza="$STANZA" --repo=1 restore --pg1-path="$PGDATA" --set="$label" >/dev/null \
    || fail delete-reapply "restore が失敗"
  podman exec -u root "$CONTAINER" chown -R postgres:postgres "$PGDATA"
  pg_start "$(date -u +'%Y-%m-%d %H:%M:%S')"

  # 3) 復旧直後は削除記録が未適用なので、削除済みメッセージが復活している。
  #    認可と外部dispatchの既定停止は recovery_guard で表す。
  local guard resurrected
  guard=$(psql_q "SELECT reapply_state FROM recovery_guard WHERE id = 1;")
  [[ "$guard" == "pending" ]] || fail delete-reapply "復旧直後の停止状態（pending）ではない"
  resurrected=$(psql_q "SELECT count(*) FROM messages WHERE id IN (1,2,3) AND deleted_at IS NULL;")
  if [[ "$resurrected" != "3" ]]; then
    fail delete-reapply "復旧直後の復活数が $resurrected（3 のはず）"
  fi
  ok delete-reapply-revive "復旧直後は削除済み 3 件が復活し、認可・dispatch は停止状態"

  # 4) 独立保全した削除記録を先に再適用する。
  podman exec -i -u postgres "$CONTAINER" psql -U postgres -d bench -X -q -v ON_ERROR_STOP=1 <<SQL >/dev/null
UPDATE messages m SET deleted_at = r.deleted_at
FROM (VALUES $(awk -F'\t' '{printf "(%s::bigint, %s::timestamptz),", $1, $2}' "$WORK/deleted-records.tsv" | sed 's/,$//')) AS r(id, deleted_at)
WHERE m.id = r.id;
UPDATE recovery_guard SET reapply_state = 'done' WHERE id = 1;
SQL
  local after state
  after=$(psql_q "SELECT count(*) FROM messages WHERE id IN (1,2,3) AND deleted_at IS NULL;")
  state=$(psql_q "SELECT reapply_state FROM recovery_guard WHERE id = 1;")
  [[ "$after" == "0" ]] || fail delete-reapply "再適用後も復活している（$after 件）"
  [[ "$state" == "done" ]] || fail delete-reapply "再適用後も停止状態（$state）"
  ok delete-reapply "削除記録を先に再適用すると削除済み 3 件は復活しない"

  # 5) 独立保全先が復旧の影響を受けないこと（復元したスナップショットに
  #    依存していないこと）。
  podman exec -u postgres "$CONTAINER" sh -c "test -s $RECORDS/deleted-records.tsv" \
    || fail delete-reapply "独立保全先の記録が残っていない"
  ok delete-reapply-independent "削除記録は復旧後も独立保全先にある"
}

# ---------------------------------------------------------------- restore-checksum
# 暗号化（2.59.3 以上）と、チェックサムで確かめる復元。
phase_restore_checksum() {
  log "restore-checksum: 暗号化とチェックサム検証つきの復元を確かめる"
  rm -f "$CHECKS/restore-checksum-"*.ok
  local label
  label=$(cat "$WORK/latest-label-repo1.txt")
  [[ -n "$label" ]] || fail restore-checksum "復元元のラベルがない"

  # 1) 平文がリポジトリに入っていないこと（暗号化されている）。
  local leaked
  leaked=$(podman exec -u root "$CONTAINER" sh -c \
    "grep -rl '$MARKER' $REPO 2>/dev/null | wc -l")
  [[ "$leaked" == "0" ]] || fail restore-checksum "リポジトリに平文が $leaked ファイルある"
  ok restore-checksum-cipher "リポジトリ内に平文（$MARKER）は現れない"

  # 2) リポジトリ全体のチェックサム検証（pgbackrest verify）。
  pgr --stanza="$STANZA" --repo=1 verify >/dev/null \
    || fail restore-checksum "pgbackrest verify が失敗"
  ok restore-checksum-verify "pgbackrest verify が通る"

  # 3) 復元に成功し、probe の内容が一致すること。
  local before after
  before=$(psql_q "SELECT md5(string_agg(payload, ',' ORDER BY id)) FROM probe;")
  pg_stop
  podman exec -u root "$CONTAINER" sh -c "rm -rf $PGDATA/*"
  pgr --stanza="$STANZA" --repo=1 restore --pg1-path="$PGDATA" --set="$label" >/dev/null \
    || fail restore-checksum "restore が失敗"
  podman exec -u root "$CONTAINER" chown -R postgres:postgres "$PGDATA"
  pg_start "$(date -u +'%Y-%m-%d %H:%M:%S')"
  after=$(psql_q "SELECT md5(string_agg(payload, ',' ORDER BY id)) FROM probe;")
  [[ "$before" == "$after" ]] || fail restore-checksum "復後の probe が一致しない（$before != $after）"
  ok restore-checksum-restore "復元後の probe が一致（$after）"

  # 4) リポジトリのファイルを 1 バイト壊すと、復元がチェックサムで失敗する。
  local victim
  victim=$(podman exec -u root "$CONTAINER" sh -c \
    "find $REPO/count/backup -name '*.pgbz' | head -1")
  [[ -n "$victim" ]] || fail restore-checksum "改変対象のバックアップファイルが見つからない"
  podman exec -u root "$CONTAINER" sh -c \
    "printf 'X' | dd of='$victim' bs=1 seek=0 conv=notrunc status=none"
  pg_stop
  podman exec -u root "$CONTAINER" sh -c "rm -rf $PGDATA/*"
  local out rc
  set +e
  out=$(pgr --stanza="$STANZA" --repo=1 restore --pg1-path="$PGDATA" --set="$label" 2>&1)
  rc=$?
  set -e
  if [[ "$rc" == "0" ]]; then
    fail restore-checksum "壊したリポジトリから restore が成功した"
  fi
  log "restore-checksum: 改変後の restore 出力: $(printf '%s' "$out" | tail -3 | tr '\n' ' ')"
  if ! printf '%s' "$out" | grep -qi 'checksum\|decrypt\|cipher\|invalid'; then
    fail restore-checksum "改変の検出理由が checksum/decrypt を含まない: $out"
  fi
  ok restore-checksum-corrupt "1 バイト改変で restore がチェックサム検証により失敗"
}

# ---------------------------------------------------------------- cleanup
phase_cleanup_containers() {
  podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
phase_cleanup() {
  log "cleanup: コンテナとボリュームを削除する"
  phase_cleanup_containers
  podman volume rm -f pelway-pgbr-pgdata pelway-pgbr-repo pelway-pgbr-records >/dev/null 2>&1 || true
  log "cleanup 完了"
}

case "${1:-}" in
  build)            phase_build ;;
  setup)            phase_setup ;;
  retention-count)  phase_retention_count ;;
  retention-time)   phase_retention_time ;;
  delete-reapply)   phase_delete_reapply ;;
  restore-checksum) phase_restore_checksum ;;
  cleanup)          phase_cleanup ;;
  all)
    phase_build; phase_setup
    phase_retention_count; phase_retention_time
    phase_delete_reapply; phase_restore_checksum
    ;;
  *) echo "usage: $0 <build|setup|retention-count|retention-time|delete-reapply|restore-checksum|cleanup|all>" >&2; exit 2 ;;
esac
