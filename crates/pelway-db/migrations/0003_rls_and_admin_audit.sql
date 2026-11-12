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

-- 管理ロール（BYPASSRLS）の利用記録。ADR 0005 の Consequences（ポリシーを回避する
-- ロールの利用は監査の対象）に従う。アプリロールには権限を与えない。
CREATE TABLE admin_access_log (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    db_user     TEXT        NOT NULL,
    reason      TEXT        NOT NULL,
    started_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
GRANT INSERT, SELECT ON admin_access_log TO pelway_admin;
GRANT USAGE ON SEQUENCE admin_access_log_id_seq TO pelway_admin;
