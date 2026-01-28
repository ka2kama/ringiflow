-- workflow_instances と workflow_steps に version カラムを追加
-- 楽観的ロックによる並行更新の競合検出に使用
--
-- 参照: docs/03_詳細設計書/11_ワークフロー承認却下機能設計.md

-- workflow_instances に version カラムを追加
ALTER TABLE workflow_instances
ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

COMMENT ON COLUMN workflow_instances.version IS '楽観的ロック用バージョン番号';

-- workflow_steps に version カラムを追加
ALTER TABLE workflow_steps
ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

COMMENT ON COLUMN workflow_steps.version IS '楽観的ロック用バージョン番号';
