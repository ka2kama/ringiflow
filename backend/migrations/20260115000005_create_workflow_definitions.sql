-- workflow_definitions テーブルの作成
--
-- ワークフローのテンプレート定義を管理する。
-- JSON で定義を保持し、バージョン管理に対応。

CREATE TABLE workflow_definitions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    definition JSONB NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_by UUID NOT NULL REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT workflow_definitions_status_check CHECK (status IN ('draft', 'published', 'archived'))
);

-- インデックス
CREATE INDEX workflow_definitions_tenant_status_idx ON workflow_definitions(tenant_id, status);

-- updated_at 自動更新トリガー
CREATE TRIGGER workflow_definitions_updated_at
    BEFORE UPDATE ON workflow_definitions
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON TABLE workflow_definitions IS 'ワークフロー定義（テンプレート）';
COMMENT ON COLUMN workflow_definitions.id IS '主キー';
COMMENT ON COLUMN workflow_definitions.tenant_id IS 'テナントID（FK）';
COMMENT ON COLUMN workflow_definitions.name IS '定義名';
COMMENT ON COLUMN workflow_definitions.description IS '説明';
COMMENT ON COLUMN workflow_definitions.version IS 'バージョン';
COMMENT ON COLUMN workflow_definitions.definition IS '定義本体（JSON）';
COMMENT ON COLUMN workflow_definitions.status IS '状態（draft/published/archived）';
COMMENT ON COLUMN workflow_definitions.created_by IS '作成者（FK）';
