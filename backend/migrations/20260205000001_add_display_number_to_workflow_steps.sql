-- workflow_steps に表示用連番カラムを追加し、既存データをマイグレーション
--
-- 参照: docs/03_詳細設計書/12_表示用ID設計.md#workflow_steps-テーブルへのカラム追加phase-b
--
-- 設計判断:
--   - 採番スコープ: テナント単位（display_id_counters の entity_type = 'workflow_step'）
--   - ユニーク制約: (instance_id, display_number) — インスタンス内で一意

-- 1. NULLABLE でカラムを追加
ALTER TABLE workflow_steps
    ADD COLUMN display_number BIGINT;

COMMENT ON COLUMN workflow_steps.display_number IS '表示用連番（インスタンス内で一意）';

-- 2. インスタンス内でのユニーク制約（NULL は対象外）
CREATE UNIQUE INDEX idx_workflow_steps_display_number
    ON workflow_steps (instance_id, display_number)
    WHERE display_number IS NOT NULL;

-- 3. 既存データへの display_number 割り当て（各インスタンス内で created_at 順、同一時刻は id 順）
-- 注: インスタンス単位でリセットせず、テナント全体で通し番号を採番するが、
--     既存データマイグレーションではインスタンス単位で 1 から振り直す
WITH numbered AS (
    SELECT id, instance_id,
           ROW_NUMBER() OVER (PARTITION BY instance_id ORDER BY created_at, id) AS rn
    FROM workflow_steps
)
UPDATE workflow_steps ws
SET display_number = n.rn
FROM numbered n
WHERE ws.id = n.id;

-- 4. カウンターテーブルの初期化（テナント単位で全ステップの件数を設定）
-- workflow_steps は tenant_id を直接持たないため、workflow_instances 経由で取得
INSERT INTO display_id_counters (tenant_id, entity_type, last_number)
SELECT wi.tenant_id, 'workflow_step', COUNT(ws.id)
FROM workflow_instances wi
LEFT JOIN workflow_steps ws ON wi.id = ws.instance_id
GROUP BY wi.tenant_id
ON CONFLICT (tenant_id, entity_type)
DO UPDATE SET last_number = EXCLUDED.last_number;
