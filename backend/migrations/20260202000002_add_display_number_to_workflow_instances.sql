-- workflow_instances に表示用連番カラムを追加し、既存データをマイグレーション
--
-- 参照: docs/03_詳細設計書/12_表示用ID設計.md

-- 1. NULLABLE でカラムを追加
ALTER TABLE workflow_instances
    ADD COLUMN display_number BIGINT;

COMMENT ON COLUMN workflow_instances.display_number IS '表示用連番（テナント内で一意）';

-- 2. テナント内でのユニーク制約（NULL は対象外）
CREATE UNIQUE INDEX idx_workflow_instances_display_number
    ON workflow_instances (tenant_id, display_number)
    WHERE display_number IS NOT NULL;

-- 3. 既存データへの display_number 割り当て（created_at 順、同一時刻は id 順）
WITH numbered AS (
    SELECT id, tenant_id,
           ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY created_at, id) AS rn
    FROM workflow_instances
)
UPDATE workflow_instances wi
SET display_number = n.rn
FROM numbered n
WHERE wi.id = n.id;

-- 4. カウンターテーブルの初期化（既存データの件数で last_number を設定）
INSERT INTO display_id_counters (tenant_id, entity_type, last_number)
SELECT tenant_id, 'workflow_instance', COUNT(*)
FROM workflow_instances
GROUP BY tenant_id
ON CONFLICT (tenant_id, entity_type)
DO UPDATE SET last_number = EXCLUDED.last_number;
