-- ドキュメント管理テーブルの作成
-- 詳細設計書: docs/03_詳細設計書/17_ドキュメント管理設計.md
--
-- Presigned URL 方式のファイルアップロードで使用する。
-- folder_id と workflow_instance_id は排他制約（XOR）で、
-- ドキュメントは必ずどちらか一方のコンテキストに属する。

CREATE TABLE documents (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id             UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    filename              VARCHAR(255) NOT NULL,
    content_type          VARCHAR(100) NOT NULL,
    size                  BIGINT NOT NULL,
    s3_key                VARCHAR(1000) NOT NULL,
    folder_id             UUID REFERENCES folders(id) ON DELETE SET NULL,
    workflow_instance_id  UUID REFERENCES workflow_instances(id) ON DELETE CASCADE,
    status                VARCHAR(20) NOT NULL DEFAULT 'uploading',
    uploaded_by           UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at            TIMESTAMPTZ,
    -- folder_id XOR workflow_instance_id: 必ずどちらか一方のみ
    CONSTRAINT documents_context_check CHECK (
        (folder_id IS NOT NULL AND workflow_instance_id IS NULL)
        OR (folder_id IS NULL AND workflow_instance_id IS NOT NULL)
    )
);

-- RLS 有効化
ALTER TABLE documents ENABLE ROW LEVEL SECURITY;

-- テナント分離ポリシー（folders.sql と同じ NULLIF パターン）
CREATE POLICY tenant_isolation ON documents
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- インデックス
CREATE INDEX idx_documents_tenant_id ON documents (tenant_id);
CREATE INDEX idx_documents_folder_id ON documents (folder_id) WHERE folder_id IS NOT NULL;
CREATE INDEX idx_documents_workflow_instance_id ON documents (workflow_instance_id) WHERE workflow_instance_id IS NOT NULL;
CREATE INDEX idx_documents_status ON documents (status) WHERE status != 'deleted';
