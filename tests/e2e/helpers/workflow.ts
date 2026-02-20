/**
 * ワークフロー操作ヘルパー
 *
 * E2E テスト共通のワークフロー作成・操作ユーティリティ。
 */

import { type Page, expect } from "@playwright/test";

/**
 * 汎用申請テンプレートでワークフローを作成して申請する。
 *
 * ログインユーザー自身を承認者に指定する自己承認パターン。
 * テスト戦略: 承認者を指定するために、ログインユーザーの名前で検索する。
 */
export async function createAndSubmitWorkflow(
  page: Page,
  title: string,
  approverSearch: string,
): Promise<void> {
  await page.goto("/workflows/new");
  await page.getByText("汎用申請").click();
  await page.getByPlaceholder("申請のタイトルを入力").fill(title);

  const dynamicForm = page.locator(".bg-secondary-50");
  await dynamicForm.locator("input[name='title']").fill("テスト件名");
  await dynamicForm.getByLabel("内容").fill("テスト内容");

  await page.locator("#approver-search").fill(approverSearch);
  await page.locator("li").filter({ hasText: approverSearch }).first().click();

  await page.getByRole("button", { name: "申請する" }).click();
  await expect(page.getByText("申請が完了しました")).toBeVisible();
}

/**
 * 2段階承認申請テンプレートでワークフローを作成して申請する。
 *
 * 上長承認と経理承認の2名の承認者を順次選択する。
 * 承認者選択時、先に選択した入力欄は badge に置き換わるため、
 * 次の検索入力は残った唯一の #approver-search に対して行う。
 */
export async function createAndSubmitMultiStepWorkflow(
  page: Page,
  title: string,
  step1ApproverSearch: string,
  step2ApproverSearch: string,
): Promise<void> {
  await page.goto("/workflows/new");
  await page.getByText("2段階承認申請").click();
  await page.getByPlaceholder("申請のタイトルを入力").fill(title);

  // 動的フォーム: 件名、内容、金額
  const dynamicForm = page.locator(".bg-secondary-50");
  await dynamicForm.locator("input[name='title']").fill("テスト件名");
  await dynamicForm.getByLabel("内容").fill("テスト内容");
  await dynamicForm.getByLabel("金額").fill("100000");

  // 上長承認の承認者を選択（1つ目の #approver-search）
  await page.locator("#approver-search").first().fill(step1ApproverSearch);
  await page
    .locator("li")
    .filter({ hasText: step1ApproverSearch })
    .first()
    .click();

  // 選択完了を待機（badge に置き換わり、#approver-search が1つになる）
  await expect(page.locator("#approver-search")).toHaveCount(1);

  // 経理承認の承認者を選択（残った唯一の #approver-search）
  await page.locator("#approver-search").fill(step2ApproverSearch);
  await page
    .locator("li")
    .filter({ hasText: step2ApproverSearch })
    .first()
    .click();

  // 全承認者の選択完了を待機（全 #approver-search が badge に置換される）
  await expect(page.locator("#approver-search")).toHaveCount(0);

  await page.getByRole("button", { name: "申請する" }).click();
  await expect(page.getByText("申請が完了しました")).toBeVisible();
}

/**
 * タスク一覧から指定タイトルのタスク詳細を開く。
 */
export async function openTaskDetail(
  page: Page,
  taskTitle: string,
): Promise<void> {
  await page.goto("/tasks");
  const taskRow = page.locator("tr").filter({ hasText: taskTitle });
  await taskRow.getByRole("link").first().click();
}

/**
 * タスク詳細画面で承認操作を行う（確認ダイアログ含む）。
 */
export async function approveTask(page: Page): Promise<void> {
  await page.getByRole("button", { name: "承認", exact: true }).click();
  await page.getByRole("button", { name: "承認する" }).click();
  await expect(page.getByText("承認しました")).toBeVisible();
}

/**
 * タスク詳細画面で却下操作を行う（確認ダイアログ含む）。
 */
export async function rejectTask(page: Page): Promise<void> {
  await page.getByRole("button", { name: "却下" }).click();
  await page.getByRole("button", { name: "却下する" }).click();
  await expect(page.getByText("却下しました")).toBeVisible();
}

/**
 * タスク詳細画面で差し戻し操作を行う（確認ダイアログ含む）。
 */
export async function requestChanges(page: Page): Promise<void> {
  await page.getByRole("button", { name: "差し戻し" }).click();
  await page.getByRole("button", { name: "差し戻す" }).click();
  await expect(page.getByText("差し戻しました")).toBeVisible();
}

/**
 * 申請一覧でワークフローのステータスを検証する。
 */
export async function verifyWorkflowStatus(
  page: Page,
  workflowTitle: string,
  expectedStatus: string,
): Promise<void> {
  await page.goto("/workflows");
  const workflowRow = page.locator("tr").filter({ hasText: workflowTitle });
  await expect(
    workflowRow.getByText(expectedStatus, { exact: true }),
  ).toBeVisible();
}
