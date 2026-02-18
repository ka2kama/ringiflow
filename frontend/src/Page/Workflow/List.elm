module Page.Workflow.List exposing
    ( Model
    , Msg
    , applyFilter
    , filterWorkflows
    , init
    , isCompletedToday
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
import Browser.Navigation as Nav
import Component.Badge as Badge
import Component.Button as Button
import Component.LoadingSpinner as LoadingSpinner
import Data.WorkflowInstance as WorkflowInstance exposing (Status, WorkflowInstance)
import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (onClick, onInput)
import Iso8601
import RemoteData exposing (RemoteData(..))
import Route exposing (WorkflowFilter)
import Shared exposing (Shared)
import Task
import Time
import Util.DateFormat as DateFormat



-- MODEL


{-| ページの状態
-}
type alias Model =
    { -- 共有状態（API 呼び出しに必要）
      shared : Shared

    -- ナビゲーション（URL 同期用）
    , key : Nav.Key

    -- API データ
    , workflows : RemoteData ApiError (List WorkflowInstance)

    -- フィルタ状態
    , statusFilter : Maybe Status
    , completedToday : Bool

    -- 現在時刻（completedToday フィルタの日付比較用）
    , now : Maybe Time.Posix
    }


{-| 初期化

フィルタ付き URL からの遷移に対応。
`completedToday=True` の場合は `Time.now` を取得して日付比較に使う。

-}
init : Shared -> Nav.Key -> WorkflowFilter -> ( Model, Cmd Msg )
init shared key filter =
    let
        timeCmd =
            if filter.completedToday then
                Task.perform GotCurrentTime Time.now

            else
                Cmd.none
    in
    ( { shared = shared
      , key = key
      , workflows = Loading
      , statusFilter = filter.status
      , completedToday = filter.completedToday
      , now = Nothing
      }
    , Cmd.batch
        [ WorkflowApi.listMyWorkflows
            { config = Shared.toRequestConfig shared
            , toMsg = GotWorkflows
            }
        , timeCmd
        ]
    )


{-| 共有状態を更新

Main.elm から新しい共有状態（CSRF トークン取得後など）を受け取る。

-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }


{-| URL 変更時にフィルタ状態のみを更新（データ再取得しない）

Main.elm の同一ページ判定から呼ばれる。
`completedToday` が `True` に変わった場合、フレッシュな `Time.now` を取得する。

-}
applyFilter : WorkflowFilter -> Model -> ( Model, Cmd Msg )
applyFilter filter model =
    let
        newModel =
            { model | statusFilter = filter.status, completedToday = filter.completedToday }

        cmd =
            if filter.completedToday then
                Task.perform GotCurrentTime Time.now

            else
                Cmd.none
    in
    ( newModel, cmd )



-- UPDATE


{-| メッセージ
-}
type Msg
    = GotWorkflows (Result ApiError (List WorkflowInstance))
    | SetStatusFilter (Maybe Status)
    | ClearCompletedToday
    | GotCurrentTime Time.Posix
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
            ( model
            , Nav.replaceUrl model.key
                (Route.toString (Route.Workflows { status = maybeStatus, completedToday = False }))
            )

        ClearCompletedToday ->
            ( model
            , Nav.replaceUrl model.key
                (Route.toString (Route.Workflows { status = model.statusFilter, completedToday = False }))
            )

        GotCurrentTime now ->
            ( { model | now = Just now }
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
            let
                zone =
                    Shared.zone model.shared

                filter =
                    { status = model.statusFilter, completedToday = model.completedToday }
            in
            viewWorkflowList zone model.now filter workflows


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


viewWorkflowList : Time.Zone -> Maybe Time.Posix -> WorkflowFilter -> List WorkflowInstance -> Html Msg
viewWorkflowList zone maybeNow filter workflows =
    let
        filteredWorkflows =
            filterWorkflows zone maybeNow filter workflows
    in
    div []
        [ viewFilterBar filter
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
                [ div [ class "overflow-x-auto rounded-lg border border-secondary-200" ] [ viewWorkflowTable zone filteredWorkflows ]
                , viewCount (List.length filteredWorkflows)
                ]
        ]


{-| フィルタバー

ステータスドロップダウンと completedToday バッジを表示する。

-}
viewFilterBar : WorkflowFilter -> Html Msg
viewFilterBar filter =
    div [ class "mb-4 flex flex-wrap items-center gap-2" ]
        [ viewStatusDropdown filter.status
        , if filter.completedToday then
            viewCompletedTodayBadge

          else
            text ""
        ]


{-| ステータスフィルタのドロップダウン
-}
viewStatusDropdown : Maybe Status -> Html Msg
viewStatusDropdown currentFilter =
    let
        statusOptions =
            [ ( Nothing, "すべて" )
            , ( Just WorkflowInstance.Draft, "下書き" )
            , ( Just WorkflowInstance.Pending, "申請待ち" )
            , ( Just WorkflowInstance.InProgress, "承認中" )
            , ( Just WorkflowInstance.Approved, "承認済み" )
            , ( Just WorkflowInstance.Rejected, "却下" )
            , ( Just WorkflowInstance.ChangesRequested, "差し戻し" )
            , ( Just WorkflowInstance.Cancelled, "キャンセル" )
            ]

        isSelected maybeStatus =
            currentFilter == maybeStatus
    in
    span [ class "flex items-center gap-2" ]
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


{-| 「本日完了のみ」バッジ

ダッシュボードの「本日完了」カードから遷移した場合に表示。
「×」ボタンでフィルタを解除できる。

-}
viewCompletedTodayBadge : Html Msg
viewCompletedTodayBadge =
    span [ class "inline-flex items-center gap-1 rounded-full bg-success-50 px-3 py-1 text-sm text-success-700" ]
        [ text "本日完了のみ"
        , button
            [ onClick ClearCompletedToday
            , class "ml-1 hover:text-success-900"
            , attribute "aria-label" "フィルタ解除"
            ]
            [ text "×" ]
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
    table [ class "w-full" ]
        [ thead [ class "bg-secondary-50" ]
            [ tr []
                [ th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "ID" ]
                , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "タイトル" ]
                , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "ステータス" ]
                , th [ class "px-4 py-3 text-left text-xs font-medium uppercase tracking-wider text-secondary-500" ] [ text "作成日" ]
                ]
            ]
        , tbody [ class "divide-y divide-secondary-200 bg-white" ]
            (List.map (viewWorkflowRow zone) workflows)
        ]


viewWorkflowRow : Time.Zone -> WorkflowInstance -> Html Msg
viewWorkflowRow zone workflow =
    tr [ class "hover:bg-secondary-50 transition-colors" ]
        [ td [ class "px-4 py-3 text-sm text-secondary-500" ] [ text workflow.displayId ]
        , td [ class "px-4 py-3" ]
            [ a [ href (Route.toString (Route.WorkflowDetail workflow.displayNumber)), class "text-primary-600 hover:text-primary-700 hover:underline" ]
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



-- FILTER LOGIC


{-| フィルタ条件に基づいてワークフローを絞り込む

`completedToday` が `True` の場合、`status` フィルタは無視される（プリセット優先）。
`now` が `Nothing`（まだ取得されていない）場合はフィルタを適用せず全件を返す。

-}
filterWorkflows : Time.Zone -> Maybe Time.Posix -> WorkflowFilter -> List WorkflowInstance -> List WorkflowInstance
filterWorkflows zone maybeNow filter workflows =
    if filter.completedToday then
        case maybeNow of
            Just now ->
                List.filter (isCompletedToday zone now) workflows

            Nothing ->
                workflows

    else
        case filter.status of
            Nothing ->
                workflows

            Just status ->
                List.filter (\w -> w.status == status) workflows


{-| ワークフローが「本日完了」に該当するかを判定

条件: status が Approved かつ updatedAt が今日の日付。

-}
isCompletedToday : Time.Zone -> Time.Posix -> WorkflowInstance -> Bool
isCompletedToday zone now workflow =
    (workflow.status == WorkflowInstance.Approved)
        && (case Iso8601.toTime workflow.updatedAt of
                Ok updatedPosix ->
                    sameDate zone now updatedPosix

                Err _ ->
                    False
           )


{-| 2つの Time.Posix が同じ日付かを判定（タイムゾーン考慮）
-}
sameDate : Time.Zone -> Time.Posix -> Time.Posix -> Bool
sameDate zone a b =
    (Time.toYear zone a == Time.toYear zone b)
        && (Time.toMonth zone a == Time.toMonth zone b)
        && (Time.toDay zone a == Time.toDay zone b)
