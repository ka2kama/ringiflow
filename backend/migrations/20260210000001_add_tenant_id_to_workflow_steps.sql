-- workflow_steps テーブルに tenant_id カラムを追加
-- 構文リファレンス: README.md
--
-- RLS（Row Level Security）による二重防御のため、
-- workflow_instances 経由ではなく直接 tenant_id を持たせる（非正規化）。
-- JOIN ベースの RLS ポリシーは PostgreSQL 公式ドキュメントで非推奨。

-- 1. nullable で追加
ALTER TABLE workflow_steps
    ADD COLUMN tenant_id UUID;

-- 2. 既存データをバックフィル（workflow_instances から tenant_id を取得）
UPDATE workflow_steps ws
SET tenant_id = wi.tenant_id
FROM workflow_instances wi
WHERE ws.instance_id = wi.id;

-- 3. NOT NULL 制約を設定
ALTER TABLE workflow_steps
    ALTER COLUMN tenant_id SET NOT NULL;

-- 4. 外部キー制約を追加
ALTER TABLE workflow_steps
    ADD CONSTRAINT workflow_steps_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- 5. RLS 用インデックス（tenant_id での絞り込みを高速化）
CREATE INDEX workflow_steps_tenant_id_idx ON workflow_steps(tenant_id);

-- コメント
COMMENT ON COLUMN workflow_steps.tenant_id IS 'テナントID（FK、RLS 二重防御用）';
