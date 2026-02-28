-- users.password_hash カラムの削除
-- 構文リファレンス: README.md
--
-- Auth Service 分離（#80）の最終フェーズ。
-- パスワードハッシュは auth.credentials テーブルに移行済み。
-- 詳細: docs/90_実装解説/04_AuthService/01_AuthService_機能解説.md

ALTER TABLE users DROP COLUMN password_hash;

-- コメント更新
COMMENT ON TABLE users IS 'ユーザー情報（認証情報は auth.credentials で管理）';
