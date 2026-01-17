-- workflow_instances テーブルの作成
-- 構文リファレンス: README.md
--
-- 実行中のワークフローインスタンスを管理する。
-- 定義から生成され、申請から完了までのライフサイクルを持つ。

CREATE TABLE workflow_instances (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    definition_id UUID NOT NULL REFERENCES workflow_definitions(id),
    definition_version INTEGER NOT NULL,
    title VARCHAR(500) NOT NULL,
    form_data JSONB NOT NULL DEFAULT '{}',
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    current_step_id VARCHAR(100),
    initiated_by UUID NOT NULL REFERENCES users(id),
    submitted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT workflow_instances_status_check CHECK (
        status IN ('draft', 'pending', 'in_progress', 'approved', 'rejected', 'cancelled')
    )
);

-- インデックス
CREATE INDEX workflow_instances_tenant_status_idx ON workflow_instances(tenant_id, status);
CREATE INDEX workflow_instances_initiated_by_idx ON workflow_instances(initiated_by);
CREATE INDEX workflow_instances_created_at_idx ON workflow_instances(tenant_id, created_at DESC);

-- updated_at 自動更新トリガー
CREATE TRIGGER workflow_instances_updated_at
    BEFORE UPDATE ON workflow_instances
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON TABLE workflow_instances IS 'ワークフローインスタンス（申請案件）';
COMMENT ON COLUMN workflow_instances.id IS '主キー';
COMMENT ON COLUMN workflow_instances.tenant_id IS 'テナントID（FK）';
COMMENT ON COLUMN workflow_instances.definition_id IS '定義ID（FK）';
COMMENT ON COLUMN workflow_instances.definition_version IS '定義バージョン（作成時点）';
COMMENT ON COLUMN workflow_instances.title IS 'タイトル';
COMMENT ON COLUMN workflow_instances.form_data IS 'フォームデータ（JSON）';
COMMENT ON COLUMN workflow_instances.status IS '状態（draft/pending/in_progress/approved/rejected/cancelled）';
COMMENT ON COLUMN workflow_instances.current_step_id IS '現在のステップID';
COMMENT ON COLUMN workflow_instances.initiated_by IS '申請者（FK）';
COMMENT ON COLUMN workflow_instances.submitted_at IS '申請日時';
COMMENT ON COLUMN workflow_instances.completed_at IS '完了日時';
