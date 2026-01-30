module Data.Dashboard exposing
    ( DashboardStats
    , decoder
    )

{-| ダッシュボード統計データ型

ホーム画面に表示する KPI 統計情報を定義する。


## 統計項目

  - `pendingTasks`: 承認待ちタスク数
  - `myWorkflowsInProgress`: 申請中ワークフロー数
  - `completedToday`: 本日完了タスク数

-}

import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (required)



-- TYPES


{-| ダッシュボード統計情報
-}
type alias DashboardStats =
    { pendingTasks : Int
    , myWorkflowsInProgress : Int
    , completedToday : Int
    }



-- DECODERS


{-| DashboardStats のデコーダー

`{ "data": { "pending_tasks": 3, ... } }` 形式のレスポンスをデコードする。

-}
decoder : Decoder DashboardStats
decoder =
    Decode.field "data" statsDecoder


statsDecoder : Decoder DashboardStats
statsDecoder =
    Decode.succeed DashboardStats
        |> required "pending_tasks" Decode.int
        |> required "my_workflows_in_progress" Decode.int
        |> required "completed_today" Decode.int
