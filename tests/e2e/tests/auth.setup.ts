/**
 * 認証セットアップ
 *
 * テスト実行前に API ログインを行い、セッション Cookie を storageState に保存する。
 * 他のテストプロジェクト（chromium）はこの storageState を再利用し、
 * 認証済み状態でテストを実行する。
 *
 * 参考: https://playwright.dev/docs/auth
 */

import { test as setup } from "@playwright/test";
import { login } from "../helpers/auth";
import { ADMIN_USER } from "../helpers/test-data";

const authFile = "tests/.auth/admin.json";

setup("管理者ユーザーでログインする", async ({ request }) => {
  await login(request, ADMIN_USER.email, ADMIN_USER.password);
  await request.storageState({ path: authFile });
});
