-- 管理操作（pelway_admin によるテナント表への読み書き）の監査（#149、ADR 0005）。
-- 方式は「専用経路での監査先行・別トランザクション確定」（Issue #149 の質問票
-- Q1〜Q9 で確定）。監査行は ADR 0007 の prev_hash 鎖の形式（単一鎖。workspace_id
-- は固定値 'admin'）。削除・改竄は署名付きチェックポイント（ADR 0007）で検出する
-- ため、権限は pelway_audit の SELECT + INSERT だけに絞る。
SELECT pg_advisory_xact_lock(4210100);

-- 監査表への書き込みだけを行うロール（NOLOGIN のグループロール。接続ユーザーは
-- メンバーとして SET ROLE で降格する。0001_roles.sql と同じ作り方）。
DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'pelway_audit') THEN
        CREATE ROLE pelway_audit NOLOGIN;
    END IF;
END
$$;
ALTER ROLE pelway_audit NOSUPERUSER NOBYPASSRLS NOLOGIN;

-- 1 操作 = intent 行（操作前）+ result 行（操作後）の 2 行。kind は Q3 で確定した
-- 2 種類だけ。ref_id は result 行から対応する intent 行への参照（intent 行は NULL。
-- UNIQUE で 1 行の intent に結果は 1 行、CHECK で kind との整合を強制する）。
-- ref_id も row_hash の対象（タグ 0x02）で、付け替えは歩査とチェックポイントで
-- 検出できる。
CREATE TABLE admin_audit_events (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    workspace_id   TEXT        NOT NULL CHECK (workspace_id = 'admin'),
    at_ms          BIGINT      NOT NULL,
    actor          TEXT        NOT NULL,
    subject        TEXT        NOT NULL,
    reason         TEXT        NOT NULL,
    kind           TEXT        NOT NULL CHECK (kind IN ('admin-op-intent', 'admin-op-result')),
    result         TEXT        NOT NULL,
    snapshot_hash  BYTEA       NOT NULL CHECK (octet_length(snapshot_hash) = 32),
    policy_version BIGINT      NOT NULL,
    prev_hash      BYTEA       NOT NULL CHECK (octet_length(prev_hash) = 32),
    hash           BYTEA       NOT NULL CHECK (octet_length(hash) = 32),
    ref_id         BIGINT      UNIQUE REFERENCES admin_audit_events (id),
    CHECK ((kind = 'admin-op-intent' AND ref_id IS NULL) OR (kind = 'admin-op-result' AND ref_id IS NOT NULL))
);

-- pelway_admin・pelway_app には一切の権限を与えない（default で PUBLIC には無いが
-- 意図の明示）。RLS はかけない: 触れるロールが pelway_audit に限られるため。
REVOKE ALL ON admin_audit_events FROM PUBLIC;
GRANT SELECT, INSERT ON admin_audit_events TO pelway_audit;
