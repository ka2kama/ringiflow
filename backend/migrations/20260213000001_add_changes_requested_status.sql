-- ワークフローインスタンスに「要修正（changes_requested）」ステータスを追加する。
-- 承認者が差し戻し、申請者が修正して再申請するフローに対応。

-- CHECK 制約を更新（changes_requested を追加）
ALTER TABLE workflow_instances
    DROP CONSTRAINT workflow_instances_status_check,
    ADD CONSTRAINT workflow_instances_status_check CHECK (
        status IN ('draft', 'pending', 'in_progress', 'approved', 'rejected', 'cancelled', 'changes_requested')
    );

-- カラムコメントを更新
COMMENT ON COLUMN workflow_instances.status IS '状態（draft/pending/in_progress/approved/rejected/cancelled/changes_requested）';
