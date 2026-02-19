/**
 * 差し戻し→再申請フロー E2E テスト（E2E-006）
 *
 * 申請を作成し、承認者として差し戻し操作を行い、
 * 申請者がフォームを修正して再申請し、
 * 再度承認してステータスが正しく更新されることを検証する。
 */

import { expect, test } from "@playwright/test";
import { ADMIN_USER } from "../helpers/test-data";
import {
  approveTask,
  createAndSubmitWorkflow,
  openTaskDetail,
  requestChanges,
  verifyWorkflowStatus,
} from "../helpers/workflow";

test.describe("差し戻し→再申請フロー", () => {
  test("差し戻し後にフォームを修正して再申請し、承認するとステータスが更新される", async ({
    page,
  }) => {
    const uniqueTitle = `差し戻しテスト ${Date.now()}`;

    // Given: admin が汎用申請を作成し、自身を承認者に指定
    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);

    // When: admin がタスク詳細から差し戻す
    await openTaskDetail(page, uniqueTitle);
    await requestChanges(page);

    // And: admin がワークフロー詳細からフォームを修正して再申請する
    await page.goto("/workflows");
    await page
      .locator("tr")
      .filter({ hasText: uniqueTitle })
      .getByRole("link")
      .first()
      .click();

    await page.getByRole("button", { name: "再申請する" }).click();
    await page
      .locator('label:text-is("内容") + input')
      .fill("修正後のテスト内容");
    await page.getByRole("button", { name: "再申請する" }).click();
    await expect(page.getByText("再申請しました")).toBeVisible();

    // And: admin がタスク詳細から承認する
    await openTaskDetail(page, uniqueTitle);
    await approveTask(page);

    // Then: 申請一覧でステータスが「承認済み」に更新される
    await verifyWorkflowStatus(page, uniqueTitle, "承認済み");
  });
});
