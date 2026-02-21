/**
 * ワークフロー定義管理 E2E テスト
 *
 * テナント管理者がワークフロー定義の一覧閲覧・作成・公開・アーカイブを
 * 完了できることを検証する。
 */

import { expect, test } from "@playwright/test";

test.describe("ワークフロー定義管理", () => {
  test("テナント管理者が定義一覧ページにアクセスできる", async ({ page }) => {
    // When: 定義一覧ページに移動
    await page.goto("/workflow-definitions");

    // Then: 見出しが表示される
    await expect(
      page.getByRole("heading", { name: "ワークフロー定義" }),
    ).toBeVisible();

    // Then: 既存のシードデータが表示される
    await expect(page.getByRole("table")).toBeVisible();
  });

  test("テナント管理者が新しいワークフロー定義を作成できる", async ({
    page,
  }) => {
    const uniqueName = `E2E テスト定義 ${Date.now()}`;

    // Given: 定義一覧ページに移動し、初期ロード完了を待つ
    await page.goto("/workflow-definitions");
    await expect(page.getByRole("table")).toBeVisible();

    // When: 新規作成ダイアログを開く
    await page.getByRole("button", { name: "新規作成" }).click();

    // When: フォームに入力して送信
    await page.getByLabel("名前").fill(uniqueName);
    await page.getByLabel("説明").fill("E2E テスト用の定義です");
    await page.getByRole("button", { name: "作成", exact: true }).click();

    // Then: 成功メッセージが表示される
    await expect(
      page.getByText("ワークフロー定義を作成しました"),
    ).toBeVisible();

    // Then: 作成した定義が一覧に表示される（Draft 状態）
    const row = page.locator("tr").filter({ hasText: uniqueName });
    await expect(row).toBeVisible();
    await expect(row.getByText("下書き")).toBeVisible();
  });

  test("テナント管理者が定義を公開・アーカイブできる", async ({ page }) => {
    const uniqueName = `E2E 公開テスト ${Date.now()}`;

    // Given: 新しい定義を作成（初期ロード完了を待ってから操作）
    await page.goto("/workflow-definitions");
    await expect(page.getByRole("table")).toBeVisible();
    await page.getByRole("button", { name: "新規作成" }).click();
    await page.getByLabel("名前").fill(uniqueName);
    await page.getByRole("button", { name: "作成", exact: true }).click();
    await expect(
      page.getByText("ワークフロー定義を作成しました"),
    ).toBeVisible();

    // When: 公開する
    const row = page.locator("tr").filter({ hasText: uniqueName });
    await row.getByRole("button", { name: "公開" }).click();
    await page.getByRole("button", { name: "公開する" }).click();

    // Then: 公開成功メッセージが表示される
    await expect(
      page.getByText("ワークフロー定義を公開しました"),
    ).toBeVisible();

    // Then: ステータスが「公開済み」に変わる
    const publishedRow = page.locator("tr").filter({ hasText: uniqueName });
    await expect(publishedRow.getByText("公開済み")).toBeVisible();

    // When: アーカイブする
    await publishedRow.getByRole("button", { name: "アーカイブ" }).click();
    await page.getByRole("button", { name: "アーカイブする" }).click();

    // Then: アーカイブ成功メッセージが表示される
    await expect(
      page.getByText("ワークフロー定義をアーカイブしました"),
    ).toBeVisible();

    // Then: ステータスが「アーカイブ済み」に変わる
    const archivedRow = page.locator("tr").filter({ hasText: uniqueName });
    await expect(archivedRow.getByText("アーカイブ済み")).toBeVisible();
  });

  test("デザイナー画面で定義を検証できる", async ({ page }) => {
    const uniqueName = `E2E 検証テスト ${Date.now()}`;

    // Given: 新しい定義を作成する
    await page.goto("/workflow-definitions");
    await expect(page.getByRole("table")).toBeVisible();
    await page.getByRole("button", { name: "新規作成" }).click();
    await page.getByLabel("名前").fill(uniqueName);
    await page.getByRole("button", { name: "作成", exact: true }).click();
    await expect(
      page.getByText("ワークフロー定義を作成しました"),
    ).toBeVisible();

    // Given: デザイナー画面を開く
    const row = page.locator("tr").filter({ hasText: uniqueName });
    await row.getByRole("link", { name: "編集" }).click();
    await expect(
      page.getByRole("heading", { name: "ワークフローデザイナー" }),
    ).toBeVisible();

    // When: 検証ボタンをクリックする
    await page.getByRole("button", { name: "検証" }).click();

    // Then: 検証結果が表示される（デフォルト定義は有効）
    await expect(page.getByText("フロー定義は有効です")).toBeVisible();
  });

  test("デザイナー画面から定義を公開できる", async ({ page }) => {
    const uniqueName = `E2E デザイナー公開テスト ${Date.now()}`;

    // Given: 新しい定義を作成する
    await page.goto("/workflow-definitions");
    await expect(page.getByRole("table")).toBeVisible();
    await page.getByRole("button", { name: "新規作成" }).click();
    await page.getByLabel("名前").fill(uniqueName);
    await page.getByRole("button", { name: "作成", exact: true }).click();
    await expect(
      page.getByText("ワークフロー定義を作成しました"),
    ).toBeVisible();

    // Given: デザイナー画面を開く
    const row = page.locator("tr").filter({ hasText: uniqueName });
    await row.getByRole("link", { name: "編集" }).click();
    await expect(
      page.getByRole("heading", { name: "ワークフローデザイナー" }),
    ).toBeVisible();

    // When: 公開ボタンをクリックする
    await page.getByRole("button", { name: "公開" }).click();

    // Then: 確認ダイアログが表示される
    await expect(page.getByText("を公開しますか？")).toBeVisible();

    // When: 公開を確定する
    await page.getByRole("button", { name: "公開する" }).click();

    // Then: 公開成功メッセージが表示される
    await expect(page.getByText("公開しました")).toBeVisible();
  });
});
