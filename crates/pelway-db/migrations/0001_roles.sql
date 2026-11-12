-- アプリと管理で使うロール（クラスタ全体で共有）。並行するマイグレーションで
-- 二重に作らないよう、トランザクション内 advisory lock で直列化する。
--
-- - pelway_app: アプリ用。RLS ポリシーの適用対象。
-- - pelway_admin: 管理の呼び出し口。0003 の監査付き手続きだけを呼べる。
-- - pelway_admin_executor: 監査付き手続きの所有者（SECURITY DEFINER の実行主体）。
--   BYPASSRLS と tenant 表への DML 権限を持ち、監査表には SELECT のみ。
-- - pelway_audit_writer: 監査表の所有者。監査行の追記だけを行う。
SELECT pg_advisory_xact_lock(4210100);

DO $$
DECLARE
    role_name text;
BEGIN
    FOREACH role_name IN ARRAY ARRAY['pelway_app', 'pelway_admin', 'pelway_admin_executor', 'pelway_audit_writer'] LOOP
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = role_name) THEN
            EXECUTE format('CREATE ROLE %I NOLOGIN', role_name);
        END IF;
    END LOOP;
END
$$;

-- 既存ロールにも属性を明示的に合わせる。アプリロールが SUPERUSER / BYPASSRLS の
-- ままだと RLS の強制が崩れ、管理側ロールが SUPERUSER だと監査表の権限分離を
-- スーパーユーザー権限で壊せるため、どちらも許さない。
ALTER ROLE pelway_app            NOSUPERUSER NOBYPASSRLS NOLOGIN;
ALTER ROLE pelway_admin          NOSUPERUSER BYPASSRLS   NOLOGIN;
ALTER ROLE pelway_admin_executor NOSUPERUSER BYPASSRLS   NOLOGIN;
ALTER ROLE pelway_audit_writer   NOSUPERUSER NOBYPASSRLS NOLOGIN;
