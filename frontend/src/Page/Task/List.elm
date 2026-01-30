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
    div [ class "task-list-page" ]
        [ viewHeader
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "page-header" ]
        [ h1 [] [ text "タスク一覧" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.tasks of
        Loading ->
            div [ class "loading" ] [ text "読み込み中..." ]

        Failure ->
            viewError

        Success tasks ->
            viewTaskList tasks


viewError : Html Msg
viewError =
    div [ class "error-message" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "btn btn-secondary" ]
            [ text "再読み込み" ]
        ]


viewTaskList : List TaskItem -> Html Msg
viewTaskList tasks =
    if List.isEmpty tasks then
        div [ class "empty-message" ] [ text "承認待ちのタスクはありません" ]

    else
        div []
            [ viewTaskTable tasks
            , viewCount (List.length tasks)
            ]


viewTaskTable : List TaskItem -> Html Msg
viewTaskTable tasks =
    table [ class "task-table" ]
        [ thead []
            [ tr []
                [ th [] [ text "ステップ名" ]
                , th [] [ text "申請タイトル" ]
                , th [] [ text "ステータス" ]
                , th [] [ text "期限" ]
                , th [] [ text "開始日" ]
                ]
            ]
        , tbody []
            (List.map viewTaskRow tasks)
        ]


viewTaskRow : TaskItem -> Html Msg
viewTaskRow task =
    tr []
        [ td []
            [ a [ href (Route.toString (Route.TaskDetail task.id)) ]
                [ text task.stepName ]
            ]
        , td [] [ text task.workflow.title ]
        , td []
            [ span [ class (stepStatusToCssClass task.status) ]
                [ text (WorkflowInstance.stepStatusToJapanese task.status) ]
            ]
        , td [] [ text (formatMaybeDate task.dueDate) ]
        , td [] [ text (formatMaybeDate task.startedAt) ]
        ]


viewCount : Int -> Html Msg
viewCount count =
    div [ class "task-count" ]
        [ text ("全 " ++ String.fromInt count ++ " 件") ]



-- HELPERS


{-| ステップステータスを CSS クラス名に変換
-}
stepStatusToCssClass : WorkflowInstance.StepStatus -> String
stepStatusToCssClass status =
    case status of
        WorkflowInstance.StepPending ->
            "status-pending"

        WorkflowInstance.StepActive ->
            "status-active"

        WorkflowInstance.StepCompleted ->
            "status-completed"

        WorkflowInstance.StepSkipped ->
            "status-skipped"


{-| Maybe な日時文字列から日付部分を抽出
-}
formatMaybeDate : Maybe String -> String
formatMaybeDate maybeDate =
    case maybeDate of
        Just isoString ->
            String.left 10 isoString

        Nothing ->
            "-"
