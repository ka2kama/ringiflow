-- フォルダ管理テーブルの作成
-- 詳細設計書: docs/40_詳細設計書/17_ドキュメント管理設計.md
--
-- materialized path パターンで階層構造を管理する。
-- path 列にルートからの完全パス（例: "/企画書/2026年度/"）を格納し、
-- LIKE 検索でサブツリーを高速に取得できる。

CREATE TABLE folders (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    parent_id   UUID REFERENCES folders(id) ON DELETE RESTRICT,
    path        TEXT NOT NULL,
    depth       INTEGER NOT NULL CHECK (depth >= 1 AND depth <= 5),
    created_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    -- 同一テナント・同一親フォルダ内で名前の重複を禁止
    -- NULLS NOT DISTINCT: parent_id が NULL（ルート直下）でも重複チェックが機能する
    UNIQUE NULLS NOT DISTINCT (tenant_id, parent_id, name)
);

-- RLS 有効化
ALTER TABLE folders ENABLE ROW LEVEL SECURITY;

-- テナント分離ポリシー
CREATE POLICY tenant_isolation ON folders
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- インデックス
CREATE INDEX idx_folders_tenant_id ON folders (tenant_id);
CREATE INDEX idx_folders_parent_id ON folders (parent_id);
CREATE INDEX idx_folders_path ON folders (path);
