module Page.Task.List exposing
    ( Model
    , Msg
    , init
    , update
    , updateShared
    , view
    )

{-| タスク一覧ページ

自分にアサインされた承認待ちタスクの一覧を表示する。


## 機能

  - タスク一覧の表示（テーブル形式）
  - 各タスクの申請タイトル、ステップ名、ステータス、期限を表示
  - 詳細ページへの遷移

-}

import Api exposing (ApiError)
import Api.Task as TaskApi
import Component.LoadingSpinner as LoadingSpinner
import Data.Task exposing (TaskItem)
import Data.WorkflowInstance as WorkflowInstance
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)
import Time
import Util.DateFormat as DateFormat



-- MODEL


{-| ページの状態
-}
type alias Model =
    { shared : Shared
    , tasks : RemoteData ApiError (List TaskItem)
    }


{-| 初期化
-}
init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , tasks = Loading
      }
    , TaskApi.listMyTasks
        { config = Shared.toRequestConfig shared
        , toMsg = GotTasks
        }
    )


{-| 共有状態を更新
-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



-- UPDATE


{-| メッセージ
-}
type Msg
    = GotTasks (Result ApiError (List TaskItem))
    | Refresh


{-| 状態更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotTasks result ->
            case result of
                Ok tasks ->
                    ( { model | tasks = Success tasks }
                    , Cmd.none
                    )

                Err err ->
                    ( { model | tasks = Failure err }
                    , Cmd.none
                    )

        Refresh ->
            ( { model | tasks = Loading }
            , TaskApi.listMyTasks
                { config = Shared.toRequestConfig model.shared
                , toMsg = GotTasks
                }
            )



-- VIEW


{-| ビュー
-}
view : Model -> Html Msg
view model =
    div []
        [ viewHeader
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ] [ text "タスク一覧" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.tasks of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure _ ->
            viewError

        Success tasks ->
            viewTaskList (Shared.zone model.shared) tasks


viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "mt-2 inline-flex items-center rounded-lg border border-secondary-100 px-4 py-2 text-sm font-medium text-secondary-700 transition-colors hover:bg-secondary-50" ]
            [ text "再読み込み" ]
        ]


viewTaskList : Time.Zone -> List TaskItem -> Html Msg
viewTaskList zone tasks =
    if List.isEmpty tasks then
        div [ class "py-12 text-center" ]
            [ p [ class "text-secondary-500" ] [ text "承認待ちのタスクはありません" ]
            , p [ class "mt-2 text-sm text-secondary-400" ] [ text "新しいタスクが割り当てられるとここに表示されます" ]
            ]

    else
        div []
            [ div [ class "overflow-x-auto" ] [ viewTaskTable zone tasks ]
            , viewCount (List.length tasks)
            ]


viewTaskTable : Time.Zone -> List TaskItem -> Html Msg
viewTaskTable zone tasks =
    table [ class "w-full" ]
        [ thead [ class "border-b border-secondary-100" ]
            [ tr []
                [ th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "ステップ名" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "申請タイトル" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "ステータス" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "期限" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "開始日" ]
                ]
            ]
        , tbody []
            (List.map (viewTaskRow zone) tasks)
        ]


viewTaskRow : Time.Zone -> TaskItem -> Html Msg
viewTaskRow zone task =
    tr [ class "border-b border-secondary-100" ]
        [ td [ class "px-4 py-3" ]
            [ a [ href (Route.toString (Route.TaskDetail task.id)), class "text-primary-600 hover:text-primary-700 hover:underline" ]
                [ text task.stepName ]
            ]
        , td [ class "px-4 py-3" ] [ text task.workflow.title ]
        , td [ class "px-4 py-3" ]
            [ span [ class ("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium " ++ WorkflowInstance.stepStatusToCssClass task.status) ]
                [ text (WorkflowInstance.stepStatusToJapanese task.status) ]
            ]
        , td [ class "px-4 py-3" ] [ text (DateFormat.formatMaybeDate zone task.dueDate) ]
        , td [ class "px-4 py-3" ] [ text (DateFormat.formatMaybeDate zone task.startedAt) ]
        ]


viewCount : Int -> Html Msg
viewCount count =
    div [ class "mt-4 text-sm text-secondary-500" ]
        [ text ("全 " ++ String.fromInt count ++ " 件") ]
