module Page.Workflow.New exposing (init, isDirty, subscriptions, update, updateShared, view)

{-| 新規申請フォームページ

ワークフロー定義を選択し、フォームを入力して申請するページ。
型定義は New.Types に、Update ロジックは New.Update に、
フォーム表示は New.FormView に分離している。

このモジュールはオーケストレーターとして、初期化・状態遷移・
メッセージルーティング・ページレイアウトを担当する。


## 画面フロー

1.  ワークフロー定義一覧を取得・表示
2.  ユーザーが定義を選択
3.  動的フォームを生成・表示
4.  フォーム入力 → バリデーション
5.  下書き保存 or 申請


## 設計

詳細: [申請フォーム UI 設計](../../../../docs/40_詳細設計書/10_ワークフロー申請フォームUI設計.md)

-}

import Api exposing (ApiError)
import Api.ErrorMessage as ErrorMessage
import Api.User as UserApi
import Api.WorkflowDefinition as WorkflowDefinitionApi
import Component.ErrorState as ErrorState
import Component.FileUpload as FileUpload
import Component.LoadingSpinner as LoadingSpinner
import Data.UserItem exposing (UserItem)
import Dict
import Html exposing (..)
import Html.Attributes exposing (..)
import Page.Workflow.New.FormView as FormView
import Page.Workflow.New.Types exposing (..)
import Page.Workflow.New.Update as Update
import RemoteData exposing (RemoteData)
import Shared exposing (Shared)



-- INIT


{-| 初期化

ページ表示時にワークフロー定義一覧とユーザー一覧を並行取得する。

-}
init : Shared -> ( Model, Cmd Msg )
init shared =
    ( { shared = shared
      , users = RemoteData.Loading
      , state = Loading
      }
    , Cmd.batch
        [ fetchDefinitions shared
        , fetchUsers shared
        ]
    )


{-| ワークフロー定義一覧を取得
-}
fetchDefinitions : Shared -> Cmd Msg
fetchDefinitions shared =
    WorkflowDefinitionApi.listDefinitions
        { config = Shared.toRequestConfig shared
        , toMsg = GotDefinitions
        }


{-| テナント内ユーザー一覧を取得
-}
fetchUsers : Shared -> Cmd Msg
fetchUsers shared =
    UserApi.listUsers
        { config = Shared.toRequestConfig shared
        , toMsg = GotUsers
        }


{-| 共有状態を更新

Main.elm から新しい共有状態（CSRF トークン取得後など）を受け取る。

-}
updateShared : Shared -> Model -> Model
updateShared shared model =
    { model | shared = shared }


{-| フォームに未保存の変更があるかを返す
-}
isDirty : Model -> Bool
isDirty model =
    case model.state of
        Loaded loaded ->
            case loaded.formState of
                Editing editing ->
                    editing.isDirty_

                SelectingDefinition ->
                    False

        _ ->
            False



-- UPDATE


{-| 状態更新（外側）

GotDefinitions で Loading → Loaded/Failed の状態遷移を処理。
GotUsers は state に依存せず users を更新。
それ以外は Loaded 状態のときのみ Update.updateLoaded に委譲。

-}
update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        GotDefinitions result ->
            case result of
                Ok definitions ->
                    ( { model
                        | state =
                            Loaded
                                { definitions = definitions
                                , formState = SelectingDefinition
                                }
                      }
                    , Cmd.none
                    )

                Err error ->
                    ( { model | state = Failed error }
                    , Cmd.none
                    )

        GotUsers result ->
            case result of
                Ok users ->
                    ( { model | users = RemoteData.Success users }
                    , Cmd.none
                    )

                Err error ->
                    ( { model | users = RemoteData.Failure error }
                    , Cmd.none
                    )

        _ ->
            case model.state of
                Loaded loaded ->
                    let
                        ( newLoaded, cmd ) =
                            Update.updateLoaded msg model.shared model.users loaded
                    in
                    ( { model | state = Loaded newLoaded }, cmd )

                _ ->
                    ( model, Cmd.none )



-- SUBSCRIPTIONS


{-| FileUpload の進捗購読

Editing 状態のファイルアップロードコンポーネントの subscriptions を集約する。

-}
subscriptions : Model -> Sub Msg
subscriptions model =
    case model.state of
        Loaded loaded ->
            case loaded.formState of
                Editing editing ->
                    editing.fileUploads
                        |> Dict.map
                            (\fieldId fileUploadModel ->
                                FileUpload.subscriptions fileUploadModel
                                    |> Sub.map (FileUploadMsg fieldId)
                            )
                        |> Dict.values
                        |> Sub.batch

                SelectingDefinition ->
                    Sub.none

        _ ->
            Sub.none



-- VIEW


{-| ページの描画
-}
view : Model -> Html Msg
view model =
    div []
        [ h1 [ class "mb-6 text-2xl font-bold text-secondary-900" ] [ text "新規申請" ]
        , viewBody model
        ]


{-| メインコンテンツ

PageState のパターンマッチで Loading/Failed/Loaded を分岐。
Failed では ErrorState.viewSimple + ErrorMessage.toUserMessage で
ApiError に応じた具体的なエラーメッセージを表示する。

-}
viewBody : Model -> Html Msg
viewBody model =
    case model.state of
        Loading ->
            LoadingSpinner.view

        Failed error ->
            ErrorState.viewSimple
                (ErrorMessage.toUserMessage { entityName = "ワークフロー定義" } error)

        Loaded loaded ->
            viewLoaded model.users loaded


{-| Loaded 状態の描画

FormState のパターンマッチで SelectingDefinition/Editing を分岐。

-}
viewLoaded : RemoteData ApiError (List UserItem) -> LoadedState -> Html Msg
viewLoaded users loaded =
    case loaded.formState of
        SelectingDefinition ->
            FormView.viewDefinitionSelector loaded.definitions Nothing

        Editing editing ->
            div []
                [ FormView.viewSaveMessage editing.saveMessage
                , FormView.viewDefinitionSelector loaded.definitions (Just editing.selectedDefinition.id)
                , FormView.viewFormInputs users editing
                ]
