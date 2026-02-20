module Data.WorkflowDefinition exposing
    ( ApprovalStepInfo
    , WorkflowDefinition
    , WorkflowDefinitionId
    , WorkflowDefinitionStatus(..)
    , approvalStepInfos
    , decoder
    , definitionStatus
    , detailDecoder
    , encodeCreateRequest
    , encodeVersionRequest
    , listDecoder
    , statusFromString
    , statusToBadge
    , statusToJapanese
    )

{-| ワークフロー定義のデータ型

バックエンドの `WorkflowDefinition` に対応する型とデコーダーを提供する。


## 用途

  - 申請フォームでユーザーが選択可能なワークフロー定義の表示
  - 選択されたワークフロー定義に基づく動的フォーム生成
  - ワークフロー定義管理画面でのステータス管理・CRUD 操作

-}

import Data.AdminUser exposing (BadgeConfig)
import Json.Decode as Decode exposing (Decoder)
import Json.Decode.Pipeline exposing (optional, required)
import Json.Encode as Encode



-- TYPES


{-| ワークフロー定義 ID

UUID 文字列をラップした型。型安全性のため String ではなく専用型を使用。

-}
type alias WorkflowDefinitionId =
    String


{-| ワークフロー定義

バックエンドから取得するワークフロー定義のデータ構造。
`definition` フィールドにフォーム定義（フィールド一覧など）が含まれる。

-}
type alias WorkflowDefinition =
    { id : WorkflowDefinitionId
    , name : String
    , description : Maybe String
    , version : Int
    , definition : Decode.Value -- 動的な JSON 構造
    , status : String
    , createdBy : String
    , createdAt : String
    , updatedAt : String
    }


{-| ワークフロー定義のステータス

バックエンドの `WorkflowDefinitionStatus` enum に対応。
Draft → Published → Archived のライフサイクルを表す。

-}
type WorkflowDefinitionStatus
    = Draft
    | Published
    | Archived


{-| 文字列からステータスに変換

バックエンドは小文字（"draft", "published", "archived"）で返す。
不明な値は Draft にフォールバックする（新規作成直後のデフォルト）。

-}
statusFromString : String -> WorkflowDefinitionStatus
statusFromString str =
    case str of
        "draft" ->
            Draft

        "published" ->
            Published

        "archived" ->
            Archived

        _ ->
            Draft


{-| ステータスを日本語に変換（UI 表示用）
-}
statusToJapanese : WorkflowDefinitionStatus -> String
statusToJapanese status =
    case status of
        Draft ->
            "下書き"

        Published ->
            "公開済み"

        Archived ->
            "アーカイブ済み"


{-| ステータスに応じた Badge 設定を返す

`Data.AdminUser.BadgeConfig` 型を再利用。

-}
statusToBadge : WorkflowDefinitionStatus -> BadgeConfig
statusToBadge status =
    case status of
        Draft ->
            { colorClass = "bg-secondary-100 text-secondary-600 border-secondary-200"
            , label = "下書き"
            }

        Published ->
            { colorClass = "bg-success-50 text-success-600 border-success-200"
            , label = "公開済み"
            }

        Archived ->
            { colorClass = "bg-secondary-100 text-secondary-500 border-secondary-200"
            , label = "アーカイブ済み"
            }


{-| WorkflowDefinition の status フィールド（String）を型に変換する
-}
definitionStatus : WorkflowDefinition -> WorkflowDefinitionStatus
definitionStatus def =
    statusFromString def.status



-- ENCODERS


{-| ワークフロー定義作成リクエストの JSON を生成

最小限のデフォルト definition（開始ステップのみ）を含む。
実質的な定義編集はデザイナー（#725/#726）で行う。

-}
encodeCreateRequest : { name : String, description : String } -> Encode.Value
encodeCreateRequest { name, description } =
    Encode.object
        [ ( "name", Encode.string name )
        , ( "description", Encode.string description )
        , ( "definition", defaultDefinition )
        ]


{-| バージョン指定リクエストの JSON を生成

公開・アーカイブ操作で楽観的ロック用の version を送信する。

-}
encodeVersionRequest : { version : Int } -> Encode.Value
encodeVersionRequest { version } =
    Encode.object
        [ ( "version", Encode.int version )
        ]


{-| 最小限のデフォルト定義（開始ステップのみ）
-}
defaultDefinition : Encode.Value
defaultDefinition =
    Encode.object
        [ ( "steps"
          , Encode.list identity
                [ Encode.object
                    [ ( "id", Encode.string "start" )
                    , ( "type", Encode.string "start" )
                    , ( "name", Encode.string "開始" )
                    ]
                ]
          )
        ]



-- DECODERS


{-| 単一のワークフロー定義をデコード
-}
decoder : Decoder WorkflowDefinition
decoder =
    Decode.succeed WorkflowDefinition
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> optional "description" (Decode.nullable Decode.string) Nothing
        |> required "version" Decode.int
        |> required "definition" Decode.value
        |> required "status" Decode.string
        |> required "created_by" Decode.string
        |> required "created_at" Decode.string
        |> required "updated_at" Decode.string


{-| 単一のワークフロー定義レスポンスをデコード

API レスポンスの `{ data: {...} }` 形式に対応。

-}
detailDecoder : Decoder WorkflowDefinition
detailDecoder =
    Decode.field "data" decoder


{-| ワークフロー定義一覧をデコード

API レスポンスの `{ data: [...] }` 形式に対応。

-}
listDecoder : Decoder (List WorkflowDefinition)
listDecoder =
    Decode.field "data" (Decode.list decoder)



-- HELPERS


{-| 承認ステップの情報（ID と名前のペア）
-}
type alias ApprovalStepInfo =
    { id : String
    , name : String
    }


{-| 定義 JSON から承認ステップの情報（ID と名前）一覧を抽出する

定義 JSON の `steps` 配列から `type == "approval"` のステップの ID と名前を
順序を保って返す。

-}
approvalStepInfos : WorkflowDefinition -> List ApprovalStepInfo
approvalStepInfos def =
    case Decode.decodeValue approvalStepInfosDecoder def.definition of
        Ok infos ->
            infos

        Err _ ->
            []


approvalStepInfosDecoder : Decoder (List ApprovalStepInfo)
approvalStepInfosDecoder =
    Decode.field "steps" (Decode.list stepInfoDecoder)
        |> Decode.map (List.filterMap identity)


stepInfoDecoder : Decoder (Maybe ApprovalStepInfo)
stepInfoDecoder =
    Decode.map3
        (\id name stepType ->
            if stepType == "approval" then
                Just { id = id, name = name }

            else
                Nothing
        )
        (Decode.field "id" Decode.string)
        (Decode.field "name" Decode.string)
        (Decode.field "type" Decode.string)
