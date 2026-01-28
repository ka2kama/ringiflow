module Session exposing
    ( Session
    , User
    , getUserId
    , init
    , toRequestConfig
    , withCsrfToken
    , withUser
    )

{-| セッション状態モジュール

全ページで共有される認証・テナント情報を管理する。


## 用途

  - API リクエスト時のヘッダー情報（tenantId, csrfToken）の提供
  - ログイン中のユーザー情報の保持
  - 認証状態に基づく UI 制御


## 設計方針

Session は「グローバル状態」として Main.elm で保持し、
各ページモジュールに渡す。ページモジュールは Session を
直接変更せず、Main.elm 経由で更新する。

-}

import Api.Http exposing (RequestConfig)



-- TYPES


{-| ユーザー情報

ログイン中のユーザーを表す。
GET /auth/me のレスポンスから構築される。

-}
type alias User =
    { id : String
    , email : String
    , name : String
    , roles : List String
    }


{-| セッション状態

アプリケーション全体で共有される状態。
未認証時は user が Nothing となる。

-}
type alias Session =
    { user : Maybe User
    , tenantId : String
    , csrfToken : Maybe String
    , apiBaseUrl : String
    }



-- CONSTRUCTORS


{-| セッションを初期化

開発環境では仮のテナント ID を使用。
本番環境では GET /auth/me でテナント情報を取得する。

-}
init : { apiBaseUrl : String } -> Session
init { apiBaseUrl } =
    { user = Nothing
    , tenantId = "00000000-0000-0000-0000-000000000001" -- 開発用テナント
    , csrfToken = Nothing
    , apiBaseUrl = apiBaseUrl
    }


{-| CSRF トークンを設定
-}
withCsrfToken : String -> Session -> Session
withCsrfToken token session =
    { session | csrfToken = Just token }


{-| ユーザー情報を設定
-}
withUser : User -> Session -> Session
withUser user session =
    { session
        | user = Just user
        , tenantId = extractTenantId user
    }



-- HELPERS


{-| ユーザーからテナント ID を抽出

MVP ではユーザー情報にテナント ID が含まれる想定。
将来的には User 型に tenantId フィールドを追加する。

-}
extractTenantId : User -> String
extractTenantId _ =
    -- TODO: User 型に tenantId を追加後、ここを修正
    "00000000-0000-0000-0000-000000000001"


{-| API リクエスト設定に変換

Api.Http モジュールの関数で使用する RequestConfig を生成。

    session
        |> Session.toRequestConfig
        |> Api.Workflow.listMyWorkflows

-}
toRequestConfig : Session -> RequestConfig
toRequestConfig session =
    { baseUrl = session.apiBaseUrl
    , tenantId = Just session.tenantId
    , csrfToken = session.csrfToken
    }


{-| ログイン中のユーザー ID を取得

未ログイン時は Nothing を返す。

-}
getUserId : Session -> Maybe String
getUserId session =
    session.user |> Maybe.map .id
