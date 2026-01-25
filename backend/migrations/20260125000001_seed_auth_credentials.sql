-- 開発用ユーザーの認証情報を auth.credentials に追加
-- 構文リファレンス: README.md
--
-- E2E テストで実際のログインフローをテストするために必要。
-- パスワード: password123（開発環境専用）
--
-- 関連: #98 E2E API テストを hurl で追加する

INSERT INTO auth.credentials (id, user_id, tenant_id, credential_type, credential_data, is_active)
VALUES
    -- admin@example.com
    ('00000000-0000-0000-0000-000000000011',
     '00000000-0000-0000-0000-000000000001',
     '00000000-0000-0000-0000-000000000001',
     'password',
     '$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M',
     true),
    -- user@example.com
    ('00000000-0000-0000-0000-000000000012',
     '00000000-0000-0000-0000-000000000002',
     '00000000-0000-0000-0000-000000000001',
     'password',
     '$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M',
     true);

-- コメント
COMMENT ON TABLE auth.credentials IS 'シードデータ: admin@example.com, user@example.com のパスワード認証情報（password123）';
