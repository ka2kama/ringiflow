-- user_roles テーブルの作成
-- 構文リファレンス: README.md
--
-- ユーザーとロールの関連を管理する（多対多）。

CREATE TABLE user_roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CONSTRAINT user_roles_user_role_key UNIQUE (user_id, role_id)
);

-- インデックス
CREATE INDEX user_roles_user_idx ON user_roles(user_id);
CREATE INDEX user_roles_role_idx ON user_roles(role_id);

-- コメント
COMMENT ON TABLE user_roles IS 'ユーザーとロールの関連';
COMMENT ON COLUMN user_roles.id IS '主キー';
COMMENT ON COLUMN user_roles.user_id IS 'ユーザーID（FK）';
COMMENT ON COLUMN user_roles.role_id IS 'ロールID（FK）';
