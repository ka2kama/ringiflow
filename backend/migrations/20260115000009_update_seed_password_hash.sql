-- 開発用ユーザーのパスワードハッシュを更新
-- パスワード: password123 (開発環境専用、パスワード要件: 英字+数字)
-- アルゴリズム: Argon2id (RFC 9106 推奨パラメータ)

UPDATE users
SET password_hash = '$argon2id$v=19$m=65536,t=1,p=1$olntqw+EoVpwH4B1vUAI0A$5yCA1izLODgz8nQOInDGwbuQB/AS0sIQDwpmIilve5M'
WHERE id IN (
    '00000000-0000-0000-0000-000000000001',
    '00000000-0000-0000-0000-000000000002'
);
