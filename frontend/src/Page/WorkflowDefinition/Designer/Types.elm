module Page.WorkflowDefinition.Designer.Types exposing
    ( CanvasState
    , Model
    , Msg(..)
    , PageState(..)
    , canvasElementId
    )

{-| Designer ページの共有型定義

型定義を独立モジュールに配置し、Designer.elm とサブモジュール間の
循環依存を防止する。Designer.elm が本モジュールの型を re-export するため、
外部モジュール（Main.elm、テスト）からの import パスは変更不要。

-}

import Api exposing (ApiError)
import Data.DesignerCanvas exposing (Bounds, DraggingState, ReconnectEnd, StepNode, StepType, Transition)
import Data.WorkflowDefinition exposing (ValidationResult, WorkflowDefinition)
import Dict exposing (Dict)
import Json.Encode as Encode
import Shared exposing (Shared)



-- CONSTANTS


{-| キャンバス SVG 要素の HTML id
-}
canvasElementId : String
canvasElementId =
    "designer-canvas"



-- MODEL


{-| 外側 Model: 共通フィールド + 状態 ADT
-}
type alias Model =
    { shared : Shared
    , definitionId : String
    , state : PageState
    }


{-| ページの状態を表す ADT

Loading 中はキャンバス関連フィールドが存在しないため、
キャンバス操作が型レベルで不可能になる。

-}
type PageState
    = Loading
    | Failed ApiError
    | Loaded CanvasState


{-| Loaded 時のみ存在するキャンバス状態
-}
type alias CanvasState =
    { steps : Dict String StepNode
    , transitions : List Transition
    , selectedStepId : Maybe String
    , selectedTransitionIndex : Maybe Int
    , dragging : Maybe DraggingState
    , canvasBounds : Maybe Bounds
    , nextStepNumber : Int
    , propertyName : String
    , propertyEndStatus : String
    , name : String
    , description : String
    , version : Int
    , isSaving : Bool
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , isDirty_ : Bool
    , validationResult : Maybe ValidationResult
    , isValidating : Bool
    , isPublishing : Bool
    , pendingPublish : Bool
    }



-- UPDATE


type Msg
    = PaletteMouseDown StepType
    | CanvasMouseMove Float Float
    | CanvasMouseUp
    | StepClicked String
    | CanvasBackgroundClicked
    | StepMouseDown String Float Float
    | ConnectionPortMouseDown String Float Float
    | TransitionEndpointMouseDown Int ReconnectEnd Float Float
    | TransitionClicked Int
    | UpdatePropertyName String
    | UpdatePropertyEndStatus String
    | UpdateDefinitionName String
    | SaveClicked
    | GotDefinition (Result ApiError WorkflowDefinition)
    | GotSaveResult (Result ApiError WorkflowDefinition)
    | ValidateClicked
    | GotValidationResult (Result ApiError ValidationResult)
    | PublishClicked
    | ConfirmPublish
    | CancelPublish
    | GotPublishResult (Result ApiError WorkflowDefinition)
    | DismissMessage
    | DeleteSelectedStep
    | DeleteSelectedTransition
    | KeyDown String
    | GotCanvasBounds Encode.Value
