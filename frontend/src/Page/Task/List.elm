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
import Data.Task exposing (TaskItem)
import Data.WorkflowInstance as WorkflowInstance
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick)
import Route
import Shared exposing (Shared)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { shared : Shared
    , tasks : RemoteData (List TaskItem)
    }


{-| リモートデータの状態
-}
type RemoteData a
    = Loading
    | Failure
    | Success a


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

                Err _ ->
                    ( { model | tasks = Failure }
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
        Loading ->
            div [ class "py-8 text-center text-secondary-500" ] [ text "読み込み中..." ]

        Failure ->
            viewError

        Success tasks ->
            viewTaskList tasks


viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "mt-2 inline-flex items-center rounded-lg border border-secondary-100 px-4 py-2 text-sm font-medium text-secondary-700 transition-colors hover:bg-secondary-50" ]
            [ text "再読み込み" ]
        ]


viewTaskList : List TaskItem -> Html Msg
viewTaskList tasks =
    if List.isEmpty tasks then
        div [ class "py-8 text-center text-secondary-500" ] [ text "承認待ちのタスクはありません" ]

    else
        div []
            [ viewTaskTable tasks
            , viewCount (List.length tasks)
            ]


viewTaskTable : List TaskItem -> Html Msg
viewTaskTable tasks =
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
            (List.map viewTaskRow tasks)
        ]


viewTaskRow : TaskItem -> Html Msg
viewTaskRow task =
    tr [ class "border-b border-secondary-100" ]
        [ td [ class "px-4 py-3 text-sm" ]
            [ a [ href (Route.toString (Route.TaskDetail task.id)), class "text-primary-600 hover:text-primary-700 hover:underline" ]
                [ text task.stepName ]
            ]
        , td [ class "px-4 py-3 text-sm" ] [ text task.workflow.title ]
        , td [ class "px-4 py-3 text-sm" ]
            [ span [ class ("inline-flex items-center rounded-full px-2.5 py-0.5 text-xs font-medium " ++ stepStatusToCssClass task.status) ]
                [ text (WorkflowInstance.stepStatusToJapanese task.status) ]
            ]
        , td [ class "px-4 py-3 text-sm" ] [ text (formatMaybeDate task.dueDate) ]
        , td [ class "px-4 py-3 text-sm" ] [ text (formatMaybeDate task.startedAt) ]
        ]


viewCount : Int -> Html Msg
viewCount count =
    div [ class "mt-4 text-sm text-secondary-500" ]
        [ text ("全 " ++ String.fromInt count ++ " 件") ]



-- HELPERS


{-| ステップステータスを CSS クラス名に変換
-}
stepStatusToCssClass : WorkflowInstance.StepStatus -> String
stepStatusToCssClass status =
    case status of
        WorkflowInstance.StepPending ->
            "bg-gray-100 text-gray-600"

        WorkflowInstance.StepActive ->
            "bg-warning-50 text-warning-600"

        WorkflowInstance.StepCompleted ->
            "bg-success-50 text-success-600"

        WorkflowInstance.StepSkipped ->
            "bg-secondary-100 text-secondary-500"


{-| Maybe な日時文字列から日付部分を抽出
-}
formatMaybeDate : Maybe String -> String
formatMaybeDate maybeDate =
    case maybeDate of
        Just isoString ->
            String.left 10 isoString

        Nothing ->
            "-"
