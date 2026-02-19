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
import {
  ADMIN_AUTH_FILE,
  ADMIN_USER,
  REGULAR_USER,
  USER_AUTH_FILE,
} from "../helpers/test-data";

setup("管理者ユーザーでログインする", async ({ request }) => {
  await login(request, ADMIN_USER.email, ADMIN_USER.password);
  await request.storageState({ path: ADMIN_AUTH_FILE });
});

setup("一般ユーザーでログインする", async ({ request }) => {
  await login(request, REGULAR_USER.email, REGULAR_USER.password);
  await request.storageState({ path: USER_AUTH_FILE });
});
