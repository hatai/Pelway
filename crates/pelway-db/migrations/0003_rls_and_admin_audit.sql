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

-- 管理ロール（BYPASSRLS）の利用記録と、監査付きの管理経路。
-- ADR 0005 の Consequences（ポリシーを回避するロールの利用は監査の対象）に従う。
--
-- 権限の分離: 管理ステートメントの実行主体（pelway_admin_executor）には監査表への
-- 権限を一切与えない。監査表の所有者は別ロール（pelway_audit_writer）で、追記を
-- 行う関数だけを提供する。このため管理ステートメントから監査記録を削除・改変する
-- ことはできず、監査は追記のみ（append-only）になる。db_user には接続ユーザー
-- （session_user）を記録する。statement は 1 文だけ。search_path を固定して
-- 定義者権限の乗っ取りを防ぐ。
CREATE TABLE admin_access_log (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    db_user     TEXT        NOT NULL,
    reason      TEXT        NOT NULL,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
ALTER TABLE admin_access_log OWNER TO pelway_audit_writer;

-- 監査行の追記（executor から呼ばれる）。所有者権限で INSERT する。
CREATE FUNCTION pelway_audit_write(reason text) RETURNS void
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, public
AS $$ BEGIN INSERT INTO admin_access_log (db_user, reason) VALUES (session_user, reason); END $$;

CREATE FUNCTION pelway_admin_exec(reason text, statement text) RETURNS void
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, public
AS $$
BEGIN
    PERFORM pelway_audit_write(reason);
    EXECUTE statement;
END
$$;

CREATE FUNCTION pelway_admin_query(reason text, statement text) RETURNS text
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, public
AS $$
DECLARE
    result text;
BEGIN
    PERFORM pelway_audit_write(reason);
    EXECUTE statement INTO result;
    RETURN result;
END
$$;

ALTER FUNCTION pelway_admin_exec(text, text) OWNER TO pelway_admin_executor;
ALTER FUNCTION pelway_admin_query(text, text) OWNER TO pelway_admin_executor;
ALTER FUNCTION pelway_audit_write(text) OWNER TO pelway_audit_writer;

-- 関数の EXECUTE 権限は既定で PUBLIC に付く。奪って限定した相手だけに渡す。
REVOKE ALL ON FUNCTION pelway_admin_exec(text, text),
    pelway_admin_query(text, text), pelway_audit_write(text) FROM PUBLIC;
GRANT EXECUTE
    ON FUNCTION pelway_admin_exec(text, text), pelway_admin_query(text, text)
    TO pelway_admin;
GRANT EXECUTE ON FUNCTION pelway_audit_write(text) TO pelway_admin_executor;
