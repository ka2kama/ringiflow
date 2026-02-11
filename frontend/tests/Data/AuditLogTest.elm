module Data.AuditLogTest exposing (suite)

{-| Data.AuditLog モジュールのテスト
-}

import Data.AuditLog as AuditLog
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.AuditLog"
        [ auditLogListDecoderTests
        , actionToJapaneseTests
        , resultToJapaneseTests
        ]



-- AuditLogList decoder


auditLogListDecoderTests : Test
auditLogListDecoderTests =
    describe "auditLogListDecoder"
        [ test "next_cursor が string のケース" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "log-1",
                                    "actor_id": "user-1",
                                    "actor_name": "山田太郎",
                                    "action": "user.create",
                                    "result": "success",
                                    "resource_type": "user",
                                    "resource_id": "user-2",
                                    "detail": {"email": "new@example.com"},
                                    "source_ip": "192.168.1.1",
                                    "created_at": "2026-02-01T10:00:00Z"
                                }
                            ],
                            "next_cursor": "cursor-abc123"
                        }
                        """
                in
                Decode.decodeString AuditLog.auditLogListDecoder json
                    |> Result.map .nextCursor
                    |> Expect.equal (Ok (Just "cursor-abc123"))
        , test "next_cursor が null のケース（最終ページ）" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [],
                            "next_cursor": null
                        }
                        """
                in
                Decode.decodeString AuditLog.auditLogListDecoder json
                    |> Result.map .nextCursor
                    |> Expect.equal (Ok Nothing)
        , test "detail が null のケース" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": [
                                {
                                    "id": "log-1",
                                    "actor_id": "user-1",
                                    "actor_name": "山田太郎",
                                    "action": "user.create",
                                    "result": "success",
                                    "resource_type": "user",
                                    "resource_id": "user-2",
                                    "detail": null,
                                    "source_ip": null,
                                    "created_at": "2026-02-01T10:00:00Z"
                                }
                            ],
                            "next_cursor": null
                        }
                        """
                in
                Decode.decodeString AuditLog.auditLogListDecoder json
                    |> Result.map (\list -> List.head list.data |> Maybe.map .detail)
                    |> Expect.equal (Ok (Just Nothing))
        ]



-- actionToJapanese


actionToJapaneseTests : Test
actionToJapaneseTests =
    describe "actionToJapanese"
        [ test "user.create → ユーザー作成" <|
            \_ ->
                AuditLog.actionToJapanese "user.create"
                    |> Expect.equal "ユーザー作成"
        , test "user.update → ユーザー更新" <|
            \_ ->
                AuditLog.actionToJapanese "user.update"
                    |> Expect.equal "ユーザー更新"
        , test "user.deactivate → ユーザー無効化" <|
            \_ ->
                AuditLog.actionToJapanese "user.deactivate"
                    |> Expect.equal "ユーザー無効化"
        , test "user.activate → ユーザー有効化" <|
            \_ ->
                AuditLog.actionToJapanese "user.activate"
                    |> Expect.equal "ユーザー有効化"
        , test "role.create → ロール作成" <|
            \_ ->
                AuditLog.actionToJapanese "role.create"
                    |> Expect.equal "ロール作成"
        , test "role.update → ロール更新" <|
            \_ ->
                AuditLog.actionToJapanese "role.update"
                    |> Expect.equal "ロール更新"
        , test "role.delete → ロール削除" <|
            \_ ->
                AuditLog.actionToJapanese "role.delete"
                    |> Expect.equal "ロール削除"
        , test "不明なアクション → そのまま返す" <|
            \_ ->
                AuditLog.actionToJapanese "unknown.action"
                    |> Expect.equal "unknown.action"
        ]



-- resultToJapanese


resultToJapaneseTests : Test
resultToJapaneseTests =
    describe "resultToJapanese"
        [ test "success → 成功" <|
            \_ ->
                AuditLog.resultToJapanese "success"
                    |> Expect.equal "成功"
        , test "failure → 失敗" <|
            \_ ->
                AuditLog.resultToJapanese "failure"
                    |> Expect.equal "失敗"
        , test "不明な結果 → そのまま返す" <|
            \_ ->
                AuditLog.resultToJapanese "unknown"
                    |> Expect.equal "unknown"
        ]
