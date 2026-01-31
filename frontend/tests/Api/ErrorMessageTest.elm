module Api.ErrorMessageTest exposing (suite)

{-| Api.ErrorMessage モジュールのテスト

ApiError をユーザー向けメッセージに変換する関数の正確性を検証する。

-}

import Api exposing (ApiError(..), ProblemDetails)
import Api.ErrorMessage as ErrorMessage
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Api.ErrorMessage"
        [ toUserMessageTests
        ]


{-| テスト用の ProblemDetails ヘルパー
-}
testProblem : String -> ProblemDetails
testProblem detail =
    { errorType = "about:blank"
    , title = "エラー"
    , status = 400
    , detail = detail
    }



-- toUserMessage


toUserMessageTests : Test
toUserMessageTests =
    describe "toUserMessage"
        [ test "Conflict はエンティティ名と detail を含むメッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    (Conflict (testProblem "バージョン競合"))
                    |> Expect.equal "このワークフローは既に更新されています。最新の状態を取得してください。（バージョン競合）"
        , test "Forbidden は権限エラーメッセージと detail を返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "タスク" }
                    (Forbidden (testProblem "管理者権限が必要"))
                    |> Expect.equal "この操作を実行する権限がありません。（管理者権限が必要）"
        , test "BadRequest は detail をそのまま返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    (BadRequest (testProblem "入力値が不正です"))
                    |> Expect.equal "入力値が不正です"
        , test "NotFound はエンティティ名を含むメッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    (NotFound (testProblem "見つかりません"))
                    |> Expect.equal "ワークフローが見つかりません。"
        , test "NotFound はエンティティ名「タスク」でも正しく動作する" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "タスク" }
                    (NotFound (testProblem "見つかりません"))
                    |> Expect.equal "タスクが見つかりません。"
        , test "Unauthorized はログイン要求メッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    Unauthorized
                    |> Expect.equal "ログインが必要です。"
        , test "ServerError はサーバーエラーメッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    (ServerError (testProblem "内部エラー"))
                    |> Expect.equal "サーバーエラーが発生しました。"
        , test "NetworkError はネットワークエラーメッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    NetworkError
                    |> Expect.equal "ネットワークエラーが発生しました。"
        , test "Timeout はタイムアウトメッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    Timeout
                    |> Expect.equal "リクエストがタイムアウトしました。"
        , test "DecodeError はデータ処理エラーメッセージを返す" <|
            \_ ->
                ErrorMessage.toUserMessage { entityName = "ワークフロー" }
                    (DecodeError "Unexpected token")
                    |> Expect.equal "データの処理中にエラーが発生しました。"
        ]
