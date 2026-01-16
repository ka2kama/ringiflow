-- users テーブルの作成
--
-- ユーザー情報を管理する。
-- テナントに紐付き、メール/パスワード認証またはSSO認証に対応。

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255),
    status VARCHAR(20) NOT NULL DEFAULT 'active',
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT users_tenant_email_key UNIQUE (tenant_id, email),
    CONSTRAINT users_status_check CHECK (status IN ('active', 'inactive', 'deleted'))
);

-- インデックス
CREATE INDEX users_tenant_status_idx ON users(tenant_id, status);

-- updated_at 自動更新トリガー
CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON TABLE users IS 'ユーザー情報';
COMMENT ON COLUMN users.id IS '主キー';
COMMENT ON COLUMN users.tenant_id IS 'テナントID（FK）';
COMMENT ON COLUMN users.email IS 'メールアドレス（テナント内でユニーク）';
COMMENT ON COLUMN users.name IS '表示名';
COMMENT ON COLUMN users.password_hash IS 'パスワードハッシュ（SSO の場合 NULL）';
COMMENT ON COLUMN users.status IS '状態（active/inactive/deleted）';
COMMENT ON COLUMN users.last_login_at IS '最終ログイン日時';
