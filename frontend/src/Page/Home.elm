module Page.Home exposing (Model, Msg, init, update, updateShared, view)

{-| ホームページ（ダッシュボード）

アプリケーションのトップページ。
KPI 統計情報（承認待ち、申請中、本日完了）とクイックアクションを表示する。

-}

import Api exposing (ApiError)
import Api.Dashboard as DashboardApi
import Data.Dashboard exposing (DashboardStats)
import Html exposing (..)
import Html.Attributes exposing (..)
import Shared exposing (Shared)



-- MODEL


{-| ダッシュボード画面の状態

RemoteData パターンで API 呼び出しの状態を管理する。

-}
type RemoteData a
    = Loading
    | Failure
    | Success a


type alias Model =
    { shared : Shared
    , stats : RemoteData DashboardStats
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

                Err _ ->
                    ( { model | stats = Failure }, Cmd.none )



-- VIEW


{-| ダッシュボード画面の描画
-}
view : Model -> Html Msg
view model =
    div []
        [ h2 [] [ text "ダッシュボード" ]
        , viewStats model.stats
        , viewQuickActions
        ]


{-| KPI 統計カードの表示

RemoteData パターンで Loading / Failure / Success を切り替える。

-}
viewStats : RemoteData DashboardStats -> Html Msg
viewStats remoteStats =
    case remoteStats of
        Loading ->
            div
                [ style "padding" "2rem"
                , style "text-align" "center"
                , style "color" "#5f6368"
                ]
                [ text "統計情報を読み込み中..." ]

        Failure ->
            div
                [ style "padding" "1.5rem"
                , style "background-color" "#fce8e6"
                , style "border-radius" "8px"
                , style "color" "#c5221f"
                ]
                [ text "統計情報の取得に失敗しました" ]

        Success stats ->
            -- TODO(human): KPI カードを実装する
            viewStatsCards stats


{-| KPI カードの描画

3 つの統計値をカードとして横並びに表示する。

-}
viewStatsCards : DashboardStats -> Html Msg
viewStatsCards stats =
    -- TODO(human): KPI カードのデザインを実装してください
    -- 現在はプレースホルダーとして数値のみ表示
    div
        [ style "display" "flex"
        , style "gap" "1rem"
        , style "margin-top" "1rem"
        ]
        [ viewStatCard "承認待ちタスク" stats.pendingTasks "#e8f0fe" "#1a73e8"
        , viewStatCard "申請中" stats.myWorkflowsInProgress "#fef7e0" "#ea8600"
        , viewStatCard "本日完了" stats.completedToday "#e6f4ea" "#34a853"
        ]


{-| 統計カード（単体）

TODO(human): カードのデザインを改善してください

-}
viewStatCard : String -> Int -> String -> String -> Html Msg
viewStatCard label value bgColor textColor =
    div
        [ style "flex" "1"
        , style "padding" "1.5rem"
        , style "background-color" bgColor
        , style "border-radius" "8px"
        , style "text-align" "center"
        ]
        [ div
            [ style "font-size" "2rem"
            , style "font-weight" "bold"
            , style "color" textColor
            ]
            [ text (String.fromInt value) ]
        , div
            [ style "margin-top" "0.5rem"
            , style "color" "#5f6368"
            ]
            [ text label ]
        ]


{-| クイックアクションエリア
-}
viewQuickActions : Html msg
viewQuickActions =
    div
        [ style "display" "flex"
        , style "gap" "1rem"
        , style "margin-top" "1.5rem"
        ]
        [ a
            [ href "/workflows"
            , style "display" "inline-block"
            , style "padding" "0.75rem 1.5rem"
            , style "background-color" "#34a853"
            , style "color" "white"
            , style "text-decoration" "none"
            , style "border-radius" "4px"
            ]
            [ text "申請一覧" ]
        , a
            [ href "/workflows/new"
            , style "display" "inline-block"
            , style "padding" "0.75rem 1.5rem"
            , style "background-color" "#1a73e8"
            , style "color" "white"
            , style "text-decoration" "none"
            , style "border-radius" "4px"
            ]
            [ text "新規申請" ]
        , a
            [ href "/tasks"
            , style "display" "inline-block"
            , style "padding" "0.75rem 1.5rem"
            , style "background-color" "#ea8600"
            , style "color" "white"
            , style "text-decoration" "none"
            , style "border-radius" "4px"
            ]
            [ text "タスク一覧" ]
        ]
