-- users.password_hash から auth.credentials への移行
-- 構文リファレンス: README.md
--
-- 既存のパスワードハッシュを credentials テーブルに移行する。
-- 移行期間中は両方のテーブルにデータを保持する。
--
-- 設計詳細: docs/03_詳細設計書/08_AuthService設計.md

-- 既存のパスワードハッシュを credentials に移行
INSERT INTO auth.credentials (user_id, tenant_id, credential_type, credential_data, is_active)
SELECT
    id AS user_id,
    tenant_id,
    'password' AS credential_type,
    password_hash AS credential_data,
    true AS is_active
FROM users
WHERE password_hash IS NOT NULL
  AND password_hash != '$INVALID_HASH_PLEASE_SET_PASSWORD$';


-- コメント更新
COMMENT ON TABLE users IS 'ユーザー情報。パスワードハッシュは auth.credentials にも保存（移行期間中）。';
