module Data.WorkflowDefinitionTest exposing (suite)

{-| Data.WorkflowDefinition モジュールのテスト
-}

import Data.WorkflowDefinition as WorkflowDefinition
import Expect
import Json.Decode as Decode
import Json.Encode as Encode
import Test exposing (..)


suite : Test
suite =
    describe "Data.WorkflowDefinition"
        [ decoderTests
        , listDecoderTests
        , approvalStepInfosTests
        , encodeUpdateRequestTests
        , validationResultDecoderTests
        ]



-- decoder


decoderTests : Test
decoderTests =
    describe "decoder"
        [ test "全フィールドをデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001",
                            "name": "経費精算申請",
                            "description": "経費の精算を申請します",
                            "version": 1,
                            "definition": {"form": {"fields": []}},
                            "status": "active",
                            "created_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Result.map
                        (\d ->
                            { id = d.id
                            , name = d.name
                            , description = d.description
                            , version = d.version
                            }
                        )
                    |> Expect.equal
                        (Ok
                            { id = "def-001"
                            , name = "経費精算申請"
                            , description = Just "経費の精算を申請します"
                            , version = 1
                            }
                        )
        , test "description が null の場合は Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001",
                            "name": "休暇申請",
                            "description": null,
                            "version": 1,
                            "definition": {},
                            "status": "active",
                            "created_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Result.map .description
                    |> Expect.equal (Ok Nothing)
        , test "description がない場合も Nothing" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001",
                            "name": "休暇申請",
                            "version": 1,
                            "definition": {},
                            "status": "active",
                            "created_by": "user-001",
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-01T00:00:00Z"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Result.map .description
                    |> Expect.equal (Ok Nothing)
        , test "必須フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "id": "def-001"
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.decoder json
                    |> Expect.err
        ]



-- listDecoder


listDecoderTests : Test
listDecoderTests =
    describe "listDecoder"
        [ test "data フィールドから一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "def-001",
                                    "name": "経費精算",
                                    "version": 1,
                                    "definition": {},
                                    "status": "active",
                                    "created_by": "user-001",
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                },
                                {
                                    "id": "def-002",
                                    "name": "休暇申請",
                                    "version": 1,
                                    "definition": {},
                                    "status": "active",
                                    "created_by": "user-001",
                                    "created_at": "2026-01-01T00:00:00Z",
                                    "updated_at": "2026-01-01T00:00:00Z"
                                }
                            ]
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.listDecoder json
                    |> Result.map List.length
                    |> Expect.equal (Ok 2)
        , test "空の一覧をデコード" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": []
                        }
                        """
                in
                Decode.decodeString WorkflowDefinition.listDecoder json
                    |> Expect.equal (Ok [])
        , test "data フィールドがない場合はエラー" <|
            \_ ->
                let
                    json =
                        """
                        []
                        """
                in
                Decode.decodeString WorkflowDefinition.listDecoder json
                    |> Expect.err
        ]



-- approvalStepInfos


approvalStepInfosTests : Test
approvalStepInfosTests =
    describe "approvalStepInfos"
        [ test "複数の承認ステップから id と name を抽出" <|
            \_ ->
                let
                    definition =
                        makeDefinition
                            (Encode.object
                                [ ( "steps"
                                  , Encode.list identity
                                        [ Encode.object
                                            [ ( "id", Encode.string "step-1" )
                                            , ( "name", Encode.string "部長承認" )
                                            , ( "type", Encode.string "approval" )
                                            ]
                                        , Encode.object
                                            [ ( "id", Encode.string "step-2" )
                                            , ( "name", Encode.string "経理承認" )
                                            , ( "type", Encode.string "approval" )
                                            ]
                                        ]
                                  )
                                ]
                            )
                in
                WorkflowDefinition.approvalStepInfos definition
                    |> List.map (\info -> ( info.id, info.name ))
                    |> Expect.equal
                        [ ( "step-1", "部長承認" )
                        , ( "step-2", "経理承認" )
                        ]
        , test "承認ステップが空の場合は空リスト" <|
            \_ ->
                let
                    definition =
                        makeDefinition
                            (Encode.object
                                [ ( "steps", Encode.list identity [] ) ]
                            )
                in
                WorkflowDefinition.approvalStepInfos definition
                    |> Expect.equal []
        , test "approval 以外のステップは除外" <|
            \_ ->
                let
                    definition =
                        makeDefinition
                            (Encode.object
                                [ ( "steps"
                                  , Encode.list identity
                                        [ Encode.object
                                            [ ( "id", Encode.string "step-1" )
                                            , ( "name", Encode.string "部長承認" )
                                            , ( "type", Encode.string "approval" )
                                            ]
                                        , Encode.object
                                            [ ( "id", Encode.string "step-notify" )
                                            , ( "name", Encode.string "通知" )
                                            , ( "type", Encode.string "notification" )
                                            ]
                                        ]
                                  )
                                ]
                            )
                in
                WorkflowDefinition.approvalStepInfos definition
                    |> List.map .id
                    |> Expect.equal [ "step-1" ]
        ]



-- encodeUpdateRequest


encodeUpdateRequestTests : Test
encodeUpdateRequestTests =
    describe "encodeUpdateRequest"
        [ test "name/description/definition/version を正しくエンコードする" <|
            \_ ->
                let
                    definition =
                        Encode.object [ ( "steps", Encode.list identity [] ) ]

                    encoded =
                        WorkflowDefinition.encodeUpdateRequest
                            { name = "経費精算フロー"
                            , description = "経費精算の申請フロー"
                            , definition = definition
                            , version = 3
                            }

                    decodedName =
                        Decode.decodeValue (Decode.field "name" Decode.string) encoded

                    decodedDescription =
                        Decode.decodeValue (Decode.field "description" Decode.string) encoded

                    decodedVersion =
                        Decode.decodeValue (Decode.field "version" Decode.int) encoded

                    decodedDefinition =
                        Decode.decodeValue (Decode.field "definition" Decode.value) encoded
                in
                Expect.all
                    [ \_ -> decodedName |> Expect.equal (Ok "経費精算フロー")
                    , \_ -> decodedDescription |> Expect.equal (Ok "経費精算の申請フロー")
                    , \_ -> decodedVersion |> Expect.equal (Ok 3)
                    , \_ -> decodedDefinition |> Result.map (\_ -> True) |> Expect.equal (Ok True)
                    ]
                    ()
        ]



-- validationResultDecoder


validationResultDecoderTests : Test
validationResultDecoderTests =
    describe "validationResultDecoder"
        [ test "valid: true, errors: [] をデコードする" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "valid": true,
                                "errors": []
                            }
                        }
                        """
                in
                case Decode.decodeString WorkflowDefinition.validationResultDecoder json of
                    Ok result ->
                        Expect.all
                            [ \r -> r.valid |> Expect.equal True
                            , \r -> r.errors |> Expect.equal []
                            ]
                            result

                    Err err ->
                        Expect.fail ("Expected Ok, got Err: " ++ Decode.errorToString err)
        , test "valid: false, errors: [{code, message, step_id}] をデコードする" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "valid": false,
                                "errors": [
                                    {
                                        "code": "missing_start_step",
                                        "message": "開始ステップが必要です"
                                    },
                                    {
                                        "code": "orphaned_step",
                                        "message": "ステップ 'approval_1' が接続されていません",
                                        "step_id": "approval_1"
                                    }
                                ]
                            }
                        }
                        """
                in
                case Decode.decodeString WorkflowDefinition.validationResultDecoder json of
                    Ok result ->
                        Expect.all
                            [ \r -> r.valid |> Expect.equal False
                            , \r -> List.length r.errors |> Expect.equal 2
                            , \r ->
                                List.head r.errors
                                    |> Maybe.map (\e -> ( e.code, e.stepId ))
                                    |> Expect.equal (Just ( "missing_start_step", Nothing ))
                            , \r ->
                                r.errors
                                    |> List.drop 1
                                    |> List.head
                                    |> Maybe.map (\e -> ( e.code, e.stepId ))
                                    |> Expect.equal (Just ( "orphaned_step", Just "approval_1" ))
                            ]
                            result

                    Err err ->
                        Expect.fail ("Expected Ok, got Err: " ++ Decode.errorToString err)
        ]



-- Helpers


{-| テスト用の WorkflowDefinition を構築するヘルパー
-}
makeDefinition : Encode.Value -> WorkflowDefinition.WorkflowDefinition
makeDefinition def =
    { id = "def-001"
    , name = "テスト定義"
    , description = Nothing
    , version = 1
    , definition = def
    , status = "active"
    , createdBy = "user-001"
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }
