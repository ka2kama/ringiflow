module Page.Workflow.Detail.Types exposing
    ( EditState(..)
    , EditingState
    , LoadedState
    , Model
    , Msg(..)
    , PageState(..)
    , PendingAction(..)
    , initLoaded
    )

{-| Detail ページの共有型定義

型定義を独立モジュールに配置し、Detail.elm とサブモジュール間の
循環依存を防止する。

-}

import Api exposing (ApiError)
import Component.ApproverSelector as ApproverSelector
import Data.Document exposing (Document, DownloadUrlResponse)
import Data.UserItem exposing (UserItem)
import Data.WorkflowComment exposing (WorkflowComment)
import Data.WorkflowDefinition exposing (WorkflowDefinition)
import Data.WorkflowInstance exposing (WorkflowInstance, WorkflowStep)
import Dict exposing (Dict)
import RemoteData exposing (RemoteData(..))
import Shared exposing (Shared)



-- MODEL


{-| 確認待ちの操作

承認/却下ボタンクリック後、確認ダイアログで最終確認するまで保持する。

-}
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
    | ConfirmRequestChanges WorkflowStep


{-| ページの状態（ADR-054 パターン A: 外側に共通フィールド）
-}
type alias Model =
    { shared : Shared
    , workflowDisplayNumber : Int
    , state : PageState
    }


{-| ページの状態遷移
-}
type PageState
    = Loading
    | Failed ApiError
    | Loaded LoadedState


{-| Loaded 時のみ存在するフィールド

workflow 取得完了後の状態でのみ有効なフィールドを集約する。
definition/comments は Loaded 後も非同期ロード中のため RemoteData を維持。

編集状態は EditState ADT で Viewing/Editing を分離し、
編集中のみ有効なフィールドが Editing バリアント内に存在する（ADR-054）。

-}
type alias LoadedState =
    { workflow : WorkflowInstance
    , definition : RemoteData ApiError WorkflowDefinition

    -- 承認/却下/差し戻し
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String

    -- コメントスレッド
    , comments : RemoteData ApiError (List WorkflowComment)
    , newCommentBody : String
    , isPostingComment : Bool

    -- 添付ファイル
    , attachments : RemoteData ApiError (List Document)

    -- ユーザー一覧（承認者選択で使用）
    , users : RemoteData ApiError (List UserItem)

    -- 編集状態
    , editState : EditState
    }


{-| 再提出の編集状態

Viewing → Editing（StartEditing）
Editing → Viewing（CancelEditing, GotResubmitResult Ok）

-}
type EditState
    = Viewing
    | Editing EditingState


{-| 編集中の状態

編集中のみ有効なフィールドを集約する。
Workflow/New.elm の EditingState と同じパターン。

-}
type alias EditingState =
    { editFormData : Dict String String
    , editApprovers : Dict String ApproverSelector.State
    , resubmitValidationErrors : Dict String String
    , isResubmitting : Bool
    }


{-| LoadedState の初期値を構築

GotWorkflow Ok 受信時、workflow から LoadedState を生成する。
definition は後続の GotDefinition で、comments は後続の GotComments で更新される。

-}
initLoaded : WorkflowInstance -> LoadedState
initLoaded workflow =
    { workflow = workflow
    , definition = RemoteData.Loading
    , comment = ""
    , isSubmitting = False
    , pendingAction = Nothing
    , errorMessage = Nothing
    , successMessage = Nothing
    , comments = RemoteData.Loading
    , newCommentBody = ""
    , isPostingComment = False
    , attachments = RemoteData.Loading
    , users = NotAsked
    , editState = Viewing
    }



-- UPDATE


{-| メッセージ
-}
type Msg
    = GotWorkflow (Result ApiError WorkflowInstance)
    | GotDefinition (Result ApiError WorkflowDefinition)
    | Refresh
    | UpdateComment String
    | ClickApprove WorkflowStep
    | ClickReject WorkflowStep
    | ClickRequestChanges WorkflowStep
    | ConfirmAction
    | CancelAction
    | GotApproveResult (Result ApiError WorkflowInstance)
    | GotRejectResult (Result ApiError WorkflowInstance)
    | GotRequestChangesResult (Result ApiError WorkflowInstance)
    | GotComments (Result ApiError (List WorkflowComment))
    | UpdateNewComment String
    | SubmitComment
    | GotPostCommentResult (Result ApiError WorkflowComment)
      -- 再提出
    | StartEditing
    | CancelEditing
    | UpdateEditFormField String String
    | EditApproverSearchChanged String String
    | EditApproverSelected String UserItem
    | EditApproverCleared String
    | EditApproverKeyDown String String
    | EditApproverDropdownClosed String
    | SubmitResubmit
    | GotResubmitResult (Result ApiError WorkflowInstance)
    | GotUsers (Result ApiError (List UserItem))
      -- 添付ファイル
    | GotAttachments (Result ApiError (List Document))
    | DownloadFile String
    | GotDownloadUrl (Result ApiError DownloadUrlResponse)
    | DismissMessage
