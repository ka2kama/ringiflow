-- RingiFlow PostgreSQL 初期化スクリプト
-- docker-entrypoint-initdb.d で自動実行される

-- UUID 生成（gen_random_uuid() を使用可能にする）
-- Aurora PostgreSQL でも利用可能
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 暗号化関数（パスワードハッシュ等で使用）
CREATE EXTENSION IF NOT EXISTS "pgcrypto";
