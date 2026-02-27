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
  test("承認ボタンをクリックすると確認ダイアログが表示される", async ({
    page,
  }) => {
    // Given: ワークフロー申請を作成・提出し、タスク詳細を開く
    const uniqueTitle = `ダイアログ確認テスト ${Date.now()}`;
    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);
    await openTaskDetail(page, uniqueTitle);

    // When: 承認ボタンをクリックする
    await page.getByRole("button", { name: "承認", exact: true }).click();

    // Then: 確認ダイアログが表示され、タイトルとメッセージが適切であること
    const dialog = page.locator("#confirm-dialog");
    await expect(dialog).toBeVisible();
    await expect(page.locator("#confirm-dialog-title")).toHaveText(
      "承認の確認",
    );
    await expect(page.locator("#confirm-dialog-message")).toHaveText(
      "この申請を承認しますか？",
    );
  });

  test("確認ダイアログのキャンセルで操作が実行されない", async ({ page }) => {
    // Given: ワークフロー申請を作成・提出し、タスク詳細を開く
    const uniqueTitle = `キャンセルテスト ${Date.now()}`;
    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);
    await openTaskDetail(page, uniqueTitle);

    // When: 承認ボタン → 確認ダイアログのキャンセルボタンをクリック
    await page.getByRole("button", { name: "承認", exact: true }).click();
    await expect(page.locator("#confirm-dialog")).toBeVisible();
    await page.getByRole("button", { name: "キャンセル" }).click();

    // Then: ダイアログが閉じ、承認ボタンがまだ有効（操作が実行されていない）
    await expect(page.locator("#confirm-dialog")).not.toBeVisible();
    await expect(
      page.getByRole("button", { name: "承認", exact: true }),
    ).toBeVisible();
  });

  test("申請を作成すると承認者のタスク一覧に表示される", async ({ page }) => {
    // Given: ワークフロー申請を作成・提出する
    const uniqueTitle = `承認テスト ${Date.now()}`;
    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);

    // When: タスク一覧に移動する
    await page.goto("/tasks");
    await expect(
      page.getByRole("heading", { name: "タスク一覧" }),
    ).toBeVisible();

    // Then: 作成した申請のタスクが表示される
    await expect(page.getByText(uniqueTitle)).toBeVisible();
  });

  test("タスク詳細から承認するとステータスが更新される", async ({ page }) => {
    // Given: ワークフロー申請を作成・提出する
    const uniqueTitle = `承認完了テスト ${Date.now()}`;
    await createAndSubmitWorkflow(page, uniqueTitle, ADMIN_USER.name);

    // When: タスク詳細を開き承認する
    await openTaskDetail(page, uniqueTitle);
    await approveTask(page);

    // Then: 申請一覧でステータスが「承認済み」に更新される
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
