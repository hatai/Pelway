-- RLS を示すのに必要な最小の表（04 §2 の論理表の一部。確定 DDL ではない）。
-- workspace_id は契約型 WorkspaceId（文字列 newtype）に合わせて TEXT とする。

-- 会社（テナント）の根となる表。RLS のポリシーは id = 現在の workspace_id。
CREATE TABLE workspaces (
    id         TEXT PRIMARY KEY,
    name       TEXT        NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- チャンネル。複合主キーの第 1 列が workspace_id で、他社の行と衝突しない。
CREATE TABLE channels (
    workspace_id TEXT        NOT NULL REFERENCES workspaces (id),
    id           TEXT        NOT NULL,
    name         TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id)
);

-- メッセージ。チャンネルへの複合外部キーで、別のワークスペースのチャンネルを
-- 参照する行を作れない。unique(workspace, channel, seq) は 04 §2 のとおり。
CREATE TABLE messages (
    workspace_id TEXT        NOT NULL,
    channel_id   TEXT        NOT NULL,
    id           TEXT        NOT NULL,
    seq          BIGINT      NOT NULL,
    author       TEXT        NOT NULL,
    body         TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, id),
    FOREIGN KEY (workspace_id, channel_id) REFERENCES channels (workspace_id, id),
    UNIQUE (workspace_id, channel_id, seq)
);

-- pelway_admin には表への直接権限を与えない。管理ロールができるのは 0003 の
-- 監査付き関数（pelway_admin_exec / pelway_admin_query）の呼び出しだけで、
-- それ以外の経路では何も読み書きできない。
GRANT SELECT, INSERT, UPDATE, DELETE
    ON workspaces, channels, messages TO pelway_app;
