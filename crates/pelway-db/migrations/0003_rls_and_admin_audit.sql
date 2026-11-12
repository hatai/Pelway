-- 各テーブルの RLS ポリシー（ADR 0005）。現在の workspace_id は app.workspace_id
-- に設定された値（begin_workspace_tx が set_config でトランザクションの中だけで
-- 設定する）。未設定のとき current_setting は NULL を返し、比較は NULL になるため
-- どの行も返さない（fail-closed）。
--
-- なお PostgreSQL のプレースホルダ GUC は、トランザクション限定の設定が
-- コミット・ロールバックで戻るとき空文字列になることがある。nullif で空文字列も
-- NULL と同じ扱いにし、空の workspace_id を持つ行にも一致しないようにする。

ALTER TABLE workspaces ENABLE ROW LEVEL SECURITY;
ALTER TABLE workspaces FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON workspaces
    FOR ALL TO pelway_app
    USING (id = nullif(current_setting('app.workspace_id', true), ''))
    WITH CHECK (id = nullif(current_setting('app.workspace_id', true), ''));

ALTER TABLE channels ENABLE ROW LEVEL SECURITY;
ALTER TABLE channels FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON channels
    FOR ALL TO pelway_app
    USING (workspace_id = nullif(current_setting('app.workspace_id', true), ''))
    WITH CHECK (workspace_id = nullif(current_setting('app.workspace_id', true), ''));

ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
ALTER TABLE messages FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON messages
    FOR ALL TO pelway_app
    USING (workspace_id = nullif(current_setting('app.workspace_id', true), ''))
    WITH CHECK (workspace_id = nullif(current_setting('app.workspace_id', true), ''));

-- 管理ロール（BYPASSRLS）の監査付き経路（ADR 0005 の Consequences）。
-- 実行主体（pelway_admin_executor）は tenant 表への DML と監査表への SELECT のみ
-- （削除・改変は不可能 = append-only）。監査表への書き込みは所有者
-- （pelway_audit_writer）の関数だけが行う。実行手続きは同じ reason の監査行が
-- 無ければ拒否し、呼び出し側（admin_exec ヘルパー）は監査行を先に別の文として
-- 確定するため、ステートメントが失敗（拒否を含む）しても試行が監査に残る。
CREATE TABLE admin_access_log (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    db_user     TEXT        NOT NULL,
    reason      TEXT        NOT NULL,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE admin_access_log OWNER TO pelway_audit_writer;
GRANT SELECT ON admin_access_log TO pelway_admin_executor;

-- 監査行の追記。所有者権限で INSERT する。
CREATE FUNCTION pelway_audit_write(reason text) RETURNS void
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, public
AS $$ BEGIN INSERT INTO admin_access_log (db_user, reason) VALUES (session_user, reason); END $$;

-- 管理ステートメント（1 文）の実行。search_path 固定で定義者権限の乗っ取りを防ぐ。
CREATE PROCEDURE pelway_admin_exec(reason text, statement text)
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, public
AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM admin_access_log WHERE admin_access_log.reason = pelway_admin_exec.reason) THEN
        RAISE EXCEPTION 'admin statement requires a preceding audit row';
    END IF;
    EXECUTE statement;
END
$$;

ALTER PROCEDURE pelway_admin_exec(text, text) OWNER TO pelway_admin_executor;
ALTER FUNCTION pelway_audit_write(text) OWNER TO pelway_audit_writer;

-- EXECUTE 権限は既定で PUBLIC に付く。奪って限定した相手だけに渡す。
REVOKE ALL ON PROCEDURE pelway_admin_exec(text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION pelway_audit_write(text) FROM PUBLIC;
GRANT EXECUTE ON PROCEDURE pelway_admin_exec(text, text) TO pelway_admin;
GRANT EXECUTE ON FUNCTION pelway_audit_write(text) TO pelway_admin;
