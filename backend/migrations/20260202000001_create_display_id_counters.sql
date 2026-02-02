-- 表示用 ID の採番カウンターテーブル
-- テナント × エンティティ種別ごとに最後に採番した番号を管理する
--
-- 参照: docs/03_詳細設計書/12_表示用ID設計.md

CREATE TABLE display_id_counters (
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    entity_type VARCHAR(50) NOT NULL,
    last_number BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, entity_type),
    CONSTRAINT chk_last_number_non_negative CHECK (last_number >= 0)
);

COMMENT ON TABLE display_id_counters IS '表示用 ID の採番カウンター';
COMMENT ON COLUMN display_id_counters.tenant_id IS 'テナント ID（FK）';
COMMENT ON COLUMN display_id_counters.entity_type IS 'エンティティ種別（workflow_instance, workflow_step）';
COMMENT ON COLUMN display_id_counters.last_number IS '最後に採番した番号（0 は未採番）';
