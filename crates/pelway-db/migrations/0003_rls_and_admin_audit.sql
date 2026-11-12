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
CREATE TABLE admin_access_log (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    db_user     TEXT        NOT NULL,
    reason      TEXT        NOT NULL,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- pelway_admin には表への直接権限が無いため、この2つの SECURITY DEFINER 関数が
-- 管理操作の唯一の経路になる。関数は監査行を書いてからステートメントを実行する
-- （監査と操作は同じトランザクションで原子）。よってロールの能力を使えば必ず
-- 監査行が残る。db_user には接続ユーザー（session_user）を記録する。
--
-- 関数の所有者はマイグレーションを実行したロール（開発ではスーパーユーザー）。
-- 本番では表の所有者と別の、BYPASSRLS と表権限を持つ NOLOGIN ロールに所有させる
-- （所有者だと FORCE ROW LEVEL SECURITY が効いてしまう）。
-- statement は 1 文だけ。search_path を固定して定義者権限の乗っ取りを防ぐ。
CREATE FUNCTION pelway_admin_exec(reason text, statement text) RETURNS void
    LANGUAGE plpgsql
    SECURITY DEFINER
    SET search_path = pg_catalog, public
AS $$
BEGIN
    INSERT INTO admin_access_log (db_user, reason) VALUES (session_user, reason);
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
    INSERT INTO admin_access_log (db_user, reason) VALUES (session_user, reason);
    EXECUTE statement INTO result;
    RETURN result;
END
$$;

-- 関数の EXECUTE 権限は既定で PUBLIC に付く。奪って管理ロールだけに渡す。
REVOKE ALL ON FUNCTION pelway_admin_exec(text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION pelway_admin_query(text, text) FROM PUBLIC;
GRANT EXECUTE
    ON FUNCTION pelway_admin_exec(text, text), pelway_admin_query(text, text)
    TO pelway_admin;
