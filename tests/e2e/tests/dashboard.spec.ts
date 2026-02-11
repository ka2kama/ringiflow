/**
 * ダッシュボード E2E テスト
 *
 * 認証済みユーザーがダッシュボードにアクセスし、
 * KPI 統計情報が表示されることを検証する。
 */

import { expect, test } from "@playwright/test";

test.describe("ダッシュボード", () => {
  test("認証済みユーザーがダッシュボードを表示できる", async ({ page }) => {
    await page.goto("/");

    // ダッシュボードの見出しが表示される
    await expect(page.getByRole("heading", { name: "ダッシュボード" })).toBeVisible();
  });

  test("KPI 統計カードが表示される", async ({ page }) => {
    await page.goto("/");

    // 3つの KPI カードが表示される
    await expect(page.getByText("承認待ちタスク")).toBeVisible();
    await expect(page.getByText("申請中")).toBeVisible();
    await expect(page.getByText("本日完了")).toBeVisible();
  });

  test("クイックアクションが表示される", async ({ page }) => {
    await page.goto("/");

    // メインコンテンツ内のクイックアクションリンクが表示される
    // サイドバーにも同名リンクがあるため、メインコンテンツにスコープする
    const main = page.locator("#main-content");
    await expect(main.getByRole("link", { name: "申請一覧" })).toBeVisible();
    await expect(main.getByRole("link", { name: "新規申請" })).toBeVisible();
    await expect(main.getByRole("link", { name: "タスク一覧" })).toBeVisible();
  });
});
