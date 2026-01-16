-- roles テーブルの作成
--
-- 権限ロールを定義する。
-- システムロール（tenant_id = NULL）とテナント固有ロールをサポート。

CREATE TABLE roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID REFERENCES tenants(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    permissions JSONB NOT NULL DEFAULT '[]',
    is_system BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT roles_tenant_name_key UNIQUE (tenant_id, name)
);

-- updated_at 自動更新トリガー
CREATE TRIGGER roles_updated_at
    BEFORE UPDATE ON roles
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON TABLE roles IS '権限ロール';
COMMENT ON COLUMN roles.id IS '主キー';
COMMENT ON COLUMN roles.tenant_id IS 'テナントID（NULL = システムロール）';
COMMENT ON COLUMN roles.name IS 'ロール名';
COMMENT ON COLUMN roles.description IS '説明';
COMMENT ON COLUMN roles.permissions IS '権限リスト（JSON配列）';
COMMENT ON COLUMN roles.is_system IS 'システム定義ロールか（削除・編集不可）';
