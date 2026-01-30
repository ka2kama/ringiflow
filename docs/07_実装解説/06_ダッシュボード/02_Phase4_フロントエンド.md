# Phase 4: フロントエンド（Elm）

## 概要

ホームページを Stateless から Stateful に変換し、ダッシュボード API から KPI 統計情報を取得・表示する。

### 対応 Issue

[#38 ダッシュボード](https://github.com/ka2kama/ringiflow/issues/38)

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`Data/Dashboard.elm`](../../../frontend/src/Data/Dashboard.elm)（新規） | `DashboardStats` 型 + JSON デコーダー |
| [`Api/Dashboard.elm`](../../../frontend/src/Api/Dashboard.elm)（新規） | `getStats` API クライアント |
| [`Page/Home.elm`](../../../frontend/src/Page/Home.elm) | Stateful ダッシュボードページ |
| [`Main.elm`](../../../frontend/src/Main.elm) | Nested TEA 統合 |

## 実装内容

### 1. データモジュール

```elm
type alias DashboardStats =
    { pendingTasks : Int
    , myWorkflowsInProgress : Int
    , completedToday : Int
    }
```

レスポンスの `{ "data": { ... } }` エンベロープに対応するデコーダーを定義。

### 2. Home ページの Stateful 化

変更前（Stateless）:
```elm
-- Main.elm
type Page = HomePage | ...
```

変更後（Stateful）:
```elm
-- Main.elm
type Page = HomePage Home.Model | ...
type Msg = HomeMsg Home.Msg | ...
```

Home ページが独自の Model / Msg / update / view を持つ Nested TEA パターンに変換。

### 3. RemoteData パターン

```elm
type RemoteData a
    = Loading
    | Failure
    | Success a
```

API 呼び出しの状態を型で管理し、各状態に応じた UI を描画する。

### 4. Main.elm の変更

Nested TEA 統合のため、以下の箇所を更新:

| 箇所 | 変更内容 |
|------|---------|
| `Page` 型 | `HomePage` → `HomePage Home.Model` |
| `Msg` 型 | `HomeMsg Home.Msg` を追加 |
| `initPage` | `Home.init shared` を呼び出し |
| `update` | `HomeMsg` のディスパッチ |
| `updatePageShared` | `Home.updateShared` を呼び出し |
| `viewMain` | `Home.view |> Html.map HomeMsg` |

## 設計解説

### 1. Page.Home の Stateful 化

場所: [`Page/Home.elm`](../../../frontend/src/Page/Home.elm)

なぜこの設計か:
- ダッシュボードは API からデータを取得するため、状態管理が必要
- 他のページ（WorkflowList, TaskList 等）と同じパターンを踏襲
- `init` で API リクエストを発行し、結果を Model に保持

代替案:
- Main.elm で直接 API を呼び、Home にプロパティとして渡す
  - トレードオフ: ページ固有のロジックが Main に漏れる。Nested TEA パターンに反する
- `elm-community/remote-data` パッケージを使用
  - トレードオフ: `NotAsked` 状態は今回不要（init 時に即座にリクエスト）。自前の 3 値で十分

### 2. import `ApiError` vs `ApiError(..)`

場所: [`Page/Home.elm`](../../../frontend/src/Page/Home.elm)

```elm
-- 採用: 型のみインポート
import Api exposing (ApiError)

-- 不採用: コンストラクタも公開
import Api exposing (ApiError(..))
```

なぜこの設計か:
- `Msg` の型注釈で `Result ApiError DashboardStats` として型名のみ使用
- コンストラクタ（`BadRequest`, `Unauthorized` 等）の pattern matching は Home ページで不要
- 不要なコンストラクタの公開はモジュール境界の制御を弱める

### 3. KPI カードのプレースホルダー実装

場所: [`Page/Home.elm`](../../../frontend/src/Page/Home.elm) の `viewStatsCards` / `viewStatCard`

なぜこの設計か:
- データ表示の基盤（API → Model → View のパイプライン）が機能することの確認を優先
- デザイン改善は別途対応可能

## 次のステップ

- KPI カードのデザイン改善（TODO(human) として残存）
- E2E 動作確認
