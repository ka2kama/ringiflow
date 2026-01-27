module Data.WorkflowDefinitionTest exposing (suite)

{-| Data.WorkflowDefinition モジュールのテスト
-}

import Data.WorkflowDefinition as WorkflowDefinition
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.WorkflowDefinition"
        [ decoderTests
        , listDecoderTests
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
