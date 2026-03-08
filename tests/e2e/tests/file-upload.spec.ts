/**
 * ファイルアップロード E2E テスト（E2E-007）
 *
 * ドキュメント管理画面でのファイルアップロードと、
 * ワークフロー申請でのファイル添付を検証する。
 */

import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";

/** テスト用アップロードファイルのパス */
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const TEST_FILE = path.join(__dirname, "../fixtures/test-upload.txt");

test.describe("ファイルアップロード", () => {
  test("ドキュメント管理画面でフォルダを作成し、ファイルをアップロードすると一覧に表示される", async ({
    page,
  }) => {
    const folderName = `E2E テストフォルダ ${Date.now()}`;

    // Given: ドキュメント管理画面に移動
    await page.goto("/documents");
    await expect(
      page.getByRole("heading", { name: "ドキュメント管理" }),
    ).toBeVisible();

    // When: フォルダを作成
    await page.getByRole("button", { name: "フォルダ作成" }).click();
    await page.getByPlaceholder("フォルダ名を入力").fill(folderName);
    await page.getByRole("button", { name: "作成", exact: true }).click();

    // Then: フォルダが作成される
    await expect(page.getByText("フォルダを作成しました")).toBeVisible();

    // When: フォルダを選択
    await page.getByText(folderName).click();

    // Then: ファイル一覧パネルに「アップロード」ボタンが表示される
    await expect(
      page.getByRole("button", { name: "アップロード" }),
    ).toBeVisible();

    // When: ファイルをアップロード
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "アップロード" }).click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(TEST_FILE);

    // Then: アップロード成功メッセージが表示される
    await expect(
      page.getByText("ファイルをアップロードしました"),
    ).toBeVisible();

    // Then: ファイル一覧にアップロードしたファイルが表示される
    await expect(page.getByText("test-upload.txt")).toBeVisible();
  });

  test("ワークフロー申請でファイルを添付して申請すると、詳細画面で添付ファイルが確認できる", async ({
    page,
  }) => {
    const uniqueTitle = `E2E ファイル添付テスト ${Date.now()}`;

    // Given: 新規申請ページに移動
    await page.goto("/workflows/new");

    // When: ファイル添付申請テンプレートを選択
    await page.getByText("ファイル添付申請").click();

    // When: フォーム入力
    await page.getByPlaceholder("申請のタイトルを入力").fill(uniqueTitle);
    const dynamicForm = page.locator(".bg-secondary-50");
    await dynamicForm.locator("input[name='title']").fill("E2E テスト件名");

    // When: ファイルを添付（FileUpload コンポーネントのドロップゾーンをクリック）
    const fileChooserPromise = page.waitForEvent("filechooser");
    await page
      .getByText("ファイルをドラッグ&ドロップ、またはクリックして選択")
      .click();
    const fileChooser = await fileChooserPromise;
    await fileChooser.setFiles(TEST_FILE);

    // Then: ファイルが Pending 状態でリストに表示される
    await expect(page.getByText("test-upload.txt")).toBeVisible();

    // When: 承認者を選択
    await page.locator("#approver-search").fill("一般");
    await page.locator("li").filter({ hasText: "一般ユーザー" }).click();

    // When: 申請する
    await page.getByRole("button", { name: "申請する" }).click();

    // Then: アップロードが完了し、申請成功メッセージが表示される
    await expect(page.getByText("申請が完了しました")).toBeVisible({
      timeout: 30000,
    });

    // When: 申請一覧から詳細画面に移動
    await page.goto("/workflows");
    await page.getByText(uniqueTitle).click();

    // Then: 詳細画面の添付ファイルセクションにファイルが表示される
    await expect(
      page.getByRole("heading", { name: "添付ファイル" }),
    ).toBeVisible();
    await expect(page.getByText("test-upload.txt")).toBeVisible();
  });
});
