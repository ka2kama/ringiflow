/**
 * 申請フロー E2E テスト
 *
 * 認証済みユーザーが新規申請を作成し、
 * 申請一覧に反映されることを検証する。
 */

import { expect, test } from "@playwright/test";

test.describe("申請フロー", () => {
  test("新規申請ページにアクセスできる", async ({ page }) => {
    await page.goto("/workflows/new");

    // 新規申請の見出しが表示される
    await expect(
      page.getByRole("heading", { name: "新規申請" }),
    ).toBeVisible();
  });

  test("申請フォームに入力して送信すると申請一覧に反映される", async ({
    page,
  }) => {
    const uniqueTitle = `E2E テスト申請 ${Date.now()}`;

    // 新規申請ページに移動
    await page.goto("/workflows/new");

    // Step 1: ワークフロー定義を選択
    await page.getByText("汎用申請").click();

    // Step 2: フォーム入力
    // タイトル（placeholder で特定し、動的フォームの "件名" との id="title" 重複を回避）
    await page.getByPlaceholder("申請のタイトルを入力").fill(uniqueTitle);

    // 動的フォームフィールド（bg-secondary-50 コンテナ内にスコープ）
    const dynamicForm = page.locator(".bg-secondary-50");
    await dynamicForm.locator("input[name='title']").fill("E2E テスト件名");
    await dynamicForm.getByLabel("内容").fill("E2E テスト内容です");

    // Step 3: 承認者選択
    await page.locator("#approver-search").fill("一般");
    // ドロップダウンが表示されるのを待ってから候補を選択
    await page
      .locator("li")
      .filter({ hasText: "一般ユーザー" })
      .click();

    // 申請する
    await page.getByRole("button", { name: "申請する" }).click();

    // 成功メッセージが表示される
    await expect(page.getByText("申請が完了しました")).toBeVisible();

    // 申請一覧に移動して、作成した申請が表示されることを確認
    await page.goto("/workflows");
    await expect(page.getByText(uniqueTitle)).toBeVisible();
  });
});
