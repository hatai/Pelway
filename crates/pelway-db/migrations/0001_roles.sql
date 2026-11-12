-- アプリと管理で使うロール（クラスタ全体で共有）。並行するマイグレーションで
-- 二重に作らないよう、トランザクション内 advisory lock で直列化する。
--
-- - pelway_app: アプリ用。RLS ポリシーの適用対象。
-- - pelway_admin: 管理用（移行・バックアップ・横断の運営作業）。BYPASSRLS で
--   ポリシーを回避する。その利用の監査は #149 で設計する。
SELECT pg_advisory_xact_lock(4210100);

DO $$
DECLARE
    role_name text;
BEGIN
    FOREACH role_name IN ARRAY ARRAY['pelway_app', 'pelway_admin'] LOOP
        IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = role_name) THEN
            EXECUTE format('CREATE ROLE %I NOLOGIN', role_name);
        END IF;
    END LOOP;
END
$$;

-- 既存ロールにも属性を明示的に合わせる。アプリロールが SUPERUSER / BYPASSRLS の
-- ままだと RLS の強制が崩れるため許さない。
ALTER ROLE pelway_app   NOSUPERUSER NOBYPASSRLS NOLOGIN;
ALTER ROLE pelway_admin NOSUPERUSER BYPASSRLS   NOLOGIN;
