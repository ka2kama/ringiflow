-- tenants テーブルの作成
-- 構文リファレンス: README.md
--
-- テナント（組織）情報を管理する。
-- マルチテナント対応の基盤となるテーブル。

-- updated_at 自動更新用のトリガー関数
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- tenants テーブル
CREATE TABLE tenants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    subdomain VARCHAR(63) NOT NULL UNIQUE,
    plan VARCHAR(50) NOT NULL DEFAULT 'free',
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    settings JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT tenants_plan_check CHECK (plan IN ('free', 'standard', 'professional', 'enterprise')),
    CONSTRAINT tenants_status_check CHECK (status IN ('active', 'suspended', 'deleted'))
);

-- updated_at 自動更新トリガー
CREATE TRIGGER tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON TABLE tenants IS 'テナント（組織）情報';
COMMENT ON COLUMN tenants.id IS '主キー';
COMMENT ON COLUMN tenants.name IS 'テナント名';
COMMENT ON COLUMN tenants.subdomain IS 'サブドメイン（ユニーク）';
COMMENT ON COLUMN tenants.plan IS 'プラン（free/standard/professional/enterprise）';
COMMENT ON COLUMN tenants.status IS '状態（active/suspended/deleted）';
COMMENT ON COLUMN tenants.settings IS 'テナント設定（JSON）';
