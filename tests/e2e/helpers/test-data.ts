/**
 * E2E テスト用定数
 *
 * シードデータ（backend/migrations/20260115000008_seed_system_data.sql）に対応する。
 */

/** 開発用テナント ID */
export const TENANT_ID = "00000000-0000-0000-0000-000000000001";

/** 管理者ユーザー（tenant_admin ロール） */
export const ADMIN_USER = {
  email: "admin@example.com",
  password: "password123",
  id: "00000000-0000-0000-0000-000000000001",
  name: "管理者",
} as const;

/** 一般ユーザー（user ロール） */
export const REGULAR_USER = {
  email: "user@example.com",
  password: "password123",
  id: "00000000-0000-0000-0000-000000000002",
  name: "一般ユーザー",
} as const;

/** ワークフロー定義（シードデータ: 汎用申請） */
export const WORKFLOW_DEFINITION_ID =
  "00000000-0000-0000-0000-000000000001";

/** ワークフロー定義（シードデータ: 2段階承認申請） */
export const MULTI_STEP_DEFINITION_ID =
  "00000000-0000-0000-0000-000000000002";

/** 認証 storageState ファイルパス */
export const ADMIN_AUTH_FILE = "tests/.auth/admin.json";
export const USER_AUTH_FILE = "tests/.auth/user.json";
