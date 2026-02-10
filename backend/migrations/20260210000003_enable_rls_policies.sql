-- RLS（Row Level Security）の有効化とポリシー作成
-- 構文リファレンス: README.md
--
-- 基本設計書 7.1.3 節「二重防御」の DB 層を実装する。
-- アプリ層（リポジトリの WHERE tenant_id = $1）に加え、
-- DB 層でもテナント分離を強制する。
--
-- GUC（Grand Unified Configuration）変数 app.tenant_id を使用:
-- - set_config('app.tenant_id', $1, false) でセッション単位で設定
-- - current_setting('app.tenant_id', true) で取得（missing_ok = true）
-- - NULLIF で空文字列を NULL に変換し、UUID キャスト失敗を防止
-- - tenant_id = NULL は常に false → 未設定時はどの行もマッチしない

-- =============================================================================
-- アプリケーション用 DB ロール
-- =============================================================================
-- superuser は BYPASSRLS で RLS を無視する。
-- アプリケーションが使う非 superuser ロールを作成する。
-- 現時点ではテスト用。本番デプロイ時にアプリケーション接続をこのロールに切り替える。

DO $$
BEGIN
    IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'ringiflow_app') THEN
        CREATE ROLE ringiflow_app LOGIN PASSWORD 'ringiflow_app';
    END IF;
END
$$;

-- ringiflow_app に必要な権限を付与
GRANT USAGE ON SCHEMA public TO ringiflow_app;
GRANT USAGE ON SCHEMA auth TO ringiflow_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO ringiflow_app;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA auth TO ringiflow_app;

-- 今後作成されるテーブルにも自動で権限付与
ALTER DEFAULT PRIVILEGES IN SCHEMA public
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ringiflow_app;
ALTER DEFAULT PRIVILEGES IN SCHEMA auth
    GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO ringiflow_app;

-- =============================================================================
-- RLS 有効化
-- =============================================================================

-- public スキーマ
ALTER TABLE tenants ENABLE ROW LEVEL SECURITY;
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE roles ENABLE ROW LEVEL SECURITY;
ALTER TABLE user_roles ENABLE ROW LEVEL SECURITY;
ALTER TABLE workflow_definitions ENABLE ROW LEVEL SECURITY;
ALTER TABLE workflow_instances ENABLE ROW LEVEL SECURITY;
ALTER TABLE workflow_steps ENABLE ROW LEVEL SECURITY;
ALTER TABLE display_id_counters ENABLE ROW LEVEL SECURITY;

-- auth スキーマ
ALTER TABLE auth.credentials ENABLE ROW LEVEL SECURITY;

-- =============================================================================
-- RLS ポリシー作成
-- =============================================================================
-- USING 句: SELECT, UPDATE, DELETE に適用（行の可視性）
-- WITH CHECK 句: INSERT, UPDATE に適用（行の書き込み可否）
-- FOR ALL = SELECT + INSERT + UPDATE + DELETE

-- tenants: id で照合（自テナントのみ）
CREATE POLICY tenant_isolation ON tenants
    FOR ALL
    TO ringiflow_app
    USING (id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- users: tenant_id で照合
CREATE POLICY tenant_isolation ON users
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- roles: tenant_id で照合 + system roles（tenant_id IS NULL）は全テナントから参照可能
CREATE POLICY tenant_isolation ON roles
    FOR ALL
    TO ringiflow_app
    USING (
        tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID
        OR tenant_id IS NULL
    )
    WITH CHECK (
        tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID
        OR tenant_id IS NULL
    );

-- user_roles: tenant_id で照合
CREATE POLICY tenant_isolation ON user_roles
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- workflow_definitions: tenant_id で照合
CREATE POLICY tenant_isolation ON workflow_definitions
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- workflow_instances: tenant_id で照合
CREATE POLICY tenant_isolation ON workflow_instances
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- workflow_steps: tenant_id で照合
CREATE POLICY tenant_isolation ON workflow_steps
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- display_id_counters: tenant_id で照合
CREATE POLICY tenant_isolation ON display_id_counters
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- auth.credentials: tenant_id で照合
CREATE POLICY tenant_isolation ON auth.credentials
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);
