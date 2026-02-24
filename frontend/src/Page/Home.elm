module Page.Home exposing (Model, Msg, init, update, updateShared, view)

{-| ホームページ（ダッシュボード）

アプリケーションのトップページ。
KPI 統計情報（承認待ち、申請中、本日完了）とクイックアクションを表示する。

-}

import Api exposing (ApiError)
import Api.Dashboard as DashboardApi
import Component.Button as Button
import Component.ErrorState as ErrorState
import Component.Icons as Icons
import Component.LoadingSpinner as LoadingSpinner
import Data.Dashboard exposing (DashboardStats)
import Data.WorkflowInstance exposing (Status(..))
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
        [ h1 [ class "mb-6 text-2xl font-bold text-secondary-900" ]
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
            ErrorState.viewSimple "統計情報の取得に失敗しました"

        Success stats ->
            viewStatsCards stats


{-| KPI カードの描画

3 つの統計値をクリック可能なカードとして横並びに表示する。
各カードは対応するフィルタ付き一覧ページにリンクする。

-}
viewStatsCards : DashboardStats -> Html Msg
viewStatsCards stats =
    div [ class "grid gap-4 sm:grid-cols-3" ]
        [ viewStatCardLink
            { label = "承認待ちタスク"
            , value = stats.pendingTasks
            , bgColorClass = "bg-primary-50"
            , textColorClass = "text-primary-600"
            , route = Route.Tasks
            , icon = Icons.tasks
            }
        , viewStatCardLink
            { label = "申請中"
            , value = stats.myWorkflowsInProgress
            , bgColorClass = "bg-warning-50"
            , textColorClass = "text-warning-600"
            , route = Route.Workflows { status = Just InProgress, completedToday = False }
            , icon = Icons.workflows
            }
        , viewStatCardLink
            { label = "本日完了"
            , value = stats.completedToday
            , bgColorClass = "bg-success-50"
            , textColorClass = "text-success-600"
            , route = Route.Workflows { status = Nothing, completedToday = True }
            , icon = Icons.checkCircle
            }
        ]


{-| クリック可能な統計カード

`<a>` 要素としてレンダリングし、対応するフィルタ付きページにリンクする。
ホバー時にシャドウエフェクトでクリック可能であることを示す。

-}
viewStatCardLink :
    { label : String
    , value : Int
    , bgColorClass : String
    , textColorClass : String
    , route : Route.Route
    , icon : Html Msg
    }
    -> Html Msg
viewStatCardLink config =
    a
        [ href (Route.toString config.route)
        , class ("block rounded-lg border border-secondary-200 p-6 text-center no-underline shadow-sm transition-colors hover:shadow-md " ++ config.bgColorClass)
        ]
        [ div [ class ("mb-2 flex justify-center " ++ config.textColorClass) ]
            [ config.icon ]
        , div [ class ("text-3xl font-bold " ++ config.textColorClass) ]
            [ text (String.fromInt config.value) ]
        , div [ class "mt-2 text-sm text-secondary-500" ]
            [ text config.label ]
        ]


{-| クイックアクションエリア
-}
viewQuickActions : Html msg
viewQuickActions =
    div [ class "mt-6 flex flex-wrap gap-3" ]
        [ Button.link
            { variant = Button.Success
            , href = Route.toString (Route.Workflows Route.emptyWorkflowFilter)
            }
            [ text "申請一覧" ]
        , Button.link
            { variant = Button.Primary
            , href = Route.toString Route.WorkflowNew
            }
            [ text "新規申請" ]
        , Button.link
            { variant = Button.Warning
            , href = Route.toString Route.Tasks
            }
            [ text "タスク一覧" ]
        ]
