# 計画: Playwright E2E テスト導入（#128）

## Context

現在のテスト基盤はユニットテスト（Rust 120件、Elm 59件）と API テスト（Hurl）で構成されている。フロントエンド + バックエンドを統合したブラウザベースのテストが不足しており、Playwright を導入して E2E テスト層を追加する。

## Issue 精査

| 観点 | 分析 |
|------|------|
| Want | ユーザー操作を通じた統合テストで、フロントエンド + バックエンドの結合を検証する |
| How への偏り | なし。Playwright は既に技術選定済み（#127 で elm-program-test を見送り） |
| 完了基準の妥当性 | **要調整**: 「ログイン → ダッシュボード表示」はフロントエンドにログインページがないため、API 認証 + ダッシュボード表示として実施する |
| 暗黙の仮定 | ログインページの存在を前提としている → 現在は未実装 |
| スコープの適切さ | 適切。MVP として主要3フローをカバーし、CI 統合まで含む |

### 完了基準の調整

Issue の完了基準を現状に合わせて解釈する:

| 元の基準 | 調整後 |
|---------|--------|
| ログイン → ダッシュボード表示 | API 認証 → ダッシュボード（KPI）が表示される |
| 申請フォーム入力 → 送信 → 一覧に反映 | 変更なし |
| 申請承認フロー | 変更なし |

## 設計判断

### 1. ディレクトリ配置: `tests/e2e/`

既存の `tests/api/hurl/` パターンに従い、`tests/e2e/` に配置する。

代替案:
- `e2e/`（ルートレベル）: Playwright 公式のデフォルトだが、プロジェクト規約と不整合
- `frontend/` 内: フロントエンド固有のテストではなく E2E なので不適切

### 2. 認証方式: API ログイン + storageState

Playwright 推奨パターンの「API ログイン + storageState 保存」を採用する。

```typescript
// auth.setup.ts で API ログインし、Cookie 状態を .auth/ に保存
// 各テストはこの状態を再利用して認証済みで実行
```

理由:
- ログインページがないため UI 経由の認証は不可
- API ログインは高速でテスト安定性が高い
- Playwright 公式推奨のパターン

### 3. テスト実行環境: 開発サーバー

ローカルでは `just dev-all` で起動した開発サーバーに対してテストを実行する。CI では API テストと同様のパターンで専用環境を構築する。

### 4. ブラウザ: Chromium のみ（MVP）

初回は Chromium のみ。マルチブラウザテストは必要に応じて後から追加。

## 対象

- `tests/e2e/` — 新規作成（Playwright プロジェクト）
- `justfile` — `test-e2e` コマンド追加、`check-all` に統合
- `scripts/run-e2e-tests.sh` — E2E テスト実行スクリプト
- `.github/workflows/ci.yaml` — E2E テストジョブ追加
- `.gitignore` — Playwright 関連の除外パターン追加

## 対象外

- ログインページの実装（別 Issue で対応）
- マルチブラウザテスト（Firefox, WebKit）
- ビジュアルリグレッションテスト
- モバイルビューポートテスト

## ディレクトリ構造

```
tests/e2e/
├── package.json            # Playwright 依存関係
├── pnpm-lock.yaml
├── playwright.config.ts    # Playwright 設定
├── tests/
│   ├── auth.setup.ts       # 認証セットアップ（グローバル）
│   ├── dashboard.spec.ts   # ダッシュボードテスト
│   ├── workflow.spec.ts    # 申請フローテスト
│   └── approval.spec.ts    # 承認フローテスト
├── helpers/
│   ├── auth.ts             # 認証ヘルパー
│   └── test-data.ts        # テストデータ定数
└── .auth/                  # 認証状態保存（.gitignore）
```

## 実装計画

### Phase 1: Playwright セットアップ + スモークテスト

#### 確認事項
- ライブラリ: Playwright 最新安定版 → `npm show @playwright/test version` で確認
- パターン: Playwright storageState パターン → [公式ドキュメント](https://playwright.dev/docs/auth)
- 型: テストデータ定数 → `tests/api/hurl/vars.env`（BFF URL, テナント ID, ユーザー情報）

#### テストリスト
- [ ] Playwright がインストールされ、設定ファイルが存在する
- [ ] 認証セットアップ（auth.setup.ts）が API ログインで Cookie を取得し storageState に保存する
- [ ] 認証済み状態でダッシュボードページ（`/`）にアクセスできる
- [ ] ダッシュボードに KPI 統計（申請数カード）が表示される

#### 実装内容
1. `tests/e2e/` ディレクトリ作成
2. `package.json` 作成（`@playwright/test` 依存）
3. `pnpm install && npx playwright install chromium`
4. `playwright.config.ts` 作成:
   - `baseURL`: `http://localhost:15173`（Vite dev server）
   - `projects`: setup → chromium（依存チェーン）
   - `retries`: CI では 2 回、ローカルでは 0 回
5. `helpers/test-data.ts` 作成（テスト定数）
6. `helpers/auth.ts` 作成（API ログインヘルパー）
7. `tests/auth.setup.ts` 作成（storageState セットアップ）
8. `tests/dashboard.spec.ts` 作成（スモークテスト）
9. `.gitignore` 更新

### Phase 2: 申請フローテスト

#### 確認事項
- パターン: 申請フォームの DOM 構造 → `frontend/src/Page/Workflow/New.elm`
- パターン: 申請一覧の DOM 構造 → `frontend/src/Page/Workflow/List.elm`
- 型: ワークフロー定義のフォームフィールド → シードデータ（`汎用申請` 定義）

#### テストリスト
- [ ] 新規申請ページ（`/workflows/new`）にアクセスできる
- [ ] 申請フォームに入力して送信すると成功する
- [ ] 送信後、申請一覧（`/workflows`）に新しい申請が表示される

### Phase 3: 承認フローテスト

#### 確認事項
- パターン: タスク一覧の DOM 構造 → `frontend/src/Page/Task/List.elm`
- パターン: タスク詳細（承認/却下 UI）→ `frontend/src/Page/Task/Detail.elm`
- 型: 承認フローのステータス遷移 → `Data.WorkflowInstance.Status`

#### テストリスト
- [ ] 申請を作成すると、承認者のタスク一覧に表示される
- [ ] タスク詳細から承認操作を完了できる
- [ ] 承認後、申請のステータスが更新される

### Phase 4: CI 統合 + justfile

#### 確認事項
- パターン: API テスト CI ジョブ → `.github/workflows/ci.yaml`（`api-test` ジョブ）
- パターン: API テスト実行スクリプト → `scripts/run-api-tests.sh`
- ライブラリ: GitHub Actions の Playwright セットアップ → Grep 既存使用 or 公式ドキュメント

#### テストリスト
- [ ] `just test-e2e` でローカルで E2E テストが実行できる
- [ ] `just check-all` に E2E テストが含まれる
- [ ] CI で E2E テストジョブが実行される
- [ ] CI の `ci-success` ジョブに E2E テスト結果が含まれる

#### 実装内容
1. `scripts/run-e2e-tests.sh` 作成（`run-api-tests.sh` パターンを踏襲）:
   - API テスト環境（DB/Redis）を起動
   - バックエンドサービスを起動
   - Vite dev server を起動（`BFF_PORT=14000` でプロキシ先を変更）
   - ヘルスチェック待機
   - Playwright テスト実行
   - クリーンアップ
2. `justfile` に `test-e2e` レシピ追加
3. `check-all` に `test-e2e` 追加
4. `.github/workflows/ci.yaml` に `e2e-test` ジョブ追加:
   - 変更検出に `tests/e2e/**` パスを追加
   - `ci-success` に `e2e-test` 結果を追加
5. `check-tools` に `npx playwright --version` 確認追加
6. 開発環境構築手順書に Playwright を追加

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | フロントエンドにログインページが存在しない | 暗黙の仮定 | Issue 完了基準を「API 認証 + ダッシュボード表示」に調整 |
| 2回目 | Vite `server.proxy` は dev server 専用。`preview` には効かない可能性がある | 技術的前提 | CI では Vite dev server を使用する方針に変更（preview は不使用） |
| 3回目 | CI の E2E テスト実行時にフロントエンドのプロキシ先ポートが開発用のまま | 不完全なパス | `run-e2e-tests.sh` で `BFF_PORT=14000` を設定し、Vite dev server のプロキシ先をテスト用 BFF に向ける |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の完了基準 5 項目すべてに Phase が対応。セットアップ、テストヘルパー、3フロー、CI、check-all |
| 2 | 曖昧さ排除 | OK | 各 Phase のテストリストが具体的。「必要に応じて」等の曖昧表現なし |
| 3 | 設計判断の完結性 | OK | ディレクトリ配置、認証方式、実行環境、ブラウザの各判断に理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明記。ログインページ、マルチブラウザ等は対象外 |
| 5 | 技術的前提 | OK | Vite proxy の dev/preview 差異を確認、CI での Vite dev server 使用を決定 |
| 6 | 既存ドキュメント整合 | OK | ADR #127（elm-program-test 見送り）と矛盾なし。テストピラミッドの最上位層として位置づけ |

## 検証方法

1. `just dev-all` で開発サーバーを起動
2. `cd tests/e2e && pnpm test` でテスト実行
3. 全テストが pass することを確認
4. `just test-e2e` で justfile 経由の実行を確認
5. `just check-all` が通ることを確認
