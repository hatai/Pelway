-- アプリと管理で使うロール。ロールはクラスタ全体で共有されるため、
-- 並行するマイグレーションで二重に作らないよう、トランザクション内
-- アドバイザリロックで直列化する（sqlx は各ファイルを 1 トランザクションで
-- 実行する）。
--
-- - pelway_app: アプリ用。RLS ポリシーの適用対象。
-- - pelway_admin: 管理の呼び出し口。0003 の監査付き関数だけを呼べる。
-- - pelway_admin_executor: 監査付き関数の所有者（SECURITY DEFINER の実行主体）。
--   BYPASSRLS と tenant 表への DML 権限を持ち、監査表には権限を持たない。
-- - pelway_audit_writer: 監査表の所有者。監査行の追記だけを行う。
SELECT pg_advisory_xact_lock(4210100);

DO $$
DECLARE
    role_name text;
BEGIN
    FOREACH role_name IN ARRAY ARRAY['pelway_app', 'pelway_audit_writer'] LOOP
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = role_name) THEN
            EXECUTE format('CREATE ROLE %I NOLOGIN', role_name);
        END IF;
    END LOOP;
    FOREACH role_name IN ARRAY ARRAY['pelway_admin', 'pelway_admin_executor'] LOOP
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = role_name) THEN
            EXECUTE format('CREATE ROLE %I NOLOGIN BYPASSRLS', role_name);
        ELSE
            -- 既存ロールにも属性を合わせる（BYPASSRLS の付与にはスーパーユーザーが要る）。
            EXECUTE format('ALTER ROLE %I BYPASSRLS', role_name);
        END IF;
    END LOOP;
END
$$;
