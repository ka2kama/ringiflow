module Page.Home exposing (Model, Msg, init, update, updateShared, view)

{-| ホームページ（ダッシュボード）

アプリケーションのトップページ。
KPI 統計情報（承認待ち、申請中、本日完了）とクイックアクションを表示する。

-}

import Api exposing (ApiError)
import Api.Dashboard as DashboardApi
import Component.LoadingSpinner as LoadingSpinner
import Data.Dashboard exposing (DashboardStats)
import Html exposing (..)
import Html.Attributes exposing (..)
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)



-- MODEL


{-| ダッシュボード画面の状態

RemoteData パターンで API 呼び出しの状態を管理する。

-}
type alias Model =
    { shared : Shared
    , stats : RemoteData ApiError DashboardStats
    }


{-| 初期化: API から統計情報を取得
-}
init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , stats = Loading
      }
    , DashboardApi.getStats
        { config = Shared.toRequestConfig shared
        , toMsg = GotDashboardStats
        }
    )


{-| Shared の更新を反映
-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


type Msg
    = GotDashboardStats (Result ApiError DashboardStats)


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDashboardStats result ->
            case result of
                Ok stats ->
                    ( { model | stats = Success stats }, Cmd.none )

                Err err ->
                    ( { model | stats = Failure err }, Cmd.none )



-- VIEW


{-| ダッシュボード画面の描画
-}
view : Model -> Html Msg
view model =
    div []
        [ h2 [ class "mb-6 text-2xl font-bold text-secondary-900" ]
            [ text "ダッシュボード" ]
        , viewStats model.stats
        , viewQuickActions
        ]


{-| KPI 統計カードの表示

RemoteData パターンで Loading / Failure / Success を切り替える。

-}
viewStats : RemoteData ApiError DashboardStats -> Html Msg
viewStats remoteStats =
    case remoteStats of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure _ ->
            div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
                [ text "統計情報の取得に失敗しました" ]

        Success stats ->
            -- TODO(human): KPI カードのデザインを実装してください
            viewStatsCards stats


{-| KPI カードの描画

3 つの統計値をカードとして横並びに表示する。

-}
viewStatsCards : DashboardStats -> Html Msg
viewStatsCards stats =
    div [ class "mt-4 grid gap-4 sm:grid-cols-3" ]
        [ viewStatCard "承認待ちタスク" stats.pendingTasks "bg-primary-50" "text-primary-600"
        , viewStatCard "申請中" stats.myWorkflowsInProgress "bg-warning-50" "text-warning-600"
        , viewStatCard "本日完了" stats.completedToday "bg-success-50" "text-success-600"
        ]


{-| 統計カード（単体）

TODO(human): カードのデザインを改善してください

-}
viewStatCard : String -> Int -> String -> String -> Html Msg
viewStatCard label value bgColorClass textColorClass =
    div [ class ("rounded-xl p-6 text-center " ++ bgColorClass) ]
        [ div [ class ("text-3xl font-bold " ++ textColorClass) ]
            [ text (String.fromInt value) ]
        , div [ class "mt-2 text-sm text-secondary-500" ]
            [ text label ]
        ]


{-| クイックアクションエリア
-}
viewQuickActions : Html msg
viewQuickActions =
    div [ class "mt-6 flex flex-wrap gap-3" ]
        [ a
            [ href (Route.toString Route.Workflows)
            , class "inline-flex items-center rounded-lg bg-success-600 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-success-700"
            ]
            [ text "申請一覧" ]
        , a
            [ href (Route.toString Route.WorkflowNew)
            , class "inline-flex items-center rounded-lg bg-primary-600 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-primary-700"
            ]
            [ text "新規申請" ]
        , a
            [ href (Route.toString Route.Tasks)
            , class "inline-flex items-center rounded-lg bg-warning-600 px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-warning-700"
            ]
            [ text "タスク一覧" ]
        ]
