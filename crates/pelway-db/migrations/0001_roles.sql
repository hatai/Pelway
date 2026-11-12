-- アプリと管理で使うロール。ロールはクラスタ全体で共有されるため、
-- 並行するマイグレーションで二重に作らないよう、トランザクション内
-- アドバイザリロックで直列化する（sqlx は各ファイルを 1 トランザクションで
-- 実行する）。
SELECT pg_advisory_xact_lock(4210100);

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'pelway_app') THEN
        CREATE ROLE pelway_app NOLOGIN;
    END IF;
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'pelway_admin') THEN
        CREATE ROLE pelway_admin NOLOGIN BYPASSRLS;
    ELSE
        -- 既存ロール（旧マイグレーションや scratch database の使い回し）にも
        -- 属性を合わせる。BYPASSRLS の付与にはスーパーユーザーが要る。
        ALTER ROLE pelway_admin BYPASSRLS;
    END IF;
END
$$;
