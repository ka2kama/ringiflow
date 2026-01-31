module Api.ErrorMessage exposing (toUserMessage)

{-| API エラーメッセージ変換

ApiError をエンティティ名に応じたユーザー向けメッセージに変換する。


## 使用例

    import Api exposing (ApiError)
    import Api.ErrorMessage as ErrorMessage

    ErrorMessage.toUserMessage { entityName = "ワークフロー" } error
    --> "ワークフローが見つかりません。"

-}

import Api exposing (ApiError(..))


{-| API エラーをユーザー向けメッセージに変換

エンティティ名をパラメータ化し、NotFound や Conflict のメッセージに
エンティティ名を埋め込む。

-}
toUserMessage : { entityName : String } -> ApiError -> String
toUserMessage { entityName } error =
    case error of
        Conflict problem ->
            "この" ++ entityName ++ "は既に更新されています。最新の状態を取得してください。（" ++ problem.detail ++ "）"

        Forbidden problem ->
            "この操作を実行する権限がありません。（" ++ problem.detail ++ "）"

        BadRequest problem ->
            problem.detail

        NotFound _ ->
            entityName ++ "が見つかりません。"

        Unauthorized ->
            "ログインが必要です。"

        ServerError _ ->
            "サーバーエラーが発生しました。"

        NetworkError ->
            "ネットワークエラーが発生しました。"

        Timeout ->
            "リクエストがタイムアウトしました。"

        DecodeError _ ->
            "データの処理中にエラーが発生しました。"
