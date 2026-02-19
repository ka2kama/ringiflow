import { defineConfig, devices } from "@playwright/test";
import { ADMIN_AUTH_FILE } from "./helpers/test-data";

/**
 * Playwright 設定
 *
 * ローカル: `just dev-all` で起動した開発サーバーに対してテストを実行する。
 * CI: `scripts/test/run-e2e.sh` でテスト用環境を構築してから実行する。
 */
export default defineConfig({
  testDir: "./tests",

  /* テスト実行設定 */
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 1 : undefined,

  /* レポーター */
  reporter: process.env.CI ? "github" : "html",

  use: {
    /* Vite dev server（フロントエンド） */
    baseURL: process.env.E2E_BASE_URL ?? "http://localhost:15173",

    /* テスト失敗時にスクリーンショットを取得 */
    screenshot: "only-on-failure",
    trace: "on-first-retry",
  },

  projects: [
    /* 認証セットアップ: API ログインで Cookie を取得 */
    {
      name: "setup",
      testMatch: /.*\.setup\.ts/,
    },

    /* メインテスト: Chromium のみ（MVP） */
    {
      name: "chromium",
      use: {
        ...devices["Desktop Chrome"],
        storageState: ADMIN_AUTH_FILE,
      },
      dependencies: ["setup"],
    },
  ],
});
