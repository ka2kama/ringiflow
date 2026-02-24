-- シードデータの current_step_id 修正（Issue #819）
--
-- ADT ベースステートマシン導入に伴い、InProgress/Approved/Rejected 状態の
-- インスタンスには current_step_id が必須となった（INV-I3, I7）。
-- 既存シードデータに current_step_id を設定する。

-- ============================================================
-- 1. 初期シード（20260128000001）の修正
-- ============================================================

-- InProgress インスタンス
UPDATE workflow_instances
SET current_step_id = 'approval'
WHERE id = '33333333-3333-3333-3333-333333333333';

-- Approved インスタンス（completed_at も未設定だったため追加）
UPDATE workflow_instances
SET current_step_id = 'approval',
    completed_at = '2026-01-12 15:00:00+09'
WHERE id = '44444444-4444-4444-4444-444444444444';

-- Rejected インスタンス（completed_at も未設定だったため追加）
UPDATE workflow_instances
SET current_step_id = 'approval',
    completed_at = '2026-01-11 16:00:00+09'
WHERE id = '55555555-5555-5555-5555-555555555555';

-- ============================================================
-- 2. 追加シード（20260219000001）の修正
-- ============================================================

-- InProgress インスタンス
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000008';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000009';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-00000000000a';
UPDATE workflow_instances SET current_step_id = 'finance_approval' WHERE id = 'a0000000-0000-0000-0000-00000000000b';  -- 2段階承認: 経理承認待ち
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-00000000000c';

-- Approved インスタンス
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-00000000000d';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-00000000000e';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-00000000000f';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000010';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000011';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000012';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000013';
UPDATE workflow_instances SET current_step_id = 'finance_approval' WHERE id = 'a0000000-0000-0000-0000-000000000014';  -- 2段階承認: 最終ステップ
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000015';

-- Rejected インスタンス
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000016';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000017';
UPDATE workflow_instances SET current_step_id = 'approval' WHERE id = 'a0000000-0000-0000-0000-000000000018';
UPDATE workflow_instances SET current_step_id = 'manager_approval' WHERE id = 'a0000000-0000-0000-0000-000000000019';  -- 2段階承認: 上長承認で却下
