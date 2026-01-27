module Page.Workflow.List exposing
    ( Model
    , Msg
    , init
    , update
    , view
    )

{-| 申請一覧ページ

自分が申請したワークフローインスタンスの一覧を表示する。


## 機能

  - 申請一覧の表示
  - ステータスによるフィルタリング
  - 詳細ページへの遷移


## 設計

詳細: [申請フォーム UI 設計](../../../../docs/03_詳細設計書/10_ワークフロー申請フォームUI設計.md)

-}

import Api.Http exposing (ApiError)
import Api.Workflow as WorkflowApi
import Data.WorkflowInstance as WorkflowInstance exposing (Status, WorkflowInstance)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, onInput)
import Route
import Session exposing (Session)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { -- セッション（API 呼び出しに必要）
      session : Session

    -- API データ
    , workflows : RemoteData (List WorkflowInstance)

    -- フィルタ状態
    , statusFilter : Maybe Status
    }


{-| リモートデータの状態
-}
type RemoteData a
    = NotAsked
    | Loading
    | Failure ApiError
    | Success a


{-| 初期化
-}
init : Session -> ( Model, Cmd Msg )
init session =
    ( { session = session
      , workflows = Loading
      , statusFilter = Nothing
      }
    , WorkflowApi.listMyWorkflows
        { config = Session.toRequestConfig session
        , toMsg = GotWorkflows
        }
    )



-- UPDATE


{-| メッセージ
-}
type Msg
    = GotWorkflows (Result ApiError (List WorkflowInstance))
    | SetStatusFilter (Maybe Status)
    | Refresh


{-| 状態更新
-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotWorkflows result ->
            case result of
                Ok workflows ->
                    ( { model | workflows = Success workflows }
                    , Cmd.none
                    )

                Err error ->
                    ( { model | workflows = Failure error }
                    , Cmd.none
                    )

        SetStatusFilter maybeStatus ->
            ( { model | statusFilter = maybeStatus }
            , Cmd.none
            )

        Refresh ->
            ( { model | workflows = Loading }
            , WorkflowApi.listMyWorkflows
                { config = Session.toRequestConfig model.session
                , toMsg = GotWorkflows
                }
            )



-- VIEW


{-| ビュー
-}
view : Model -> Html Msg
view model =
    div [ class "workflow-list-page" ]
        [ viewHeader
        , viewContent model
        ]


viewHeader : Html Msg
viewHeader =
    div [ class "page-header" ]
        [ h1 [] [ text "申請一覧" ]
        , a [ href (Route.toString Route.WorkflowNew), class "btn btn-primary" ]
            [ text "+ 新規申請" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.workflows of
        NotAsked ->
            div [] []

        Loading ->
            div [ class "loading" ] [ text "読み込み中..." ]

        Failure error ->
            viewError error

        Success workflows ->
            viewWorkflowList model.statusFilter workflows


viewError : ApiError -> Html Msg
viewError _ =
    div [ class "error-message" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , button [ onClick Refresh, class "btn btn-secondary" ]
            [ text "再読み込み" ]
        ]


viewWorkflowList : Maybe Status -> List WorkflowInstance -> Html Msg
viewWorkflowList statusFilter workflows =
    let
        filteredWorkflows =
            case statusFilter of
                Nothing ->
                    workflows

                Just status ->
                    List.filter (\w -> w.status == status) workflows
    in
    div [ class "workflow-list-content" ]
        [ viewStatusFilter statusFilter
        , if List.isEmpty filteredWorkflows then
            div [ class "empty-message" ] [ text "申請がありません" ]

          else
            div []
                [ viewWorkflowTable filteredWorkflows
                , viewCount (List.length filteredWorkflows)
                ]
        ]


viewStatusFilter : Maybe Status -> Html Msg
viewStatusFilter currentFilter =
    let
        statusOptions =
            [ ( Nothing, "すべて" )
            , ( Just WorkflowInstance.Draft, "下書き" )
            , ( Just WorkflowInstance.Pending, "申請待ち" )
            , ( Just WorkflowInstance.InProgress, "承認中" )
            , ( Just WorkflowInstance.Approved, "承認済み" )
            , ( Just WorkflowInstance.Rejected, "却下" )
            , ( Just WorkflowInstance.Cancelled, "キャンセル" )
            ]

        isSelected maybeStatus =
            currentFilter == maybeStatus
    in
    div [ class "status-filter" ]
        [ label [] [ text "ステータス: " ]
        , select [ onInput (statusFromFilterValue >> SetStatusFilter) ]
            (List.map
                (\( maybeStatus, label_ ) ->
                    option
                        [ value (statusToFilterValue maybeStatus)
                        , selected (isSelected maybeStatus)
                        ]
                        [ text label_ ]
                )
                statusOptions
            )
        ]


statusToFilterValue : Maybe Status -> String
statusToFilterValue maybeStatus =
    case maybeStatus of
        Nothing ->
            ""

        Just status ->
            WorkflowInstance.statusToString status


statusFromFilterValue : String -> Maybe Status
statusFromFilterValue str =
    if String.isEmpty str then
        Nothing

    else
        WorkflowInstance.statusFromString str


viewWorkflowTable : List WorkflowInstance -> Html Msg
viewWorkflowTable workflows =
    table [ class "workflow-table" ]
        [ thead []
            [ tr []
                [ th [] [ text "タイトル" ]
                , th [] [ text "ステータス" ]
                , th [] [ text "作成日" ]
                ]
            ]
        , tbody []
            (List.map viewWorkflowRow workflows)
        ]


viewWorkflowRow : WorkflowInstance -> Html Msg
viewWorkflowRow workflow =
    tr []
        [ td []
            [ a [ href (Route.toString (Route.WorkflowDetail workflow.id)) ]
                [ text workflow.title ]
            ]
        , td []
            [ span [ class (WorkflowInstance.statusToCssClass workflow.status) ]
                [ text (WorkflowInstance.statusToJapanese workflow.status) ]
            ]
        , td [] [ text (formatDate workflow.createdAt) ]
        ]


viewCount : Int -> Html Msg
viewCount count =
    div [ class "workflow-count" ]
        [ text ("全 " ++ String.fromInt count ++ " 件") ]


{-| ISO 8601 日時文字列から日付部分を抽出
-}
formatDate : String -> String
formatDate isoString =
    String.left 10 isoString
