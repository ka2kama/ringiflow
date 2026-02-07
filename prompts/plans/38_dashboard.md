# Issue #38: ダッシュボード実装計画

## 概要

Home ページ（`/`）をダッシュボード化し、承認待ちタスク数・申請中ワークフロー数・本日完了数を表示する。

## MVP スコープ

| KPI ID | 指標 | データソース |
|--------|------|------------|
| KPI-001 | 承認待ちタスク数 | `workflow_steps` (Active, assigned_to = user) |
| KPI-004 | 申請中ワークフロー数 | `workflow_instances` (Submitted, initiated_by = user) |
| — | 本日完了タスク数 | `workflow_steps` (completed_at = today, assigned_to = user) |

Redis キャッシュは MVP では不要。DB 直接クエリで実装。

## 設計判断

**Home ページのダッシュボード化**（別ルート `/dashboard` は作らない）
- 基本設計書で `Pages/Home.elm` がダッシュボード画面
- Home.elm のコメントに「将来の拡張: ダッシュボード」と明記
- `Page/Home.elm` を stateless → stateful に変換

## Phase 分割

### Phase 1: 設計・OpenAPI 更新

OpenAPI 仕様書にダッシュボード API を追加。

変更ファイル:
- `openapi/openapi.yaml`

### Phase 2: Core Service（ユースケース + ハンドラ）

TDD で `DashboardUseCaseImpl` を実装。

変更ファイル:
- `backend/apps/core-service/src/usecase/dashboard.rs`（新規）
- `backend/apps/core-service/src/usecase/mod.rs`
- `backend/apps/core-service/src/handler/dashboard.rs`（新規）
- `backend/apps/core-service/src/handler/mod.rs`
- `backend/apps/core-service/src/main.rs`

テストリスト:
- [ ] 承認待ちタスク数が正しく返る（Active ステップのみカウント）
- [ ] 申請中ワークフロー数が正しく返る（Submitted インスタンスのみカウント）
- [ ] 本日完了タスク数が正しく返る（today の completed_at のみカウント）
- [ ] タスクがない場合はすべて 0 を返す
- [ ] 他ユーザーのデータは含まれない

### Phase 3: BFF（クライアント + ハンドラ）

変更ファイル:
- `backend/apps/bff/src/client/core_service.rs`
- `backend/apps/bff/src/handler/dashboard.rs`（新規）
- `backend/apps/bff/src/handler/mod.rs`
- `backend/apps/bff/src/main.rs`

テストリスト:
- [ ] GET /api/v1/dashboard/stats が 200 を返す
- [ ] 未認証の場合は 401 を返す

### Phase 4: フロントエンド

変更ファイル:
- `frontend/src/Data/Dashboard.elm`（新規）
- `frontend/src/Api/Dashboard.elm`（新規）
- `frontend/src/Page/Home.elm`（stateful に変換）
- `frontend/src/Main.elm`

実装内容:
- `Data/Dashboard.elm`: DashboardStats 型 + JSON デコーダー
- `Api/Dashboard.elm`: getStats API クライアント
- `Page/Home.elm`: Model/Msg/init/update/updateShared/view を追加
  - init で API 呼び出し
  - RemoteData パターンで Loading/Failure/Success を管理
  - KPI カードを表示（承認待ち、申請中、本日完了）
  - クイックアクション（既存）を維持
- `Main.elm`:
  - `HomePage` → `HomePage Home.Model`
  - `HomeMsg Home.Msg` を追加
  - initPage, update, updatePageShared, viewMain を更新

### Phase 5: 統合・仕上げ

- `just check-all` で全体チェック
- E2E 動作確認（ブラウザでダッシュボードに統計が表示されること）

## API レスポンス

```
GET /api/v1/dashboard/stats

{
  "data": {
    "pending_tasks": 3,
    "my_workflows_in_progress": 2,
    "completed_today": 5
  }
}
```

## 検証方法

1. `just check-all`（lint + テスト）
2. `just dev-all` で開発サーバー起動
3. ブラウザでログイン → `/` にダッシュボードが表示されることを確認
4. 統計値が API レスポンスと一致することを確認
