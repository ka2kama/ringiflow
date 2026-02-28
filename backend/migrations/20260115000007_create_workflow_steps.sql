-- workflow_steps テーブルの作成
-- 構文リファレンス: README.md
--
-- ワークフローの各ステップの実行状態を管理する。
-- インスタンスに紐付き、担当者への割り当てや承認/却下の記録を保持。
--
-- 注: id は UUID v7（時系列ソート可能）を使用。
-- アプリケーション側で生成するため DEFAULT 句なし。
-- 参照: docs/70_ADR/001_ID形式の選定.md

CREATE TABLE workflow_steps (
    id UUID PRIMARY KEY,
    instance_id UUID NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    step_id VARCHAR(100) NOT NULL,
    step_name VARCHAR(255) NOT NULL,
    step_type VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    assigned_to UUID REFERENCES users(id) ON DELETE SET NULL,
    decision VARCHAR(50),
    comment TEXT,
    due_date TIMESTAMPTZ,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT workflow_steps_status_check CHECK (status IN ('pending', 'active', 'completed', 'skipped')),
    CONSTRAINT workflow_steps_decision_check CHECK (decision IS NULL OR decision IN ('approved', 'rejected', 'request_changes'))
);

-- インデックス
CREATE INDEX workflow_steps_instance_idx ON workflow_steps(instance_id);
CREATE INDEX workflow_steps_assigned_to_idx ON workflow_steps(assigned_to) WHERE status = 'active';

-- updated_at 自動更新トリガー
CREATE TRIGGER workflow_steps_updated_at
    BEFORE UPDATE ON workflow_steps
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON TABLE workflow_steps IS 'ワークフローステップ（承認タスク）';
COMMENT ON COLUMN workflow_steps.id IS '主キー';
COMMENT ON COLUMN workflow_steps.instance_id IS 'インスタンスID（FK）';
COMMENT ON COLUMN workflow_steps.step_id IS '定義上のステップID';
COMMENT ON COLUMN workflow_steps.step_name IS 'ステップ名';
COMMENT ON COLUMN workflow_steps.step_type IS 'ステップ種別（approval/notification/...）';
COMMENT ON COLUMN workflow_steps.status IS '状態（pending/active/completed/skipped）';
COMMENT ON COLUMN workflow_steps.assigned_to IS '担当者（FK）';
COMMENT ON COLUMN workflow_steps.decision IS '判断（approved/rejected/request_changes）';
COMMENT ON COLUMN workflow_steps.comment IS 'コメント';
COMMENT ON COLUMN workflow_steps.due_date IS '期限';
COMMENT ON COLUMN workflow_steps.started_at IS '開始日時';
COMMENT ON COLUMN workflow_steps.completed_at IS '完了日時';
