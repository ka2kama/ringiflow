-- user_roles テーブルに tenant_id カラムを追加
-- 構文リファレンス: README.md
--
-- RLS（Row Level Security）による二重防御のため、
-- users/roles 経由ではなく直接 tenant_id を持たせる（非正規化）。
-- JOIN ベースの RLS ポリシーは PostgreSQL 公式ドキュメントで非推奨。
--
-- 注: system roles（is_system = true）は tenant_id IS NULL だが、
-- user_roles はテナントユーザーへの割り当てなので必ず tenant_id を持つ。

-- 1. nullable で追加
ALTER TABLE user_roles
    ADD COLUMN tenant_id UUID;

-- 2. 既存データをバックフィル（users から tenant_id を取得）
UPDATE user_roles ur
SET tenant_id = u.tenant_id
FROM users u
WHERE ur.user_id = u.id;

-- 3. NOT NULL 制約を設定
ALTER TABLE user_roles
    ALTER COLUMN tenant_id SET NOT NULL;

-- 4. 外部キー制約を追加
ALTER TABLE user_roles
    ADD CONSTRAINT user_roles_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;

-- 5. RLS 用インデックス（tenant_id での絞り込みを高速化）
CREATE INDEX user_roles_tenant_id_idx ON user_roles(tenant_id);

-- コメント
COMMENT ON COLUMN user_roles.tenant_id IS 'テナントID（FK、RLS 二重防御用）';
