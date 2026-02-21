/**
 * デザイナー定義ワークフローフロー E2E テスト（E2E-010）
 *
 * デザイナー形式（position 付き）の定義で申請→承認フローが
 * 正常に動作することを検証する。
 * 定義作成・公開は API で行い、申請・承認は UI で操作する。
 */

import { expect, test } from "@playwright/test";
import { ADMIN_USER, TENANT_ID } from "../helpers/test-data";
import {
  approveTask,
  openTaskDetail,
  verifyWorkflowStatus,
} from "../helpers/workflow";

/** デザイナー形式の定義 JSON（1段階承認、form: fields 空） */
const designerDefinition = {
  form: { fields: [] },
  steps: [
    {
      id: "start_1",
      type: "start",
      name: "開始",
      position: { x: 400, y: 50 },
    },
    {
      id: "approval_1",
      type: "approval",
      name: "承認",
      assignee: { type: "user" },
      position: { x: 400, y: 200 },
    },
    {
      id: "end_1",
      type: "end",
      name: "承認完了",
      status: "approved",
      position: { x: 250, y: 350 },
    },
    {
      id: "end_2",
      type: "end",
      name: "却下",
      status: "rejected",
      position: { x: 550, y: 350 },
    },
  ],
  transitions: [
    { from: "start_1", to: "approval_1" },
    { from: "approval_1", to: "end_1", trigger: "approve" },
    { from: "approval_1", to: "end_2", trigger: "reject" },
  ],
};

test.describe("デザイナー定義ワークフローフロー", () => {
  test("デザイナーで作成した定義で申請して承認できる", async ({ page }) => {
    const uniqueName = `E2E デザイナー定義 ${Date.now()}`;
    const uniqueTitle = `デザイナー申請テスト ${Date.now()}`;

    // Given: CSRF トークンを取得する
    const csrfResponse = await page.request.get("/api/v1/auth/csrf", {
      headers: { "X-Tenant-ID": TENANT_ID },
    });
    expect(csrfResponse.status()).toBe(200);
    const csrfBody = await csrfResponse.json();
    const csrfToken = csrfBody.data.token;

    const apiHeaders = {
      "X-Tenant-ID": TENANT_ID,
      "X-CSRF-Token": csrfToken,
    };

    // Given: API でデザイナー形式の定義を作成する
    const createResponse = await page.request.post(
      "/api/v1/workflow-definitions",
      { headers: apiHeaders, data: { name: uniqueName, definition: designerDefinition } },
    );
    expect(createResponse.status()).toBe(201);

    const createBody = await createResponse.json();
    const definitionId = createBody.data.id;

    // Given: 作成した定義を公開する
    const publishResponse = await page.request.post(
      `/api/v1/workflow-definitions/${definitionId}/publish`,
      { headers: apiHeaders, data: { version: 1 } },
    );
    expect(publishResponse.status()).toBe(200);

    // When: 新規申請画面でデザイナー定義を選択して申請する
    await page.goto("/workflows/new");
    await page.getByText(uniqueName).click();
    await page.getByPlaceholder("申請のタイトルを入力").fill(uniqueTitle);

    await page.locator("#approver-search").fill(ADMIN_USER.name);
    await page
      .locator("li")
      .filter({ hasText: ADMIN_USER.name })
      .first()
      .click();

    await page.getByRole("button", { name: "申請する" }).click();

    // Then: 申請完了メッセージが表示される
    await expect(page.getByText("申請が完了しました")).toBeVisible();

    // When: 承認者がタスク詳細から承認する
    await openTaskDetail(page, uniqueTitle);
    await approveTask(page);

    // Then: 申請一覧でステータスが「承認済み」に更新される
    await verifyWorkflowStatus(page, uniqueTitle, "承認済み");
  });
});
