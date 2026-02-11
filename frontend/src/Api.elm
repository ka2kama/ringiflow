module Api exposing
    ( ApiError(..)
    , ProblemDetails
    , RequestConfig
    , delete
    , deleteNoContent
    , get
    , patch
    , post
    , problemDetailsDecoder
    , put
    )

{-| HTTP リクエスト用ヘルパー

BFF への API リクエストを型安全に行うためのモジュール。
CSRF トークン、X-Tenant-ID ヘッダーの付与と RFC 9457 エラーハンドリングを提供。


## 使用例

    import Api

    fetchDefinitions : Api.RequestConfig -> (Result Api.ApiError (List WorkflowDefinition) -> msg) -> Cmd msg
    fetchDefinitions config toMsg =
        Api.get
            { config = config
            , url = "/api/v1/workflow-definitions"
            , decoder = WorkflowDefinition.listDecoder
            , toMsg = toMsg
            }


## CSRF トークンについて

状態変更リクエスト（POST/PUT/DELETE）では CSRF トークンが必要。
トークンは `GET /auth/csrf` で取得し、Model に保存しておく。

-}

import Http
import Json.Decode as Decode exposing (Decoder)



-- ERROR TYPES


{-| API エラーの型

HTTP エラーを適切に分類し、UI での表示やリカバリー処理を可能にする。

-}
type ApiError
    = BadRequest ProblemDetails
    | Unauthorized
    | Forbidden ProblemDetails
    | NotFound ProblemDetails
    | Conflict ProblemDetails
    | ServerError ProblemDetails
    | NetworkError
    | Timeout
    | DecodeError String


{-| RFC 9457 Problem Details

バックエンドが返すエラーレスポンスの標準フォーマット。
ユーザーフレンドリーなエラーメッセージ表示に使用。

-}
type alias ProblemDetails =
    { errorType : String
    , title : String
    , status : Int
    , detail : String
    }


{-| ProblemDetails のデコーダー
-}
problemDetailsDecoder : Decoder ProblemDetails
problemDetailsDecoder =
    Decode.map4 ProblemDetails
        (Decode.field "type" Decode.string)
        (Decode.field "title" Decode.string)
        (Decode.field "status" Decode.int)
        (Decode.field "detail" Decode.string)



-- REQUEST CONFIG


{-| API リクエストの設定

リクエストごとに必要なヘッダー情報を保持する。
Model から構築して各 API 関数に渡す。

-}
type alias RequestConfig =
    { baseUrl : String
    , tenantId : Maybe String
    , csrfToken : Maybe String
    }



-- REQUEST HELPERS


{-| GET リクエスト

CSRF トークンは不要だが、X-Tenant-ID ヘッダーは付与する。

-}
get :
    { config : RequestConfig
    , url : String
    , decoder : Decoder a
    , toMsg : Result ApiError a -> msg
    }
    -> Cmd msg
get { config, url, decoder, toMsg } =
    Http.request
        { method = "GET"
        , headers = buildHeaders config False
        , url = config.baseUrl ++ url
        , body = Http.emptyBody
        , expect = expectJson toMsg decoder
        , timeout = Just 30000
        , tracker = Nothing
        }


{-| POST リクエスト

CSRF トークンと X-Tenant-ID ヘッダーを付与する。

-}
post :
    { config : RequestConfig
    , url : String
    , body : Http.Body
    , decoder : Decoder a
    , toMsg : Result ApiError a -> msg
    }
    -> Cmd msg
post { config, url, body, decoder, toMsg } =
    Http.request
        { method = "POST"
        , headers = buildHeaders config True
        , url = config.baseUrl ++ url
        , body = body
        , expect = expectJson toMsg decoder
        , timeout = Just 30000
        , tracker = Nothing
        }


{-| PUT リクエスト

CSRF トークンと X-Tenant-ID ヘッダーを付与する。

-}
put :
    { config : RequestConfig
    , url : String
    , body : Http.Body
    , decoder : Decoder a
    , toMsg : Result ApiError a -> msg
    }
    -> Cmd msg
put { config, url, body, decoder, toMsg } =
    Http.request
        { method = "PUT"
        , headers = buildHeaders config True
        , url = config.baseUrl ++ url
        , body = body
        , expect = expectJson toMsg decoder
        , timeout = Just 30000
        , tracker = Nothing
        }


{-| PATCH リクエスト

CSRF トークンと X-Tenant-ID ヘッダーを付与する。

-}
patch :
    { config : RequestConfig
    , url : String
    , body : Http.Body
    , decoder : Decoder a
    , toMsg : Result ApiError a -> msg
    }
    -> Cmd msg
patch { config, url, body, decoder, toMsg } =
    Http.request
        { method = "PATCH"
        , headers = buildHeaders config True
        , url = config.baseUrl ++ url
        , body = body
        , expect = expectJson toMsg decoder
        , timeout = Just 30000
        , tracker = Nothing
        }


{-| DELETE リクエスト

CSRF トークンと X-Tenant-ID ヘッダーを付与する。

-}
delete :
    { config : RequestConfig
    , url : String
    , decoder : Decoder a
    , toMsg : Result ApiError a -> msg
    }
    -> Cmd msg
delete { config, url, decoder, toMsg } =
    Http.request
        { method = "DELETE"
        , headers = buildHeaders config True
        , url = config.baseUrl ++ url
        , body = Http.emptyBody
        , expect = expectJson toMsg decoder
        , timeout = Just 30000
        , tracker = Nothing
        }


{-| DELETE リクエスト（204 No Content 用）

レスポンスボディなしの DELETE に対応。
ロール削除など 204 を返すエンドポイント用。

-}
deleteNoContent :
    { config : RequestConfig
    , url : String
    , toMsg : Result ApiError () -> msg
    }
    -> Cmd msg
deleteNoContent { config, url, toMsg } =
    Http.request
        { method = "DELETE"
        , headers = buildHeaders config True
        , url = config.baseUrl ++ url
        , body = Http.emptyBody
        , expect = expectNoContent toMsg
        , timeout = Just 30000
        , tracker = Nothing
        }



-- INTERNAL HELPERS


{-| リクエストヘッダーを構築

X-Tenant-ID は常に付与、CSRF トークンは状態変更リクエストのみ。

-}
buildHeaders : RequestConfig -> Bool -> List Http.Header
buildHeaders config includeCsrf =
    let
        tenantHeader =
            case config.tenantId of
                Just id ->
                    [ Http.header "X-Tenant-ID" id ]

                Nothing ->
                    []

        csrfHeader =
            if includeCsrf then
                case config.csrfToken of
                    Just token ->
                        [ Http.header "X-CSRF-Token" token ]

                    Nothing ->
                        []

            else
                []
    in
    tenantHeader ++ csrfHeader


{-| JSON レスポンスを期待するヘルパー

HTTP エラーを ApiError に変換し、RFC 9457 レスポンスをデコードする。

-}
expectJson : (Result ApiError a -> msg) -> Decoder a -> Http.Expect msg
expectJson toMsg decoder =
    Http.expectStringResponse toMsg (handleResponse decoder)


{-| 204 No Content を期待するヘルパー

レスポンスボディを無視し、2xx を Ok () に変換する。

-}
expectNoContent : (Result ApiError () -> msg) -> Http.Expect msg
expectNoContent toMsg =
    Http.expectStringResponse toMsg handleNoContentResponse


{-| No Content レスポンスを処理
-}
handleNoContentResponse : Http.Response String -> Result ApiError ()
handleNoContentResponse response =
    case response of
        Http.BadUrl_ _ ->
            Err NetworkError

        Http.Timeout_ ->
            Err Timeout

        Http.NetworkError_ ->
            Err NetworkError

        Http.BadStatus_ metadata body ->
            Err (handleErrorStatus metadata.statusCode body)

        Http.GoodStatus_ _ _ ->
            Ok ()


{-| HTTP レスポンスを処理

ステータスコードに応じて適切な ApiError に変換する。

-}
handleResponse : Decoder a -> Http.Response String -> Result ApiError a
handleResponse decoder response =
    case response of
        Http.BadUrl_ _ ->
            Err NetworkError

        Http.Timeout_ ->
            Err Timeout

        Http.NetworkError_ ->
            Err NetworkError

        Http.BadStatus_ metadata body ->
            Err (handleErrorStatus metadata.statusCode body)

        Http.GoodStatus_ _ body ->
            case Decode.decodeString decoder body of
                Ok value ->
                    Ok value

                Err err ->
                    Err (DecodeError (Decode.errorToString err))


{-| エラーステータスを ApiError に変換

RFC 9457 ProblemDetails をデコードし、適切なエラー型に分類する。

-}
handleErrorStatus : Int -> String -> ApiError
handleErrorStatus statusCode body =
    let
        maybeProblem =
            Decode.decodeString problemDetailsDecoder body
                |> Result.toMaybe

        defaultProblem =
            { errorType = "about:blank"
            , title = "エラー"
            , status = statusCode
            , detail = body
            }

        problem =
            Maybe.withDefault defaultProblem maybeProblem
    in
    case statusCode of
        400 ->
            BadRequest problem

        401 ->
            Unauthorized

        403 ->
            Forbidden problem

        404 ->
            NotFound problem

        409 ->
            Conflict problem

        _ ->
            if statusCode >= 500 then
                ServerError problem

            else
                BadRequest problem
