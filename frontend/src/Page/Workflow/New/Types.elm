module Page.Workflow.New.Types exposing
    ( EditingState
    , FormState(..)
    , LoadedState
    , Model
    , Msg(..)
    , PageState(..)
    , SaveMessage(..)
    , initEditing
    )

{-| New ページの共有型定義

型定義を独立モジュールに配置し、New.elm とサブモジュール間の
循環依存を防止する。

-}

import Api exposing (ApiError)
import Api.Workflow as WorkflowApi
import Component.ApproverSelector as ApproverSelector
import Component.FileUpload as FileUpload
import Data.FormField exposing (FieldType(..), FormField)
import Data.UserItem exposing (UserItem)
import Data.WorkflowDefinition as WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance exposing (WorkflowInstance)
import Dict exposing (Dict)
import Form.DynamicForm as DynamicForm
import RemoteData exposing (RemoteData)
import Shared exposing (Shared)



-- MODEL


{-| ページの状態
-}
type alias Model =
    { shared : Shared
    , users : RemoteData ApiError (List UserItem)
    , state : PageState
    }


{-| ページの状態遷移

    Loading → Loaded（定義取得成功）
    Loading → Failed（定義取得失敗）

-}
type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState


{-| 定義ロード完了後の状態
-}
type alias LoadedState =
    { definitions : List WorkflowDefinition
    , formState : FormState
    }


{-| フォームの状態遷移

    SelectingDefinition → Editing（定義選択）

-}
type FormState
    = SelectingDefinition
    | Editing EditingState


{-| フォーム編集中の状態

定義が選択済みであることが型で保証される。

-}
type alias EditingState =
    { selectedDefinition : WorkflowDefinition
    , title : String
    , formValues : Dict String String
    , validationErrors : Dict String String
    , approvers : Dict String ApproverSelector.State
    , fileUploads : Dict String FileUpload.Model
    , savedWorkflow : Maybe WorkflowInstance
    , saveMessage : Maybe SaveMessage
    , submitting : Bool
    , isDirty_ : Bool
    }


{-| 保存結果メッセージ
-}
type SaveMessage
    = SaveSuccess String
    | SaveError String


{-| 編集状態の初期化

定義選択時に新しい EditingState を構築する。
承認ステップ情報から ApproverSelector の初期状態を生成する。
ファイルフィールドから FileUpload の初期状態を生成する。

-}
initEditing : WorkflowDefinition -> EditingState
initEditing definition =
    { selectedDefinition = definition
    , title = ""
    , formValues = Dict.empty
    , validationErrors = Dict.empty
    , approvers =
        WorkflowDefinition.approvalStepInfos definition
            |> List.map (\info -> ( info.id, ApproverSelector.init ))
            |> Dict.fromList
    , fileUploads = initFileUploads definition
    , savedWorkflow = Nothing
    , saveMessage = Nothing
    , submitting = False
    , isDirty_ = False
    }


{-| 定義のファイルフィールドから FileUpload モデルを初期化

各 file フィールドに対して FileUpload.init を呼び出す。
workflowInstanceId は未保存のため Nothing。

-}
initFileUploads : WorkflowDefinition -> Dict String FileUpload.Model
initFileUploads definition =
    case DynamicForm.extractFormFields definition.definition of
        Ok fields ->
            fields
                |> List.filterMap
                    (\field ->
                        case field.fieldType of
                            File config ->
                                Just ( field.id, FileUpload.init config Nothing )

                            _ ->
                                Nothing
                    )
                |> Dict.fromList

        Err _ ->
            Dict.empty



-- UPDATE


{-| メッセージ
-}
type Msg
    = -- 初期化
      GotDefinitions (Result ApiError (List WorkflowDefinition))
    | GotUsers (Result ApiError (List UserItem))
      -- ワークフロー定義選択
    | SelectDefinition String
      -- フォーム入力
    | UpdateTitle String
    | UpdateField String String
      -- 承認者選択（第1引数: ステップ ID）
    | UpdateApproverSearch String String
    | SelectApprover String UserItem
    | ClearApprover String
    | ApproverKeyDown String String
    | CloseApproverDropdown String
      -- ファイルアップロード（第1引数: フィールド ID）
    | FileUploadMsg String FileUpload.Msg
      -- 保存・申請
    | SaveDraft
    | GotSaveResult (Result ApiError WorkflowInstance)
    | Submit
    | GotSaveAndSubmitResult (List WorkflowApi.StepApproverRequest) (Result ApiError WorkflowInstance)
    | GotSubmitResult (Result ApiError WorkflowInstance)
      -- メッセージクリア
    | ClearMessage
