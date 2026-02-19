/**
 * 承認フロー E2E テスト（E2E-004）
 *
 * 申請を作成し、承認者として承認操作を行い、
 * ステータスが正しく更新されることを検証する。
 */

import { expect, test } from "@playwright/test";
import { ADMIN_USER, REGULAR_USER, USER_AUTH_FILE } from "../helpers/test-data";
import {
  approveTask,
  createAndSubmitMultiStepWorkflow,
  createAndSubmitWorkflow,
  openTaskDetail,
  verifyWorkflowStatus,
} from "../helpers/workflow";

test.describe("承認フロー", () => {
  test("申請を作成すると承認者のタスク一覧に表示される", async ({ page }) => {
    const uniqueTitle = `承認テスト ${Date.now()}`;

    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);

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

    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);

    // タスク一覧に移動して該当タスクの詳細を開く
    await openTaskDetail(page, uniqueTitle);

    // 承認する
    await approveTask(page);

    // 申請一覧でステータスが「承認済み」に更新されたことを確認
    await verifyWorkflowStatus(page, uniqueTitle, "承認済み");
  });

  test("2段階承認フローで両ステップ承認後にワークフローが承認済みになる", async ({
    page,
    browser,
  }) => {
    const uniqueTitle = `2段階承認テスト ${Date.now()}`;

    // Given: admin が2段階承認申請を作成し、admin を上長承認、一般ユーザーを経理承認に指定
    await createAndSubmitMultiStepWorkflow(
      page,
      uniqueTitle,
      ADMIN_USER.name,
      REGULAR_USER.name,
    );

    // When: admin がタスク詳細からステップ1（上長承認）を承認
    await openTaskDetail(page, uniqueTitle);
    await approveTask(page);

    // And: 一般ユーザーがタスク詳細からステップ2（経理承認）を承認
    const userContext = await browser.newContext({
      storageState: USER_AUTH_FILE,
    });
    const userPage = await userContext.newPage();

    await openTaskDetail(userPage, uniqueTitle);
    await approveTask(userPage);

    await userContext.close();

    // Then: 申請一覧でステータスが「承認済み」に更新される
    await verifyWorkflowStatus(page, uniqueTitle, "承認済み");
  });
});
