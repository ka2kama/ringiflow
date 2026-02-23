/**
 * ユーザー管理 E2E テスト
 *
 * 管理者がユーザーの作成・編集・無効化を完了できることを検証する。
 */

import { expect, test } from "@playwright/test";

test.describe("ユーザー管理", () => {
  test("管理者がユーザーを作成し一覧に表示される", async ({ page }) => {
    const uniqueId = Date.now();
    const email = `e2e-test-${uniqueId}@example.com`;
    const name = `E2E テストユーザー ${uniqueId}`;

    // Given: ユーザー作成ページに移動する
    await page.goto("/users/new");
    await expect(
      page.getByRole("heading", { name: "ユーザーを作成" }),
    ).toBeVisible();

    // When: フォームに入力して送信する
    await page.getByPlaceholder("user@example.com").fill(email);
    await page.getByPlaceholder("山田 太郎").fill(name);
    await page.locator("select").selectOption({ label: "user" });
    await page.getByRole("button", { name: "作成", exact: true }).click();

    // Then: 作成成功画面が表示される
    await expect(page.getByText("ユーザーを作成しました")).toBeVisible();
    await expect(page.getByText(name)).toBeVisible();

    // When: ユーザー一覧に戻る
    await page.getByRole("link", { name: "ユーザー一覧に戻る" }).click();

    // Then: 作成したユーザーが一覧に表示される
    const row = page.getByRole("row").filter({ hasText: name });
    await expect(row).toBeVisible();
  });

  test("管理者が作成したユーザーの名前を編集し変更が反映される", async ({
    page,
  }) => {
    const uniqueId = Date.now();
    const email = `e2e-edit-${uniqueId}@example.com`;
    const originalName = `E2E 編集前 ${uniqueId}`;
    const updatedName = `E2E 編集後 ${uniqueId}`;

    // Given: テスト用ユーザーを作成する
    await page.goto("/users/new");
    await page.getByPlaceholder("user@example.com").fill(email);
    await page.getByPlaceholder("山田 太郎").fill(originalName);
    await page.locator("select").selectOption({ label: "user" });
    await page.getByRole("button", { name: "作成", exact: true }).click();
    await expect(page.getByText("ユーザーを作成しました")).toBeVisible();

    // Given: 一覧に戻り、作成したユーザーの詳細ページを開く
    await page.getByRole("link", { name: "ユーザー一覧に戻る" }).click();
    await page
      .getByRole("row")
      .filter({ hasText: originalName })
      .getByRole("link")
      .first()
      .click();

    // When: 編集ページに遷移して名前を変更する
    await page.getByRole("link", { name: "編集" }).click();
    await expect(
      page.getByRole("heading", { name: "ユーザーを編集" }),
    ).toBeVisible();
    await page.getByPlaceholder("山田 太郎").clear();
    await page.getByPlaceholder("山田 太郎").fill(updatedName);
    await page.getByRole("button", { name: "保存" }).click();

    // Then: 詳細画面に遷移し、変更後の名前が表示される
    await expect(page.getByText(updatedName)).toBeVisible();
  });

  test("管理者が作成したユーザーを無効化しステータスが変更される", async ({
    page,
  }) => {
    const uniqueId = Date.now();
    const email = `e2e-deactivate-${uniqueId}@example.com`;
    const name = `E2E 無効化テスト ${uniqueId}`;

    // Given: テスト用ユーザーを作成する
    await page.goto("/users/new");
    await page.getByPlaceholder("user@example.com").fill(email);
    await page.getByPlaceholder("山田 太郎").fill(name);
    await page.locator("select").selectOption({ label: "user" });
    await page.getByRole("button", { name: "作成", exact: true }).click();
    await expect(page.getByText("ユーザーを作成しました")).toBeVisible();

    // Given: 一覧に戻り、作成したユーザーの詳細ページを開く
    await page.getByRole("link", { name: "ユーザー一覧に戻る" }).click();
    await page
      .getByRole("row")
      .filter({ hasText: name })
      .getByRole("link")
      .first()
      .click();
    await expect(
      page.getByRole("heading", { name: "基本情報" }),
    ).toBeVisible();
    await expect(
      page.getByText("アクティブ", { exact: true }),
    ).toBeVisible();

    // When: 無効化ボタンをクリックし、確認ダイアログで承認する
    await page.getByRole("button", { name: "無効化" }).click();
    await page.getByRole("button", { name: "無効化する" }).click();

    // Then: 成功メッセージとステータス変更が確認できる
    await expect(page.getByText("ユーザーを無効化しました。")).toBeVisible();
    await expect(
      page.getByText("非アクティブ", { exact: true }),
    ).toBeVisible();
  });
});
