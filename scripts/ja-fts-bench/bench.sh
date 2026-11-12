#!/usr/bin/env bash
# Issue #93: PGroonga と pg_bigm を日本語の合成チャットデータで比べる。
#
# 使い方:
#   bash scripts/ja-fts-bench/bench.sh <フェーズ>
#   フェーズ: build | setup | quality | speed | acl | backup | partial | verify | cleanup | all
#   環境変数: N_MESSAGES（既定 300000）、WORK_DIR（既定 scripts/ja-fts-bench/work）
#
# 前提: rootless Podman が使えること。ネットワークは docker.io・
# dl-cdn.alpinelinux.org・packages.groonga.org・github.com に届くこと
#（イメージビルドで使う）。計測結果の読み方は
# docs/research/ja-fts-pgroonga-pg-bigm-benchmark.md。
#
# 検索の「正解」は strpos による生の部分一致（ワイルドカード解釈なし）で
# alive な行について機械定義する。pg_bigm の likequery は _ などを
# エスケープした上で LIKE 検索するので、正解定義と一致する。
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
WORK=${WORK_DIR:-$SCRIPT_DIR/work}
N_MESSAGES=${N_MESSAGES:-300000}
N_WARM=10000
PORT_BIGM=55433
PORT_GRN=55434
PGDATA_VOL_BIGM=ja-fts-bigm-pgdata
PGDATA_VOL_GRN=ja-fts-groonga-pgdata
CHECKS=$WORK/checks
mkdir -p "$WORK" "$CHECKS"

# alive = 削除されておらず保存期限も切れていない（13 の R03/D01 相当）。
ALIVE='m.deleted_at IS NULL AND (m.expires_at IS NULL OR m.expires_at > now())'

log() { printf '[%s] %s\n' "$(date +%H:%M:%S)" "$*" >&2; }
ok()  { : >"$CHECKS/$1.ok"; log "check ok: $1${2:+ ($2)}"; }
fail(){ printf 'check FAIL: %s: %s\n' "$1" "${2:-}" >&2; exit 1; }

c_name() { [[ $1 == bigm ]] && echo ja-fts-bigm || echo ja-fts-groonga; }
img_name() { [[ $1 == bigm ]] && echo localhost/ja-fts-pgbigm:pg16 || echo localhost/ja-fts-pgroonga:pg16; }
psql_e() { # psql_e <engine> [psql args...]
  local e=$1; shift
  podman exec -i "$(c_name "$e")" psql -U postgres -d bench -X -q -v ON_ERROR_STOP=1 "$@"
}
wait_pg() { # wait_pg <コンテナ名>
  for _ in $(seq 1 90); do
    podman exec "$1" pg_isready -U postgres -d bench -q && return 0
    sleep 1
  done
  fail wait_pg "$1 が起動しない"
}
match_sql() { # match_sql <engine> <query-literal or eq col>
  if [[ $1 == groonga ]]; then echo "m.body &@~ $2"; else echo "m.body LIKE likequery($2)"; fi
}

# ---------------------------------------------------------------- build
phase_build() {
  log "build: pg_bigm / PGroonga イメージを rootless podman でビルド"
  # --network host は rootless ネットワーク経由だと ruby の OpenSSL 接続が
  # packages.groonga.org へタイムアウトするため（download_mecab.rb）。
  podman build --network host -f "$SCRIPT_DIR/Dockerfile.pgbigm" \
    -t localhost/ja-fts-pgbigm:pg16 "$SCRIPT_DIR"
  podman build --network host -f "$SCRIPT_DIR/Dockerfile.pgroonga" \
    -t localhost/ja-fts-pgroonga:pg16 "$SCRIPT_DIR"
}

# ---------------------------------------------------------------- 共通 SQL
schema_sql() { # schema_sql <拡張名> … スキーマと合成データ（両エンジン共通）
  cat <<SQL
SELECT setseed(0.42);
CREATE EXTENSION $1;
CREATE TABLE messages (
  id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  channel_id int NOT NULL,
  sender_id int NOT NULL,
  body text NOT NULL,
  expires_at timestamptz,
  deleted_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE TABLE channel_members (
  channel_id int NOT NULL,
  member_id int NOT NULL,
  PRIMARY KEY (channel_id, member_id)
);
-- halfkata（半角カナ）・fullprod（全角英字）は表記ゆれ評価用。
-- jp 配列末尾 3 語（進捙・資科・確r認）は打ち間違い。
CREATE TEMP TABLE vocab (
  jp text[], kata text[], prod text[], code text[], en text[],
  halfkata text[], fullprod text[]
);
INSERT INTO vocab VALUES (
  ARRAY['会議','資料','確認','進捗','不具合','見積','仕様','実装','試験','レビュー',
        '本番','障害','対応','設計','移行','改修','承認','週次','月次','報告',
        '相談','決定','保留','期限','設定','環境','構築','更新','追加','削除',
        '復元','保存','検索','権限','進捙','資科','確r認'],
  ARRAY['スケジュール','フィードバック','アジャイル','コンプライアンス','ポータビリティ',
        'デプロイ','モジュール','マイルストーン','バックログ','スプリント',
        'サポート','セキュリティ','パフォーマンス','キャパシティ','ロードマップ'],
  ARRAY['PostgreSQL','Podman','Pelway','Groonga','PGroonga','pg_bigm','Rust','sqlx',
        'GitHub','Slack','Apple','Linux','Windows','Chrome','Firefox'],
  ARRAY['cargo clippy --locked','git switch -c','pgroonga.enable_wal','SELECT * FROM',
        'mise run verify','docker compose up','curl -sSL','gh pr create --draft'],
  ARRAY['The release is scheduled for Friday.','please review the PR','deployment failed',
        'see you tomorrow','looks good to me','needs more tests'],
  ARRAY['ｽｹｼﾞｭｰﾙ','ﾌｨｰﾄﾞﾊﾞｯｸ','ｱｼﾞｬｲﾙ','ｺﾝﾌﾟﾗｲｱﾝｽ','ﾃﾞﾌﾟﾛｲ'],
  ARRAY['ｐｏｓｔｇｒｅｓ','ｐｏｄｍａﾝ','ｇｉｔｈｕｂ']
);
INSERT INTO channel_members
SELECT c, m FROM generate_series(1,50) c, generate_series(1,200) m WHERE random() < 0.3;
-- 本文テンプレート 11 種。語彙選択は i の素数ステップ剰余（決定的）。
-- 期限切れ 10%・削除済み 5%。テンプレート 1 の 25% は半角カナへ、
-- テンプレート 2 の 25% は全角英字へ置き換える（表記ゆれの正解行）。
INSERT INTO messages (channel_id, sender_id, body, expires_at, deleted_at, created_at)
SELECT 1 + i % 50, 1 + i % 200,
  CASE i % 11
    WHEN 0 THEN jp[1+(i*7)%37]  || 'の' || jp[1+(i*13)%37] || 'について'
                || jp[1+(i*29)%37] || 'をお願いします'
    WHEN 1 THEN (CASE WHEN random() < 0.25 THEN halfkata[1+(i*7)%5]
                      ELSE kata[1+(i*11)%15] END)
                || 'の' || jp[1+(i*7)%37] || 'は' || jp[1+(i*13)%37] || 'が必要です'
    WHEN 2 THEN (CASE WHEN random() < 0.25 THEN fullprod[1+(i*7)%3]
                      ELSE prod[1+(i*11)%15] END)
                || 'の' || jp[1+(i*7)%37] || 'で' || jp[1+(i*13)%37] || 'が発生しました'
    WHEN 3 THEN '明日の' || kata[1+(i*11)%15] || 'で' || jp[1+(i*7)%37] || 'を確認します'
    WHEN 4 THEN code[1+(i*7)%8] || ' が失敗するので' || jp[1+(i*13)%37] || 'を見てください'
    WHEN 5 THEN jp[1+(i*7)%37] || 'は' || jp[1+(i*13)%37] || '済みです。' || en[1+(i*11)%6]
    WHEN 6 THEN kata[1+(i*11)%15] || 'について' || prod[1+(i*7)%15] || 'の'
                || jp[1+(i*13)%37] || 'を参考にしてください'
    WHEN 7 THEN jp[1+(i*7)%37] || jp[1+(i*13)%37] || 'の' || jp[1+(i*29)%37] || 'は'
                || prod[1+(i*11)%15] || 'では' || jp[1+(i*17)%37] || '扱いです'
    WHEN 8 THEN jp[1+(i*7)%37] || 'の' || code[1+(i*11)%8] || '、' || en[1+(i*13)%6]
    WHEN 9 THEN prod[1+(i*7)%15] || 'と' || prod[1+(i*13)%15] || 'の'
                || jp[1+(i*29)%37] || 'を比較しました。' || en[1+(i*11)%6]
    ELSE jp[1+(i*7)%37] || 'について' || kata[1+(i*11)%15] || 'の観点で'
                || jp[1+(i*13)%37] || 'を' || jp[1+(i*29)%37] || 'します'
  END,
  CASE WHEN random() < 0.10 THEN now() - interval '1 day' ELSE NULL END,
  CASE WHEN random() < 0.05 THEN now() ELSE NULL END,
  now() - (random() * interval '180 days')
FROM generate_series(1, $N_MESSAGES) AS i,
     (SELECT jp, kata, prod, code, en, halfkata, fullprod FROM vocab) AS v;
ANALYZE;
SQL
}

# 評価クエリ。needle が「部分一致の正解」を定義する生文字列。
# kind: jp=漢字 / kata=カタカナ / prod=製品名 / code=コード / typo=打ち間違い / norm=表記ゆれ
# （norm は query=正規形、needle=ゆれ表記）
eval_queries() {
  cat <<'EOF'
資料|資料|資料|jp
会議|会議|会議|jp
進捗|進捗|進捗|jp
不具合|不具合|不具合|jp
確認|確認|確認|jp
パフォーマンス|パフォーマンス|パフォーマンス|kata
スケジュール|スケジュール|スケジュール|kata
フィードバック|フィードバック|フィードバック|kata
コンプライアンス|コンプライアンス|コンプライアンス|kata
PostgreSQL|PostgreSQL|PostgreSQL|prod
postgres小文字|postgres|postgres|prod
pgroonga_wal|pgroonga.enable_wal|pgroonga.enable_wal|code
cargo_clippy|cargo clippy|cargo clippy|code
進捙誤字|進捙|進捙|typo
スケジュール正規形|スケジュール|ｽｹｼﾞｭｰﾙ|norm
postgres正規形|postgres|ｐｏｓｔｇｒｅｓ|norm
pp部分一致|pp|pp|partial
EOF
}

load_eval_queries() { # 評価クエリを両エンジンへ一括投入
  local sql="CREATE TABLE IF NOT EXISTS eval_queries (label text, query text, needle text, kind text); TRUNCATE eval_queries;"
  while IFS='|' read -r label query needle kind; do
    sql+=$'\n'"INSERT INTO eval_queries VALUES ('$label', '$query', '$needle', '$kind');"
  done < <(eval_queries)
  for e in bigm groonga; do
    psql_e "$e" -c "$sql" >/dev/null
  done
}

# ---------------------------------------------------------------- setup
phase_setup() {
  log "setup: コンテナ起動・拡張・合成データ ${N_MESSAGES} 件・索引"
  phase_cleanup_containers
  start_container ja-fts-bigm localhost/ja-fts-pgbigm:pg16 "$PORT_BIGM" "$PGDATA_VOL_BIGM"
  start_container ja-fts-groonga localhost/ja-fts-pgroonga:pg16 "$PORT_GRN" "$PGDATA_VOL_GRN"
  wait_pg ja-fts-bigm; wait_pg ja-fts-groonga

  for e in bigm groonga; do
    local ext=pg_bigm create_idx
    [[ $e == groonga ]] && ext=pgroonga
    [[ $e == groonga ]] && create_idx="CREATE INDEX ON messages USING pgroonga (body); ANALYZE messages;" \
                        || create_idx="CREATE INDEX ON messages USING gin (body gin_bigm_ops); ANALYZE messages;"
    schema_sql "$ext" >"$WORK/schema-$e.sql"
    local t0 t1
    t0=$(date +%s)
    psql_e "$e" -f /dev/stdin <"$WORK/schema-$e.sql" >"$WORK/load-$e.log"
    t1=$(date +%s); echo "load_seconds=$((t1-t0))" >>"$WORK/load-$e.log"
    t0=$(date +%s)
    psql_e "$e" -c "$create_idx"
    t1=$(date +%s); echo "index_seconds=$((t1-t0))" >>"$WORK/load-$e.log"
    log "$e: 投入 $((t1-t0)) 秒（CREATE INDEX 含む。内訳は load-$e.log）"
  done
  ok setup "両エンジンのデータ投入と索引作成が完了"
}

start_container() { # start_container <名前> <イメージ> <ポート> <ボリューム>
  podman run -d --name "$1" -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=bench \
    -e PGDATA=/var/lib/postgresql/data/pgdata \
    -p "127.0.0.1:$3:5432" -v "$4:/var/lib/postgresql/data:Z" "$2" >/dev/null
}
phase_cleanup_containers() {
  for c in ja-fts-bigm-restore ja-fts-groonga-restore ja-fts-bigm ja-fts-groonga; do
    podman rm -f "$c" >/dev/null 2>&1 || true
  done
  podman volume rm -f "$PGDATA_VOL_BIGM" "$PGDATA_VOL_GRN" >/dev/null 2>&1 || true
}

# ---------------------------------------------------------------- quality
phase_quality() {
  log "quality: 再現率・追加ヒット・上位10件の品質"
  load_eval_queries
  {
    echo "engine|label|kind|n_answer|n_hit|n_both"
    for e in bigm groonga; do
      psql_e "$e" -A -t -F'|' <<SQL
SELECT '$e', eq.label, eq.kind,
  (SELECT count(*) FROM messages m WHERE $ALIVE AND strpos(m.body, eq.needle) > 0),
  (SELECT count(*) FROM messages m WHERE $ALIVE AND $(match_sql "$e" 'eq.query')),
  (SELECT count(*) FROM messages m WHERE $ALIVE AND strpos(m.body, eq.needle) > 0
     AND $(match_sql "$e" 'eq.query'))
FROM eval_queries eq ORDER BY eq.label;
SQL
    done
  } >"$WORK/quality.tsv"
  # 上位10件: PGroonga は pgroonga_score 順、pg_bigm は順位を持たないため id 順。
  {
    echo "engine|label|top10|wrong_in_top10"
    psql_e bigm -A -t -F'|' <<'SQL'
SELECT 'bigm', eq.label, t.n, t.w FROM eval_queries eq,
 LATERAL (SELECT count(*) AS n,
                 count(*) FILTER (WHERE strpos(m.body, eq.needle) = 0) AS w
          FROM (SELECT id FROM messages m WHERE m.deleted_at IS NULL
                  AND (m.expires_at IS NULL OR m.expires_at > now())
                  AND m.body LIKE likequery(eq.query) ORDER BY id LIMIT 10) top
          JOIN messages m USING (id)) t ORDER BY eq.label;
SQL
    psql_e groonga -A -t -F'|' <<'SQL'
SELECT 'groonga', eq.label, t.n, t.w FROM eval_queries eq,
 LATERAL (SELECT count(*) AS n,
                 count(*) FILTER (WHERE strpos(m.body, eq.needle) = 0) AS w
          FROM (SELECT id FROM messages m WHERE m.deleted_at IS NULL
                  AND (m.expires_at IS NULL OR m.expires_at > now())
                  AND m.body &@~ eq.query
                ORDER BY pgroonga_score(m) DESC, id LIMIT 10) top
          JOIN messages m USING (id)) t ORDER BY eq.label;
SQL
  } >"$WORK/quality-top10.tsv"
  ok quality "quality.tsv / quality-top10.tsv 生成"
}

# 検索速度と ACL 込み検索の共通測定: 3 回実行し件数と Execution Time の
# 中央値を "engine|label|n|median_ms" で出力する。acl_clause は空（alive だけ）
# または EXISTS 句（権限込み）。
timed_search() { # timed_search <engine> <label> <acl clause> <match式>
  local e=$1 label=$2 acl=$3 match=$4 times=() i out n=""
  for i in 1 2 3; do
    out=$(psql_e "$e" -A -t <<SQL
SELECT count(*) FROM messages m WHERE $ALIVE$acl AND $match;
EXPLAIN (ANALYZE, COSTS OFF, TIMING OFF)
SELECT count(*) FROM messages m WHERE $ALIVE$acl AND $match;
SQL
)
    n=$(echo "$out" | head -1)
    times+=("$(echo "$out" | awk '/Execution Time/{print $3}')")
  done
  printf '%s|%s|%s|%s\n' "$e" "$label" "$n" \
    "$(printf '%s\n' "${times[@]}" | sort -n | sed -n '2p')"
}

# ---------------------------------------------------------------- speed
phase_speed() {
  log "speed: 検索速度・索引サイズ・追記負荷"
  {
    echo "engine|label|n|median_ms"
    for e in bigm groonga; do
      while IFS='|' read -r label query _needle _kind; do
        timed_search "$e" "$label" "" "$(match_sql "$e" "'$query'")"
      done < <(eval_queries)
    done
  } >"$WORK/speed.tsv"

  # 索引サイズ。pg_bigm（GIN）は pg_relation_size で出る。PGroonga の索引は
  # PGDATA/base/<dboid>/pgrn.* が実体で pg_relation_size は 0 を返すため du で測る。
  {
    echo "engine|relname|bytes"
    psql_e bigm -A -t -F'|' <<'SQL'
SELECT 'bigm', relname, pg_relation_size(relid) FROM pg_stat_user_tables WHERE relname = 'messages';
SELECT 'bigm', indexrelname, pg_relation_size(indexrelid) FROM pg_stat_user_indexes
WHERE relname = 'messages';
SQL
    local dboid total
    dboid=$(psql_e groonga -A -t -c "SELECT oid FROM pg_database WHERE datname = 'bench';")
    psql_e groonga -A -t -F'|' <<SQL
SELECT 'groonga', relname, pg_relation_size(relid) FROM pg_stat_user_tables WHERE relname = 'messages';
SELECT 'groonga', indexrelname, pg_relation_size(indexrelid) FROM pg_stat_user_indexes
WHERE relname = 'messages';
SQL
    total=$(podman exec "$(c_name groonga)" sh -c \
      "du -cb \$PGDATA/base/$dboid/pgrn.* 2>/dev/null | tail -1 | cut -f1")
    echo "groonga|pgroonga_files_total|${total:-0}"
  } >"$WORK/index-size.tsv"

  # 追記負荷: 決定的 10000 件を索引なし/索引ありで投入し DO ブロック内で計測。
  # エンジンごとに SQL を組み立て直す（索引の種類だけが違う）。
  for e in bigm groonga; do
    {
      cat <<'SQL'
DROP TABLE IF EXISTS warm_result;
DROP TABLE IF EXISTS messages_warm;
CREATE UNLOGGED TABLE warm_result (phase text, ms numeric);
CREATE UNLOGGED TABLE messages_warm (body text);
DO $$
DECLARE t0 timestamptz;
BEGIN
  t0 := clock_timestamp();
  INSERT INTO messages_warm (body)
  SELECT '書き込み計測' || g || '番目: 会議資料とスケジュールの確認 ' || g
  FROM generate_series(1, 10000) g;
  INSERT INTO warm_result VALUES ('noindex', 1000*extract(epoch FROM clock_timestamp()-t0));
END $$;
SQL
      if [[ $e == groonga ]]; then
        echo "CREATE INDEX ON messages_warm USING pgroonga (body);"
      else
        echo "CREATE INDEX ON messages_warm USING gin (body gin_bigm_ops);"
      fi
      cat <<'SQL'
ANALYZE messages_warm;
DO $$
DECLARE t0 timestamptz;
BEGIN
  t0 := clock_timestamp();
  INSERT INTO messages_warm (body)
  SELECT '追記計測' || g || '件目: 進捗と不具合の確認 ' || g
  FROM generate_series(1, 10000) g;
  INSERT INTO warm_result VALUES ('withindex', 1000*extract(epoch FROM clock_timestamp()-t0));
END $$;
SELECT phase, round(ms) FROM warm_result;
SQL
    } >"$WORK/write-warm.sql"
    { echo "engine|phase|ms"; psql_e "$e" -A -t -F'|' -f /dev/stdin <"$WORK/write-warm.sql" \
        | sed "s/^/$e|/"; } >"$WORK/write-$e.tsv"
  done
  ok speed "speed.tsv / index-size.tsv / write-*.tsv 生成"
}

# ---------------------------------------------------------------- acl
phase_acl() {
  log "acl: 権限・保存期限・削除を同じ SQL で判定した検索の速さと件数"
  # 閲覧者 member=1 の EXISTS 句を alive 判定に足しただけで、測定は speed と同じ。
  local acl=" AND EXISTS (SELECT 1 FROM channel_members cm
              WHERE cm.channel_id = m.channel_id AND cm.member_id = 1)"
  {
    echo "engine|label|n|median_ms"
    for e in bigm groonga; do
      while IFS='|' read -r label query _needle _kind; do
        timed_search "$e" "$label" "$acl" "$(match_sql "$e" "'$query'")"
      done < <(eval_queries)
    done
  } >"$WORK/acl.tsv"
  # pg_bigm の ACL 込み件数は必ず strpos 正解と一致する（likequery ≡ LIKE）。
  # norm（表記ゆれ）クエリは needle と query が意図的に異なる文字列なので除外。
  local bad
  bad=$(psql_e bigm -A -t <<SQL
SELECT count(*) FROM eval_queries eq WHERE eq.kind <> 'norm' AND
 (SELECT count(*) FROM messages m WHERE $ALIVE
    AND EXISTS (SELECT 1 FROM channel_members cm
                WHERE cm.channel_id = m.channel_id AND cm.member_id = 1)
    AND strpos(m.body, eq.needle) > 0)
 <>
 (SELECT count(*) FROM messages m WHERE $ALIVE
    AND EXISTS (SELECT 1 FROM channel_members cm
                WHERE cm.channel_id = m.channel_id AND cm.member_id = 1)
    AND m.body LIKE likequery(eq.query));
SQL
)
  [[ $bad == 0 ]] && ok acl "pg_bigm の ACL 込み件数は strpos 正解と一致" \
    || fail acl "pg_bigm の件数が正解と不一致: $bad クエリ"
}

# ---------------------------------------------------------------- backup
phase_backup() {
  log "backup: pg_dump / kill -9 クラッシュ / pg_basebackup（R15 復元訓練相当）"
  local match_q="m.body &@~ '会議'"
  [[ -d "$WORK" ]] || mkdir -p "$WORK"
  : >"$WORK/backup.tsv"

  # 1) pg_dump → 別コンテナへリストア → 検索一致（索引は CREATE INDEX ごと再構築）
  for e in bigm groonga; do
    local match_q
    [[ $e == bigm ]] && match_q="m.body LIKE likequery('会議')" \
                      || match_q="m.body &@~ '会議'"
    podman exec "$(c_name "$e")" pg_dump -U postgres -d bench -Fc -f /tmp/bench.dump
    local c="ja-fts-$e-restore" expect got
    podman rm -f "$c" >/dev/null 2>&1 || true
    podman run -d --name "$c" -e POSTGRES_PASSWORD=bench -e POSTGRES_DB=bench \
      "$(img_name "$e")" >/dev/null
    wait_pg "$c"
    podman cp "$(c_name "$e"):/tmp/bench.dump" "$WORK/bench-$e.dump"
    podman cp "$WORK/bench-$e.dump" "$c:/tmp/bench.dump"
    podman exec "$c" pg_restore -U postgres -d bench --no-owner /tmp/bench.dump
    expect=$(psql_e "$e" -A -t -c \
      "SELECT count(*) FROM messages m WHERE $ALIVE AND $match_q;")
    got=$(podman exec -i "$c" psql -U postgres -d bench -X -A -t -v ON_ERROR_STOP=1 -c \
      "SELECT count(*) FROM messages m WHERE $ALIVE AND $match_q;" 2>&1) || true
    echo "dump|$e|expect=$expect|got=$got" >>"$WORK/backup.tsv"
    [[ "$got" == "$expect" ]] && ok "dump-$e" "pg_dump リストア後の検索一致 ($got)" \
      || fail "dump-$e" "リストア後不一致: expect=$expect got=$got"
    podman rm -f "$c" >/dev/null
  done

  # 2) kill -9 クラッシュ → 再起動 → 整合（PGroonga は enable_wal off/on 両方）
  crash_round() { # crash_round <engine> <wal on|off>
    local e=$1 wal=$2 match="m.body &@~ '会議'" n_before n_after reindexed=no
    [[ $e == bigm ]] && match="m.body LIKE likequery('会議')"
    if [[ $e == groonga ]]; then
      # pgroonga.enable_wal は PGroonga の .so をロードしたバックエンドでしか
      # 登録されない GUC。同一 psql セッション（同一 -c 列）内で検索を1回
      # 呼んでから ALTER SYSTEM する。
      psql_e "$e" \
        -c "SELECT count(*) FROM messages WHERE body &@~ '会議';" \
        -c "ALTER SYSTEM SET pgroonga.enable_wal = $( [[ $wal == on ]] && echo true || echo false );" \
        -c "SELECT pg_reload_conf();" >/dev/null
      sleep 1
    fi
    psql_e "$e" -q -c "INSERT INTO messages (channel_id, sender_id, body)
  VALUES (1, 1, 'クラッシュ試験: 会議資料の復元を確認する');" >/dev/null
    n_before=$(psql_e "$e" -A -t -c \
      "SELECT count(*) FROM messages m WHERE $ALIVE AND $match;")
    podman exec "$(c_name "$e")" bash -c \
      'kill -9 "$(head -1 "$PGDATA/postmaster.pid")"' 2>/dev/null || true
    sleep 2
    podman start "$(c_name "$e")" >/dev/null
    wait_pg "$(c_name "$e")"
    n_after=$(psql_e "$e" -A -t -c \
      "SELECT count(*) FROM messages m WHERE $ALIVE AND $match;" 2>&1) || true
    if [[ "$n_after" != "$n_before" ]]; then
      psql_e "$e" -q -c 'REINDEX INDEX messages_body_idx;' >/dev/null 2>&1 \
        && n_after=$(psql_e "$e" -A -t -c \
          "SELECT count(*) FROM messages m WHERE $ALIVE AND $match;") && reindexed=yes
    fi
    echo "crash|$e|wal=$wal|before=$n_before|after=$n_after|reindexed=$reindexed" >>"$WORK/backup.tsv"
    [[ "$n_after" == "$n_before" ]] \
      && ok "crash-$e-$wal" "kill -9 再起動後に検索一致 (wal=$wal, reindex=$reindexed)" \
      || fail "crash-$e-$wal" "再起動後不一致: before=$n_before after=$n_after"
  }
  crash_round bigm off   # pg_bigm は PG 標準 WAL（対照）
  crash_round groonga off
  crash_round groonga on

  # 3) pg_basebackup → 同一コンテナ内の第二インスタンスで復元 → 検索一致
  for e in bigm groonga; do
    local match_q2
    [[ $e == bigm ]] && match_q2="m.body LIKE likequery('会議')" \
                       || match_q2="m.body &@~ '会議'"
    podman exec "$(c_name "$e")" rm -rf /tmp/bb /tmp/bb.log
    # exec の既定ユーザーは root。postgres ユーザーで取らないと第二インスタンスが
    # 起動できない（postmaster.pid が読めない）。
    podman exec --user postgres "$(c_name "$e")" \
      pg_basebackup -U postgres -D /tmp/bb -Fp -Xs
    # ソケットは /tmp/bb に分離し、TCP を無効化して第一インスタンスとの
    # 衝突を避ける。スクリプトは一旦ファイルに書いて podman cp で渡す。
    cat >"$WORK/start_bb.sh" <<'EOS'
#!/bin/sh
exec su postgres -c "/usr/local/bin/pg_ctl -D /tmp/bb -o '-c unix_socket_directories=/tmp/bb -c listen_addresses=' -l /tmp/bb.log -w start"
EOS
    podman cp "$WORK/start_bb.sh" "$(c_name "$e"):/tmp/start_bb.sh"
    podman exec -d "$(c_name "$e")" sh /tmp/start_bb.sh
    local bb_ready=0 expect2 got2
    for _ in $(seq 1 30); do
      podman exec "$(c_name "$e")" pg_isready -h /tmp/bb -U postgres -d bench -q \
        && { bb_ready=1; break; }
      sleep 1
    done
    expect2=$(psql_e "$e" -A -t -c \
      "SELECT count(*) FROM messages m WHERE $ALIVE AND $match_q2;")
    got2=$(podman exec -i "$(c_name "$e")" psql -h /tmp/bb -U postgres -d bench \
      -X -A -t -v ON_ERROR_STOP=1 -c \
      "SELECT count(*) FROM messages m WHERE $ALIVE AND $match_q2;" 2>&1) || true
    echo "basebackup|$e|expect=$expect2|got=$got2" >>"$WORK/backup.tsv"
    [[ $bb_ready == 1 && "$got2" == "$expect2" ]] \
      && ok "basebackup-$e" "pg_basebackup 復元後も検索一致 ($got2)" \
      || fail "basebackup-$e" "復元後不一致/起動失敗: ready=$bb_ready expect=$expect2 got=$got2"
    podman exec "$(c_name "$e")" su postgres -c \
      '/usr/local/bin/pg_ctl -D /tmp/bb -m immediate stop' >/dev/null 2>&1 || true
  done
}

# ---------------------------------------------------------------- partial
phase_partial() {
  log "partial: 英字部分一致（pp→Apple）と NormalizerNFKC の正規化・fuzzy"
  local pp_like
  # 対照（pg_bigm = LIKE と同義なので部分一致は当然ヒットする）
  pp_like=$(psql_e bigm -A -t -c \
    "SELECT count(*) FROM messages m WHERE $ALIVE AND strpos(m.body, 'pp') > 0;")
  # PGroonga: 同じ 3 行に既定 TokenBigram と TokenBigramSplitSymbolAlpha
  #（英字も bigram 化）の索引を両方作り、pgroonga_condition の index_name で
  # 索引を指定して比較する。語途中の部分一致語が両構成で拾えるかを見る。
  psql_e groonga -q <<'SQL'
DROP TABLE IF EXISTS partial_test;
CREATE TABLE partial_test (id int PRIMARY KEY, body text);
INSERT INTO partial_test VALUES
  (1, 'Apple Silicon のビルド時間について'),
  (2, 'PostgreSQL のアップグレードを試しました'),
  (3, 'コードの grep は rg を使っています');
CREATE INDEX def_idx   ON partial_test USING pgroonga (body);
CREATE INDEX split_idx ON partial_test USING pgroonga (body)
  WITH (tokenizer='TokenBigramSplitSymbolAlpha');
SQL
  {
    echo "partial|strpos_like_pp=$pp_like"
    local q hits
    for q in pp ostgre ilico; do
      hits=$(psql_e groonga -A -t -v ON_ERROR_STOP=1 -c \
        "SELECT count(*) FROM partial_test
           WHERE body &@~ pgroonga_condition('$q', index_name => 'def_idx');")
      echo "partial|query=$q|tokenizer=TokenBigram|hits=$hits"
      hits=$(psql_e groonga -A -t -v ON_ERROR_STOP=1 -c \
        "SELECT count(*) FROM partial_test
           WHERE body &@~ pgroonga_condition('$q', index_name => 'split_idx');")
      echo "partial|query=$q|tokenizer=TokenBigramSplitSymbolAlpha|hits=$hits"
    done
  } >"$WORK/partial.tsv"
  # 既定 TokenBigram は語途中の部分一致を拾えず（0）、SplitSymbolAlpha は拾う（1）。
  local def_hits split_hits
  def_hits=$(awk -F'|' '$2=="query=ostgre" && $3=="tokenizer=TokenBigram"{sub("hits=","",$4);print $4}' "$WORK/partial.tsv")
  split_hits=$(awk -F'|' '$2=="query=ostgre" && $3~/SplitSymbolAlpha/{sub("hits=","",$4);print $4}' "$WORK/partial.tsv")
  [[ ${def_hits:-x} == 0 && ${split_hits:-x} == 1 ]] \
    && ok partial-split "既定は ostgre→PostgreSQL 不可、SplitSymbolAlpha で可 (def=$def_hits split=$split_hits)" \
    || fail partial-split "想定外: def=$def_hits split=$split_hits（def は 0、split は 1 が期待値）"

  # 正規化: 全角「スケジュール」クエリで半角カナ行（ｽｹｼﾞｭｰﾙ）を拾えるか
  local norm_hit norm_total
  norm_hit=$(psql_e groonga -A -t -c \
    "SELECT count(*) FROM messages m WHERE $ALIVE AND m.body &@~ 'スケジュール'
       AND strpos(m.body, 'ｽｹｼﾞｭｰﾙ') > 0;")
  norm_total=$(psql_e groonga -A -t -c \
    "SELECT count(*) FROM messages m WHERE $ALIVE AND strpos(m.body, 'ｽｹｼﾞｭｰﾙ') > 0;")
  echo "norm|halfkata_rows=$norm_total|hit_by_normalized_query=$norm_hit" >>"$WORK/partial.tsv"
  [[ ${norm_total:-0} -gt 0 && "$norm_hit" == "$norm_total" ]] \
    && ok partial-norm "半角カナ行を正規形クエリが拾えた ($norm_hit/$norm_total)" \
    || fail partial-norm "正規化ヒット不一致: $norm_hit/$norm_total"

  # fuzzy（3.1.6+）: 誤字「進捙」クエリで正字「進捗」行を拾えるか
  local fuzzy_typo fuzzy_plain
  fuzzy_typo=$(psql_e groonga -A -t -c \
    "SELECT count(*) FROM messages m WHERE $ALIVE
       AND m.body &@~ pgroonga_condition('進捙', fuzzy_max_distance_ratio => 0.34);" 2>&1) || true
  fuzzy_plain=$(psql_e groonga -A -t -c \
    "SELECT count(*) FROM messages m WHERE $ALIVE
       AND strpos(m.body, '進捗') > 0;")
  echo "fuzzy|query=進捙_ratio0.34_hit=$fuzzy_typo|truth_進捗_rows=$fuzzy_plain" >>"$WORK/partial.tsv"
  ok partial-fuzzy "fuzzy の結果を記録: hit=$fuzzy_typo / 正字行=$fuzzy_plain"
}

# ---------------------------------------------------------------- verify
phase_verify() {
  log "verify: 全計測の出力と check を確認"
  local required=(
    setup quality speed acl
    dump-bigm dump-groonga
    crash-bigm-off crash-groonga-off crash-groonga-on
    basebackup-bigm basebackup-groonga
    partial-split partial-norm partial-fuzzy
  )
  for c in "${required[@]}"; do
    [[ -f "$CHECKS/$c.ok" ]] || fail verify "check が無い: $c"
  done
  for f in quality.tsv quality-top10.tsv speed.tsv index-size.tsv write-bigm.tsv \
           write-groonga.tsv acl.tsv backup.tsv partial.tsv; do
    [[ -s "$WORK/$f" ]] || fail verify "結果ファイルが空/未生成: $f"
  done
  echo "== verify: 全項目 PASS =="
  echo "-- index-size.tsv"; cat "$WORK/index-size.tsv"
  echo "-- backup.tsv";    cat "$WORK/backup.tsv"
  echo "-- partial.tsv";   cat "$WORK/partial.tsv"
}

# ---------------------------------------------------------------- cleanup
phase_cleanup() {
  phase_cleanup_containers
  log "cleanup 完了"
}

case ${1:-} in
  build)   phase_build ;;
  setup)   phase_setup ;;
  quality) phase_quality ;;
  speed)   phase_speed ;;
  acl)     phase_acl ;;
  backup)  phase_backup ;;
  partial) phase_partial ;;
  verify)  phase_verify ;;
  cleanup) phase_cleanup ;;
  all)     phase_build; phase_setup; phase_quality; phase_speed
           phase_acl; phase_backup; phase_partial; phase_verify ;;
  *) echo "usage: $0 build|setup|quality|speed|acl|backup|partial|verify|cleanup|all" >&2; exit 2 ;;
esac
