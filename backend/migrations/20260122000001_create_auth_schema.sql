-- auth スキーマと credentials テーブルの作成
-- 構文リファレンス: README.md
--
-- Auth Service が所有するスキーマ。
-- 認証情報（パスワード、将来の TOTP/OIDC/SAML）を管理する。
--
-- 設計詳細: docs/40_詳細設計書/08_AuthService設計.md

-- auth スキーマの作成
CREATE SCHEMA IF NOT EXISTS auth;

-- credentials テーブルの作成
CREATE TABLE auth.credentials (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    tenant_id UUID NOT NULL,
    credential_type VARCHAR(20) NOT NULL,
    credential_data TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 同一ユーザー・同一種別の認証情報は一意
    CONSTRAINT uq_credentials_user_type UNIQUE (user_id, credential_type),

    -- credential_type の値を制限
    CONSTRAINT chk_credential_type CHECK (credential_type IN ('password', 'totp', 'oidc', 'saml'))
);

-- インデックス
CREATE INDEX idx_credentials_user_id ON auth.credentials(user_id);
CREATE INDEX idx_credentials_tenant_id ON auth.credentials(tenant_id);

-- updated_at 自動更新トリガー
CREATE TRIGGER credentials_updated_at
    BEFORE UPDATE ON auth.credentials
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();

-- コメント
COMMENT ON SCHEMA auth IS 'Auth Service が所有するスキーマ。認証情報を管理。';
COMMENT ON TABLE auth.credentials IS '認証情報。テナント退会時は tenant_id で削除。';
COMMENT ON COLUMN auth.credentials.id IS '主キー';
COMMENT ON COLUMN auth.credentials.user_id IS 'ユーザーID（外部キー制約なし、サービス境界の独立性のため）';
COMMENT ON COLUMN auth.credentials.tenant_id IS 'テナントID（テナント退会時の削除に使用）';
COMMENT ON COLUMN auth.credentials.credential_type IS '認証種別: password, totp, oidc, saml';
COMMENT ON COLUMN auth.credentials.credential_data IS '認証データ（パスワードハッシュ等）';
COMMENT ON COLUMN auth.credentials.is_active IS '有効フラグ';
COMMENT ON COLUMN auth.credentials.last_used_at IS '最終使用日時';
