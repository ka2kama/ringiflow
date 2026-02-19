/**
 * 却下フロー E2E テスト（E2E-005）
 *
 * 申請を作成し、承認者として却下操作を行い、
 * ステータスが正しく更新されることを検証する。
 */

import { test } from "@playwright/test";
import { ADMIN_USER } from "../helpers/test-data";
import {
  createAndSubmitWorkflow,
  openTaskDetail,
  rejectTask,
  verifyWorkflowStatus,
} from "../helpers/workflow";

test.describe("却下フロー", () => {
  test("タスク詳細から却下するとステータスが更新される", async ({ page }) => {
    const uniqueTitle = `却下テスト ${Date.now()}`;

    // Given: admin が汎用申請を作成し、自身を承認者に指定
    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);

    // When: admin がタスク詳細から却下する
    await openTaskDetail(page, uniqueTitle);
    await rejectTask(page);

    // Then: 申請一覧でステータスが「却下」に更新される
    await verifyWorkflowStatus(page, uniqueTitle, "却下");
  });
});
