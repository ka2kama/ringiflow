# Playwright

[Playwright](https://playwright.dev/) は Microsoft が開発するブラウザ自動化・テストフレームワーク。
Chromium、Firefox、WebKit に対応し、ブラウザ操作を TypeScript/JavaScript で記述してテストを実行する。

このプロジェクトでは E2E テスト（ユーザー操作を通じた統合テスト）に使用する。

## なぜ Playwright を使うのか

| 比較対象 | 違い |
|---------|------|
| **Hurl（API テスト）** | Hurl は HTTP リクエスト/レスポンスの検証。Playwright はブラウザを操作してフロントエンド + バックエンドの統合を検証する |
| **elm-program-test** | メンテナンス状況に懸念があり見送り（#127）。Playwright は Microsoft が活発に開発しており、業界標準 |
| **Cypress** | Playwright の方が高速（並列実行）、マルチブラウザ対応が強い |

選定の経緯: [#127](https://github.com/ka2kama/ringiflow/issues/127)、[セッションログ](../../../prompts/runs/2026-01/2026-01-27_テスト追加とE2Eテスト方針決定.md)

## インストール

```bash
cd tests/e2e
pnpm install
npx playwright install chromium
```

`just setup` で自動的にインストールされる。

## テスト実行

### ローカル実行

```bash
# 前提: just dev-all で開発サーバーが起動していること
cd tests/e2e && npx playwright test

# justfile 経由（API テスト環境を自動起動して実行）
just test-e2e
```

### コマンドオプション

```bash
# ブラウザを表示して実行
npx playwright test --headed

# UI モードで実行（インタラクティブにテストを選択・デバッグ）
npx playwright test --ui

# 特定のファイルだけ実行
npx playwright test tests/dashboard.spec.ts

# テスト名でフィルタ
npx playwright test -g "ダッシュボード"
```

### ローカル vs CI の環境差異

| 環境 | バックエンド | フロントエンド | E2E_BASE_URL |
|------|------------|--------------|--------------|
| ローカル（dev-all） | ポート 13000-13002 | ポート 15173 | `http://localhost:15173`（デフォルト） |
| CI / test-e2e | ポート 14000-14002 | ポート 15173 | `http://localhost:15173` |

ローカルでは `just dev-all` で起動した開発サーバーに対してテストを実行する。
CI では `scripts/run-e2e-tests.sh` がバックエンド・フロントエンドを自動起動する。

## 認証パターン: storageState

Playwright 推奨の認証パターンを採用している。

参考: [Playwright Authentication](https://playwright.dev/docs/auth)

### 仕組み

1. **セットアップ**（`auth.setup.ts`）: API ログインで Cookie を取得し、`tests/.auth/admin.json` に保存
2. **各テスト**: 保存済みの `storageState` を読み込み、認証済み状態で実行

```typescript
// auth.setup.ts
setup("管理者ユーザーでログインする", async ({ request }) => {
  await login(request, ADMIN_USER.email, ADMIN_USER.password);
  await request.storageState({ path: authFile });
});
```

```typescript
// playwright.config.ts（抜粋）
projects: [
  { name: "setup", testMatch: /.*\.setup\.ts/ },
  {
    name: "chromium",
    use: { storageState: "tests/.auth/admin.json" },
    dependencies: ["setup"],  // setup 完了後に実行
  },
],
```

### 新しいロールのテストを追加する場合

1. `test-data.ts` にユーザー定数を追加
2. `auth.setup.ts` に新しいセットアップを追加
3. `playwright.config.ts` に新しいプロジェクトを追加

```typescript
// 例: 一般ユーザーのテストを追加
setup("一般ユーザーでログインする", async ({ request }) => {
  await login(request, REGULAR_USER.email, REGULAR_USER.password);
  await request.storageState({ path: "tests/.auth/user.json" });
});
```

## テストの書き方

### 基本構造

ADR-032 に従い、E2E テストは Given-When-Then 形式で記述する。テスト名は日本語。

```typescript
test.describe("申請フロー", () => {
  test("新規申請ページにアクセスできる", async ({ page }) => {
    // Given: 認証済みユーザー（storageState で自動設定）

    // When: 新規申請ページに移動
    await page.goto("/workflows/new");

    // Then: 見出しが表示される
    await expect(
      page.getByRole("heading", { name: "新規申請" })
    ).toBeVisible();
  });
});
```

### ロケーター（要素の特定方法）

Playwright 推奨の優先順位:

| 優先度 | ロケーター | 用途 | 例 |
|--------|-----------|------|-----|
| 1 | `getByRole` | ARIA ロール | `page.getByRole("button", { name: "申請する" })` |
| 2 | `getByText` | テキスト内容 | `page.getByText("承認待ちタスク")` |
| 3 | `getByLabel` | フォームラベル | `page.getByLabel("内容")` |
| 4 | `getByPlaceholder` | プレースホルダー | `page.getByPlaceholder("申請のタイトルを入力")` |
| 5 | `locator` | CSS セレクタ（最終手段） | `page.locator("#approver-search")` |

参考: [Playwright Locators](https://playwright.dev/docs/locators)

### テストデータの分離

テスト間でデータが衝突しないよう、ユニークな識別子を使う:

```typescript
const uniqueTitle = `E2E テスト申請 ${Date.now()}`;
```

### スコープ制限（同名要素の区別）

ページ内に同名要素がある場合、`locator` でスコープを制限する:

```typescript
// メインコンテンツにスコープを限定
const main = page.locator("#main-content");
await expect(main.getByRole("link", { name: "申請一覧" })).toBeVisible();
```

## デバッグ手順

### 1. Codegen（テスト記録）

ブラウザ操作を記録してテストコードを自動生成する:

```bash
cd tests/e2e
npx playwright codegen http://localhost:15173
```

ブラウザが開き、操作を記録してコードを生成する。新しいテストを書く際のスタート地点として便利。

### 2. UI モード

テストをインタラクティブに実行・デバッグする:

```bash
npx playwright test --ui
```

テストの選択、ステップ実行、DOM スナップショットの確認が可能。

### 3. ヘッド付き実行

ブラウザを表示しながらテストを実行する:

```bash
npx playwright test --headed
```

### 4. テストレポート

テスト失敗時に HTML レポートを確認する:

```bash
npx playwright show-report
```

### 5. スクリーンショットとトレース

設定により、テスト失敗時に自動で取得される:

| 設定 | 条件 | 出力先 |
|------|------|--------|
| `screenshot: "only-on-failure"` | テスト失敗時 | `test-results/` |
| `trace: "on-first-retry"` | 初回リトライ時 | `test-results/` |

トレースファイルの確認:

```bash
npx playwright show-trace test-results/xxx/trace.zip
```

## プロジェクトでのファイル構成

```
tests/e2e/
├── package.json            # Playwright 依存関係
├── pnpm-lock.yaml
├── playwright.config.ts    # Playwright 設定
├── helpers/
│   ├── auth.ts             # API ログインヘルパー
│   └── test-data.ts        # テストデータ定数（シードデータに対応）
└── tests/
    ├── .auth/              # 認証状態保存（.gitignore）
    ├── auth.setup.ts       # 認証セットアップ
    ├── dashboard.spec.ts   # ダッシュボードテスト
    ├── workflow.spec.ts    # 申請フローテスト
    └── approval.spec.ts   # 承認フローテスト
```

→ ファイルごとの詳細設計: [実装解説](../../07_実装解説/18_E2Eテスト/01_Playwright_E2Eテスト_コード解説.md)

## 参考

- [Playwright 公式サイト](https://playwright.dev/)
- [Playwright ドキュメント](https://playwright.dev/docs/intro)
- [Authentication](https://playwright.dev/docs/auth) — storageState パターン
- [Locators](https://playwright.dev/docs/locators) — 要素の特定方法
- [Test Generator (Codegen)](https://playwright.dev/docs/codegen) — テスト記録ツール
- [Playwright GitHub](https://github.com/microsoft/playwright)
