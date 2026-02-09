module Data.DashboardTest exposing (suite)

{-| Data.Dashboard のデコーダテスト

ダッシュボード統計データの JSON デコーダが正しく動作することを検証する。

-}

import Data.Dashboard as Dashboard
import Expect
import Json.Decode as Decode
import Test exposing (..)


suite : Test
suite =
    describe "Data.Dashboard"
        [ decoderTests
        ]


decoderTests : Test
decoderTests =
    describe "decoder"
        [ test "全フィールドをデコード（data ラッパー含む）" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "pending_tasks": 3,
                                "my_workflows_in_progress": 5,
                                "completed_today": 2
                            }
                        }
                        """
                in
                Decode.decodeString Dashboard.decoder json
                    |> Expect.equal
                        (Ok
                            { pendingTasks = 3
                            , myWorkflowsInProgress = 5
                            , completedToday = 2
                            }
                        )
        , test "必須フィールド欠落でエラー" <|
            \_ ->
                let
                    json =
                        """
                        {
                            "data": {
                                "pending_tasks": 3
                            }
                        }
                        """
                in
                Decode.decodeString Dashboard.decoder json
                    |> Expect.err
        ]
