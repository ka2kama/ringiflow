# Phase 2: Core Service ユースケース

## 概要

TDD でダッシュボードユースケースを実装し、Core Service にエンドポイントを追加する。

### 対応 Issue

[#38 ダッシュボード](https://github.com/ka2kama/ringiflow/issues/38)

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`usecase/dashboard.rs`](../../../backend/apps/core-service/src/usecase/dashboard.rs) | `DashboardUseCaseImpl` — 統計情報の集計ロジック |
| [`usecase.rs`](../../../backend/apps/core-service/src/usecase.rs) | モジュール登録 |
| [`handler/dashboard.rs`](../../../backend/apps/core-service/src/handler/dashboard.rs) | `get_dashboard_stats` ハンドラ |
| [`handler.rs`](../../../backend/apps/core-service/src/handler.rs) | モジュール登録 |
| [`main.rs`](../../../backend/apps/core-service/src/main.rs) | ルーティング・DI 追加 |

## 実装内容

### ユースケース

```rust
pub struct DashboardStats {
    pub pending_tasks: i64,
    pub my_workflows_in_progress: i64,
    pub completed_today: i64,
}
```

`get_stats` メソッドが `tenant_id`、`user_id`、`now`（現在時刻）を受け取り、統計を返す。

### ハンドラ

```
GET /internal/dashboard/stats?tenant_id={}&user_id={}
```

BFF から内部呼び出しされるエンドポイント。

## テスト

5 つのユニットテストを TDD で作成:

| テスト | 検証内容 |
|--------|---------|
| `test_承認待ちタスク数がactiveステップのみカウントされる` | Active ステップのみカウント、Completed/Pending は除外 |
| `test_申請中ワークフロー数がinprogressのみカウントされる` | InProgress インスタンスのみカウント |
| `test_本日完了タスク数が今日のcompleted_atのみカウントされる` | today の completed_at のみカウント、昨日は除外 |
| `test_タスクがない場合はすべて0を返す` | 空データでゼロ値が返る |
| `test_他ユーザーのデータは含まれない` | ユーザーごとのデータ分離 |

実行方法:
```bash
cd backend && cargo test --package ringiflow-core-service dashboard
```

## 設計解説

### 1. アプリケーション層でのフィルタリング

場所: [`usecase/dashboard.rs`](../../../backend/apps/core-service/src/usecase/dashboard.rs)

なぜこの設計か:
- MVP ではリポジトリに新しいクエリメソッドを追加せず、既存の `find_by_assigned_to`/`find_by_initiated_by` を活用
- 全件取得 → アプリケーション層でフィルタリングの構成
- データ量が少ない MVP フェーズでは十分なパフォーマンス

代替案:
- リポジトリに `count_by_status` メソッドを追加（SQL の COUNT + WHERE）
  - トレードオフ: パフォーマンスは良いが、MVP では過剰な最適化
  - 将来的にデータ量が増えたら移行を検討

### 2. テスタビリティのための `now` パラメータ

場所: [`usecase/dashboard.rs`](../../../backend/apps/core-service/src/usecase/dashboard.rs)

```rust
pub async fn get_stats(
    &self,
    tenant_id: TenantId,
    user_id: UserId,
    now: DateTime<Utc>,  // テスト時に固定値を渡せる
) -> Result<DashboardStats, AppError>
```

なぜこの設計か:
- 「本日完了」の判定に現在時刻が必要
- テスト内で `Utc::now()` を使うと実行タイミングに依存して不安定になる
- 時刻を引数で受け取ることで、テストで任意の日時を指定できる

代替案:
- `Clock` トレイトを導入してモック化
  - トレードオフ: より柔軟だが、このユースケースでは引数渡しで十分
- テストで日付境界を避ける
  - トレードオフ: flaky test のリスクが残る

### 3. DashboardState の分離

場所: [`handler/dashboard.rs`](../../../backend/apps/core-service/src/handler/dashboard.rs)

```rust
pub struct DashboardState<I, S> {
    pub dashboard_usecase: DashboardUseCaseImpl<I, S>,
}
```

なぜこの設計か:
- 各ハンドラは必要なユースケースのみを State として保持する
- axum の `State` エクストラクタで型安全に取得

代替案:
- 全ユースケースを持つ `AppState` に統合
  - トレードオフ: シンプルだが、ハンドラが不要な依存を持つことになる
