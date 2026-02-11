/**
 * 承認フロー E2E テスト
 *
 * 申請を作成し、承認者として承認操作を行い、
 * ステータスが正しく更新されることを検証する。
 *
 * テスト戦略: ログインユーザー（管理者）が自身を承認者に指定して
 * 申請を作成し、同じユーザーでタスクを承認する。
 */

import { type Page, expect, test } from "@playwright/test";

/**
 * テスト用の申請を作成して送信するヘルパー。
 * ログインユーザー（管理者）自身を承認者に指定する。
 */
async function createAndSubmitWorkflow(
  page: Page,
  title: string,
): Promise<void> {
  await page.goto("/workflows/new");
  await page.getByText("汎用申請").click();
  await page.getByPlaceholder("申請のタイトルを入力").fill(title);

  const dynamicForm = page.locator(".bg-secondary-50");
  await dynamicForm.locator("input[name='title']").fill("テスト件名");
  await dynamicForm.getByLabel("内容").fill("テスト内容");

  // ログインユーザー（管理者）自身を承認者に選択
  await page.locator("#approver-search").fill("管理者");
  await page.locator("li").filter({ hasText: "管理者" }).click();

  await page.getByRole("button", { name: "申請する" }).click();
  await expect(page.getByText("申請が完了しました")).toBeVisible();
}

test.describe("承認フロー", () => {
  test("申請を作成すると承認者のタスク一覧に表示される", async ({ page }) => {
    const uniqueTitle = `承認テスト ${Date.now()}`;

    await createAndSubmitWorkflow(page, uniqueTitle);

    // タスク一覧に移動
    await page.goto("/tasks");
    await expect(
      page.getByRole("heading", { name: "タスク一覧" }),
    ).toBeVisible();

    // 作成した申請のタスクが表示される
    await expect(page.getByText(uniqueTitle)).toBeVisible();
  });

  test("タスク詳細から承認するとステータスが更新される", async ({ page }) => {
    const uniqueTitle = `承認完了テスト ${Date.now()}`;

    await createAndSubmitWorkflow(page, uniqueTitle);

    // タスク一覧に移動して該当タスクの詳細を開く
    await page.goto("/tasks");
    const taskRow = page.locator("tr").filter({ hasText: uniqueTitle });
    await taskRow.getByRole("link").first().click();

    // 承認ボタンをクリック（exact: true で確認ダイアログの「承認する」と区別）
    await page.getByRole("button", { name: "承認", exact: true }).click();

    // 確認ダイアログで「承認する」をクリック
    await page.getByRole("button", { name: "承認する" }).click();

    // 成功メッセージが表示される
    await expect(page.getByText("承認しました")).toBeVisible();

    // 申請一覧でステータスが「承認済み」に更新されたことを確認
    await page.goto("/workflows");
    const workflowRow = page.locator("tr").filter({ hasText: uniqueTitle });
    await expect(workflowRow.getByText("承認済み")).toBeVisible();
  });
});
