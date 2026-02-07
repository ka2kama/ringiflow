# Issue #115: フロントエンド ワークフロー申請フォーム - 実装計画

## 概要

Elm でワークフロー申請機能の UI を実装する。

- **前提**: バックエンド API は #35 で一部実装済み（POST のみ）
- **課題**: GET API（定義一覧、インスタンス一覧・詳細）が BFF に未実装
- **対応**: Phase 0 でバックエンド GET API を追加後、フロントエンド実装

## 前提条件の確認結果

### バックエンド実装状況

| API | リポジトリ | BFF | OpenAPI |
|-----|-----------|-----|---------|
| `POST /api/v1/workflows` | ✅ | ✅ | ✅ |
| `POST /api/v1/workflows/{id}/submit` | ✅ | ✅ | ✅ |
| `GET /api/v1/workflow-definitions` | ✅ `find_published_by_tenant` | ❌ | ❌ |
| `GET /api/v1/workflow-definitions/{id}` | ✅ `find_by_id` | ❌ | ❌ |
| `GET /api/v1/workflows` | ✅ `find_by_initiated_by` | ❌ | ❌ |
| `GET /api/v1/workflows/{id}` | ✅ `find_by_id` | ❌ | ❌ |

### フロントエンド現状

- Phase 0 完了（Main.elm, Route.elm, Ports.elm）
- `Page/` ディレクトリは空
- `elm/http` パッケージ未追加

---

## Phase 構成

### Phase 0: バックエンド GET API 追加（前提条件）

フロントエンド実装前に、不足している GET API を BFF に追加する。

**対象 API:**
1. `GET /api/v1/workflow-definitions` - ワークフロー定義一覧
2. `GET /api/v1/workflow-definitions/{id}` - ワークフロー定義詳細
3. `GET /api/v1/workflows` - ワークフロー一覧（自分の申請）
4. `GET /api/v1/workflows/{id}` - ワークフロー詳細

**作業内容:**
- Core Service: UseCase に GET メソッド追加、Handler 実装
- BFF: Handler 実装、ルーティング追加
- OpenAPI: 仕様書更新

**推定ファイル:**
- `backend/apps/core-service/src/usecase/workflow.rs`
- `backend/apps/core-service/src/handler/workflow.rs`
- `backend/apps/bff/src/handler/workflow.rs`
- `backend/apps/bff/src/router.rs`
- `openapi/openapi.yaml`

---

### Phase 1: API クライアント

**目的**: 型安全な API クライアントを実装

**タスク:**
1. `elm/http` パッケージ追加
2. データモデル定義（`Data/` モジュール）
   - `Data/WorkflowDefinition.elm` - 型 + デコーダー
   - `Data/WorkflowInstance.elm` - 型 + デコーダー
   - `Data/FormField.elm` - 動的フォームフィールド型
3. HTTP ヘルパー（`Api/Http.elm`）
   - CSRF トークン、X-Tenant-ID ヘッダー付与
   - RFC 7807 エラーレスポンスのデコード
4. API クライアント実装
   - `Api/WorkflowDefinition.elm`
   - `Api/Workflow.elm`

**新規ファイル:**
```
frontend/src/
├── Api/
│   ├── Http.elm
│   ├── Workflow.elm
│   └── WorkflowDefinition.elm
└── Data/
    ├── WorkflowDefinition.elm
    ├── WorkflowInstance.elm
    └── FormField.elm
```

---

### Phase 2: 申請フォーム UI

**目的**: ワークフロー申請フォームを実装

**タスク:**
1. ルーティング拡張（`Route.elm`）
   - `WorkflowList`, `WorkflowNew`, `WorkflowDetail` ルート追加
2. ページモジュール構造
   - `Page/Home.elm` - 既存 viewHome を移動
   - `Page/NotFound.elm` - 404 ページ
3. Main.elm のページルーティング統合
   - ページごとの Model を Union 型で管理
   - ページ遷移時の初期化処理
4. 動的フォーム生成
   - `Component/Form/DynamicField.elm`
   - フィールドタイプ別レンダリング（text, number, select, date, file）
   - バリデーション（required, minLength, maxLength, min, max）
5. 新規申請ページ（`Page/Workflow/New.elm`）
   - ワークフロー定義選択
   - タイトル入力
   - 動的フォーム表示
   - 下書き保存
   - 申請（承認者選択）

**新規ファイル:**
```
frontend/src/
├── Page/
│   ├── Home.elm
│   ├── NotFound.elm
│   └── Workflow/
│       └── New.elm
└── Component/
    └── Form/
        ├── DynamicField.elm
        └── Validation.elm
```

---

### Phase 3: 申請一覧・詳細

**目的**: 申請済みワークフローの確認機能を実装

**タスク:**
1. 申請一覧ページ（`Page/Workflow/List.elm`）
   - ワークフロー一覧取得
   - ステータス表示
   - 詳細へのリンク
2. 申請詳細ページ（`Page/Workflow/Detail.elm`）
   - ワークフロー詳細取得
   - フォームデータ表示（読み取り専用）
   - ステータス・承認ステップ表示

**新規ファイル:**
```
frontend/src/Page/Workflow/
├── List.elm
└── Detail.elm
```

---

## 設計方針

### ページモジュール構造

```elm
-- Main.elm
type PageModel
    = HomePage Home.Model
    | WorkflowListPage WorkflowList.Model
    | WorkflowNewPage WorkflowNew.Model
    | WorkflowDetailPage WorkflowDetail.Model
    | NotFoundPage
```

採用理由:
- メモリ効率（現在のページの Model のみ保持）
- 型システムで「あり得ない状態」を表現不可能に

### 動的フォーム生成

```
WorkflowDefinition.definition.form.fields
  ↓ デコード
List FormField
  ↓ レンダリング
Html Msg (動的生成された UI)
  ↓ 入力
Dict String String (フィールドID → 値)
  ↓ エンコード
Json.Encode.Value (API 送信用)
```

### エラーハンドリング

- 401 エラー: Main で処理（ログイン画面へリダイレクト）
- 400/404 エラー: 各ページで処理（エラーメッセージ表示）
- 500 エラー: グローバル通知

---

## テスト戦略

### ユニットテスト（elm-test）

| 対象 | テスト内容 |
|------|-----------|
| `Data/*.elm` | JSON デコーダー |
| `Route.elm` | URL パース / 生成 |
| `Component/Form/Validation.elm` | バリデーションロジック |

### 手動テスト

Phase 3 完了時に実施:
1. ワークフロー定義選択 → フォーム表示
2. フォーム入力 → 下書き保存
3. 承認者選択 → 申請
4. 申請一覧表示
5. 申請詳細表示

---

## 主要ファイル

### 変更対象
- `frontend/elm.json` - `elm/http` 追加
- `frontend/src/Main.elm` - ページルーティング統合
- `frontend/src/Route.elm` - ルート追加

### 新規作成（完全一覧）
```
frontend/src/
├── Api/
│   ├── Http.elm
│   ├── Workflow.elm
│   └── WorkflowDefinition.elm
├── Data/
│   ├── WorkflowDefinition.elm
│   ├── WorkflowInstance.elm
│   └── FormField.elm
├── Page/
│   ├── Home.elm
│   ├── NotFound.elm
│   └── Workflow/
│       ├── List.elm
│       ├── New.elm
│       └── Detail.elm
└── Component/
    └── Form/
        ├── DynamicField.elm
        └── Validation.elm
```

---

## 検証手順

### Phase 1 完了時
```bash
cd frontend && pnpm run test   # デコーダーテスト
cd frontend && pnpm run lint   # elm-format + elm-review
```

### Phase 2 完了時
```bash
just dev-deps                  # DB, Redis 起動
just dev                       # BFF + フロントエンド起動
# ブラウザで http://localhost:15173/workflows/new にアクセス
# ワークフロー定義選択 → フォーム表示 → 下書き保存を確認
```

### Phase 3 完了時
```bash
# ブラウザで http://localhost:15173/workflows にアクセス
# 申請一覧表示 → 詳細表示を確認
just check-all                 # 全体チェック
```

---

## 決定事項

**Phase 0（バックエンド GET API 追加）**: Issue #115 のスコープ内で実装

実装順序: Phase 0 → Phase 1 → Phase 2 → Phase 3
