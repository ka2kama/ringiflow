module Page.Workflow.List exposing
    ( Model
    , Msg
    , init
    , update
    , updateShared
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

import Api exposing (ApiError)
import Api.Workflow as WorkflowApi
import Component.Badge as Badge
import Component.Button as Button
import Component.LoadingSpinner as LoadingSpinner
import Data.WorkflowInstance as WorkflowInstance exposing (Status, WorkflowInstance)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onInput)
import RemoteData exposing (RemoteData(..))
import Route
import Shared exposing (Shared)
import Time
import Util.DateFormat as DateFormat



-- MODEL


{-| ページの状態
-}
type alias Model =
    { -- 共有状態（API 呼び出しに必要）
      shared : Shared

    -- API データ
    , workflows : RemoteData ApiError (List WorkflowInstance)

    -- フィルタ状態
    , statusFilter : Maybe Status
    }


{-| 初期化
-}
init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , workflows = Loading
      , statusFilter = Nothing
      }
    , WorkflowApi.listMyWorkflows
        { config = Shared.toRequestConfig shared
        , toMsg = GotWorkflows
        }
    )


{-| 共有状態を更新

Main.elm から新しい共有状態（CSRF トークン取得後など）を受け取る。

-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }



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

                Err err ->
                    ( { model | workflows = Failure err }
                    , Cmd.none
                    )

        SetStatusFilter maybeStatus ->
            ( { model | statusFilter = maybeStatus }
            , Cmd.none
            )

        Refresh ->
            ( { model | workflows = Loading }
            , WorkflowApi.listMyWorkflows
                { config = Shared.toRequestConfig model.shared
                , toMsg = GotWorkflows
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
    div [ class "flex items-center justify-between mb-6" ]
        [ h1 [ class "text-2xl font-bold text-secondary-900" ] [ text "申請一覧" ]
        , Button.link
            { variant = Button.Primary
            , href = Route.toString Route.WorkflowNew
            }
            [ text "+ 新規申請" ]
        ]


viewContent : Model -> Html Msg
viewContent model =
    case model.workflows of
        NotAsked ->
            text ""

        Loading ->
            LoadingSpinner.view

        Failure _ ->
            viewError

        Success workflows ->
            viewWorkflowList (Shared.zone model.shared) model.statusFilter workflows


viewError : Html Msg
viewError =
    div [ class "rounded-lg bg-error-50 p-4 text-error-700" ]
        [ p [] [ text "データの取得に失敗しました。" ]
        , Button.view
            { variant = Button.Outline
            , disabled = False
            , onClick = Refresh
            }
            [ text "再読み込み" ]
        ]


viewWorkflowList : Time.Zone -> Maybe Status -> List WorkflowInstance -> Html Msg
viewWorkflowList zone statusFilter workflows =
    let
        filteredWorkflows =
            case statusFilter of
                Nothing ->
                    workflows

                Just status ->
                    List.filter (\w -> w.status == status) workflows
    in
    div []
        [ viewStatusFilter statusFilter
        , if List.isEmpty filteredWorkflows then
            div [ class "py-12 text-center" ]
                [ p [ class "text-secondary-500" ] [ text "申請がありません" ]
                , div [ class "mt-4" ]
                    [ Button.link
                        { variant = Button.Primary
                        , href = Route.toString Route.WorkflowNew
                        }
                        [ text "新規申請を作成" ]
                    ]
                ]

          else
            div []
                [ div [ class "overflow-x-auto" ] [ viewWorkflowTable zone filteredWorkflows ]
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
    div [ class "mb-4 flex items-center gap-2" ]
        [ label [ for "status-filter" ] [ text "ステータス: " ]
        , select [ id "status-filter", onInput (statusFromFilterValue >> SetStatusFilter), class "rounded border border-secondary-100 bg-white px-3 py-1.5 text-sm" ]
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


viewWorkflowTable : Time.Zone -> List WorkflowInstance -> Html Msg
viewWorkflowTable zone workflows =
    table [ class "w-full border-collapse" ]
        [ thead [ class "border-b border-secondary-100" ]
            [ tr []
                [ th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "ID" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "タイトル" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "ステータス" ]
                , th [ class "px-4 py-3 text-left text-sm font-medium text-secondary-500" ] [ text "作成日" ]
                ]
            ]
        , tbody []
            (List.map (viewWorkflowRow zone) workflows)
        ]


viewWorkflowRow : Time.Zone -> WorkflowInstance -> Html Msg
viewWorkflowRow zone workflow =
    tr [ class "border-b border-secondary-100" ]
        [ td [ class "px-4 py-3 text-sm text-secondary-500" ] [ text workflow.displayId ]
        , td [ class "px-4 py-3" ]
            [ a [ href (Route.toString (Route.WorkflowDetail workflow.id)), class "text-primary-600 hover:text-primary-700 hover:underline" ]
                [ text workflow.title ]
            ]
        , td [ class "px-4 py-3" ]
            [ Badge.view
                { colorClass = WorkflowInstance.statusToCssClass workflow.status
                , label = WorkflowInstance.statusToJapanese workflow.status
                }
            ]
        , td [ class "px-4 py-3" ] [ text (DateFormat.formatDate zone workflow.createdAt) ]
        ]


viewCount : Int -> Html Msg
viewCount count =
    div [ class "mt-4 text-sm text-secondary-500" ]
        [ text ("全 " ++ String.fromInt count ++ " 件") ]
