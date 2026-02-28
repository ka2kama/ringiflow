# 実装計画: #780 E2E-009 ユーザー管理 E2E テスト

## Context

要件定義 E2E-009（ユーザー管理: 作成→編集→無効化）の Playwright E2E テストを追加する。
Epic #774（API テストカバレッジギャップ解消）の最後の Story。ユーザー管理 UI（`/users` 系4画面）は実装済みだが E2E テストが存在しない。

## スコープ

対象:
- `tests/e2e/tests/user-management.spec.ts` の新規作成（3テスト）
- `docs/50_テスト/E2Eテスト突合表.md` の更新（E2E-009 をカバー済みに変更）

対象外:
- ヘルパー関数の追加（ユーザー管理操作は他テストと共有不要）
- `helpers/test-data.ts` の変更（テストデータは各テスト内で動的生成）
- バリデーションエラー・権限エラーの E2E テスト（API テスト #776 でカバー済み）

## 設計判断

1. テストの独立性: 3テストを独立に実装。各テストが自身のユーザーを作成する
   - 理由: 障害分離（1テスト失敗が他に影響しない）、並列実行可能、既存パターン準拠
   - トレードオフ: テスト時間が若干増加するが、3テスト程度は許容範囲

2. データ作成方法: 全テストで UI 操作によりユーザーを作成
   - 理由: E2E テストの「ユーザー操作で完結」原則

3. ロール選択: `selectOption({ label: "user" })` でラベルにより選択
   - `Page/User/New.elm` 行332 で `role.name` がドロップダウンのラベルに使われる
   - `role_repository.rs` で `system_admin` は除外され、`tenant_admin` と `user` が表示される

4. 一覧からの詳細遷移: `getByRole("row").filter({ hasText: name }).getByRole("link").first().click()`
   - `Page/User/List.elm` で各 `<td>` 内に `<a>` が配置されている（`<tr>` クリックでは遷移しない）

## Phase 1: テスト実装

### 確認事項
- パターン: 既存 E2E テスト構造 → `tests/e2e/tests/workflow-definition-management.spec.ts`
- UI テキスト（Elm ソースから確認済み）:
  - 作成画面 heading: 「ユーザーを作成」（`New.elm` 行310）
  - 成功メッセージ: 「ユーザーを作成しました」（`New.elm` 行239）
  - 一覧戻り: 「ユーザー一覧に戻る」（`New.elm` 行257）
  - 編集画面 heading: 「ユーザーを編集」
  - 無効化ボタン: 「無効化」（`Detail.elm` 行308）
  - 無効化確認: 「無効化する」（`Detail.elm` 行400）
  - 無効化成功: 「ユーザーを無効化しました。」（`Detail.elm` 行145）
  - ステータスバッジ: 「アクティブ」/「非アクティブ」
- ライブラリ: Playwright `selectOption({ label: "text" })` → `<option>` 表示テキストでマッチ
- ライブラリ: Playwright `getByLabel` → `<label>` と同じ親 `<div>` 内の兄弟 `<select>`/`<input>` を検出。`FormField.elm` の構造（行63-75, 112-128）で `<label>` + `<select>/<input>` は同じ `<div>` の直接子
- ConfirmDialog: `<dialog>` + `showModal()`。既存パターン（`workflow-definition-management.spec.ts`）で `getByRole("button")` による操作が動作確認済み

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 管理者が `/users/new` でフォーム入力 → 作成 → 成功画面 → 一覧で確認 | 正常系 | E2E |
| 2 | 管理者がユーザー作成 → 一覧→詳細→編集で名前変更 → 保存 → 詳細で確認 | 正常系 | E2E |
| 3 | 管理者がユーザー作成 → 一覧→詳細 → 無効化 → 確認ダイアログ承認 → ステータス変更確認 | 正常系 | E2E |

### テストリスト

E2E テスト:
- [ ] 管理者がユーザーを作成し一覧に表示される
- [ ] 管理者が作成したユーザーの名前を編集し変更が反映される
- [ ] 管理者が作成したユーザーを無効化しステータスが変更される

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）

### テストコード

ファイル: `tests/e2e/tests/user-management.spec.ts`

```typescript
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
    await page.getByLabel("メールアドレス").fill(email);
    await page.getByLabel("名前").fill(name);
    await page.getByLabel("ロール").selectOption({ label: "user" });
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
    await page.getByLabel("メールアドレス").fill(email);
    await page.getByLabel("名前").fill(originalName);
    await page.getByLabel("ロール").selectOption({ label: "user" });
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
    await page.getByLabel("名前").clear();
    await page.getByLabel("名前").fill(updatedName);
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
    await page.getByLabel("メールアドレス").fill(email);
    await page.getByLabel("名前").fill(name);
    await page.getByLabel("ロール").selectOption({ label: "user" });
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
    await expect(page.getByText("アクティブ")).toBeVisible();

    // When: 無効化ボタンをクリックし、確認ダイアログで承認する
    await page.getByRole("button", { name: "無効化" }).click();
    await page.getByRole("button", { name: "無効化する" }).click();

    // Then: 成功メッセージとステータス変更が確認できる
    await expect(page.getByText("ユーザーを無効化しました。")).toBeVisible();
    await expect(page.getByText("非アクティブ")).toBeVisible();
  });
});
```

### 潜在的リスクと対策

| リスク | 対策 |
|--------|------|
| `getByLabel("ロール")` が `<select>` に到達しない | フォールバック: `page.locator("select")` でプレースホルダーテキストをフィルタ |
| `selectOption({ label: "user" })` がマッチしない | フォールバック: `selectOption("user")`（value, label, text のいずれかでマッチ） |

## Phase 2: E2E テスト突合表の更新

### 確認事項: なし（既知のパターンのみ）

変更内容:
- `docs/50_テスト/E2Eテスト突合表.md`:
  - E2E-009 行: テストファイル `user-management.spec.ts`、テスト件数 3、状態「カバー済み（作成・編集・無効化）」
  - サマリー: カバー済み 5→6、未実装（機能実装済み）1→0
  - 更新履歴に行を追加

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `<label>` と `<select>` に `for`/`id` がない → `getByLabel` が動作するか | 技術的前提 | `FormField.elm` 行112-128 で構造確認。同コンポーネントの `getByLabel("名前")` が既存テストで動作。フォールバックロケーターを記載 |
| 2回目 | `selectOption` のマッチ方法が不明 | ライブラリ API | Playwright の `selectOption({ label: "text" })` は `<option>` 表示テキストにマッチ。Elm 行332 で `role.name` がラベル |
| 3回目 | 一覧の行クリックが遷移しない可能性 | 既存パターン | `Page/User/List.elm` で `<td>` 内に `<a>` 配置。`getByRole("link").first()` でクリック |
| 4回目 | ドロップダウンに表示されるロール名が不明 | 未定義 | `role_repository.rs` で `system_admin` 除外確認。`tenant_admin`, `user` が表示。`user` を使用 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 完了基準5項目（作成テスト、編集テスト、無効化テスト、突合表更新、`just test-e2e`）すべてカバー |
| 2 | 曖昧さ排除 | OK | 全ロケーター・テキスト・操作手順がコードで確定。不確定表現なし |
| 3 | 設計判断の完結性 | OK | テスト独立性、データ作成方法、ロール選択、行ナビゲーションに判断記載 |
| 4 | スコープ境界 | OK | 対象（テストファイル、突合表）と対象外（ヘルパー、テストデータ定数、バリデーション）を明記 |
| 5 | 技術的前提 | OK | `getByLabel` の暗黙的関連付け、`selectOption` のマッチ方法、ConfirmDialog の `<dialog>` 動作をソースで確認 |
| 6 | 既存ドキュメント整合 | OK | E2E-009 要件「作成→編集→無効化」と一致。E2E ルール（クリティカルパスのみ、Given-When-Then、Date.now()、突合表更新）すべて準拠 |

## 検証方法

1. `just test-e2e` で E2E テスト全体が通ること
2. `just check-all` で全テストが通ること
