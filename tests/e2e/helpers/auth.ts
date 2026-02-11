/**
 * 認証ヘルパー
 *
 * Playwright 推奨パターン: API ログインで Cookie を取得し、storageState に保存する。
 * 参考: https://playwright.dev/docs/auth
 */

import { type APIRequestContext } from "@playwright/test";
import { TENANT_ID } from "./test-data";

/**
 * API 経由でログインし、セッション Cookie を取得する。
 *
 * BFF の POST /api/v1/auth/login を呼び出す。
 * レスポンスの Set-Cookie で session_id が設定され、
 * Playwright の APIRequestContext に自動保存される。
 */
export async function login(
  request: APIRequestContext,
  email: string,
  password: string,
): Promise<void> {
  const response = await request.post("/api/v1/auth/login", {
    headers: {
      "X-Tenant-ID": TENANT_ID,
    },
    data: { email, password },
  });

  if (!response.ok()) {
    throw new Error(
      `Login failed: ${response.status()} ${await response.text()}`,
    );
  }
}
