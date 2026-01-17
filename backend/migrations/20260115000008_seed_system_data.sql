-- 初期データのシード
-- 構文リファレンス: README.md
--
-- システムロール、開発用テナント、開発用ユーザーを作成する。

-- システムロールの作成
INSERT INTO roles (id, tenant_id, name, description, permissions, is_system) VALUES
    ('00000000-0000-0000-0000-000000000001', NULL, 'system_admin', 'システム管理者', '["*"]', true),
    ('00000000-0000-0000-0000-000000000002', NULL, 'tenant_admin', 'テナント管理者', '["tenant:*", "user:*", "workflow:*", "task:*"]', true),
    ('00000000-0000-0000-0000-000000000003', NULL, 'user', '一般ユーザー', '["workflow:read", "workflow:create", "task:read", "task:update"]', true);

-- 開発用テナントの作成
INSERT INTO tenants (id, name, subdomain, plan, status) VALUES
    ('00000000-0000-0000-0000-000000000001', 'Development Tenant', 'dev', 'enterprise', 'active');

-- 開発用ユーザーの作成
-- TODO: 初回起動時に `just seed-passwords` でパスワードを設定する必要あり
-- セキュリティ: 以下のハッシュは無効なダミー値（ログイン不可を保証）
INSERT INTO users (id, tenant_id, email, name, password_hash, status) VALUES
    ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001',
     'admin@example.com', '管理者', '$INVALID_HASH_PLEASE_SET_PASSWORD$', 'active'),
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000001',
     'user@example.com', '一般ユーザー', '$INVALID_HASH_PLEASE_SET_PASSWORD$', 'active');

-- ロール割り当て
INSERT INTO user_roles (user_id, role_id) VALUES
    ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000002'), -- admin: tenant_admin
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000003'); -- user: user

-- MVP 用ワークフロー定義
INSERT INTO workflow_definitions (id, tenant_id, name, description, version, definition, status, created_by) VALUES
    ('00000000-0000-0000-0000-000000000001',
     '00000000-0000-0000-0000-000000000001',
     '汎用申請',
     'シンプルな1段階承認ワークフロー',
     1,
     '{
       "form": {
         "fields": [
           {"id": "title", "type": "text", "label": "件名", "required": true, "maxLength": 100},
           {"id": "description", "type": "textarea", "label": "内容", "required": true, "maxLength": 2000}
         ]
       },
       "steps": [
         {"id": "start", "type": "start", "name": "開始"},
         {"id": "approval", "type": "approval", "name": "承認", "assignee": {"type": "user"}},
         {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
         {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
       ],
       "transitions": [
         {"from": "start", "to": "approval"},
         {"from": "approval", "to": "end_approved", "trigger": "approve"},
         {"from": "approval", "to": "end_rejected", "trigger": "reject"}
       ]
     }',
     'published',
     '00000000-0000-0000-0000-000000000001');

-- コメント
COMMENT ON TABLE roles IS 'システムロール: system_admin, tenant_admin, user が定義されている';
COMMENT ON TABLE tenants IS '開発用テナント (subdomain=dev) が定義されている';
COMMENT ON TABLE users IS '開発用ユーザー admin@example.com, user@example.com (パスワード: password) が定義されている';
COMMENT ON TABLE workflow_definitions IS 'MVP用の汎用申請ワークフローが定義されている';
