-- display_number を NOT NULL に変更
-- データ移行完了後に適用する
--
-- 前提: 20260205000001 で全既存データに display_number が割り当て済み

ALTER TABLE workflow_steps
    ALTER COLUMN display_number SET NOT NULL;
