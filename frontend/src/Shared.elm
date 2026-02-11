module Shared exposing
    ( Shared
    , User
    , getUserId
    , init
    , isAdmin
    , toRequestConfig
    , withCsrfToken
    , withUser
    , zone
    )

{-| 共有状態モジュール

全ページで共有される認証・テナント情報を管理する。


## 用途

  - API リクエスト時のヘッダー情報（tenantId, csrfToken）の提供
  - ログイン中のユーザー情報の保持
  - 認証状態に基づく UI 制御


## 設計方針

Shared は「グローバル状態」として Main.elm で保持し、
各ページモジュールに渡す。ページモジュールは Shared を
直接変更せず、Main.elm 経由で更新する。

-}

import Api exposing (RequestConfig)
import Time



-- TYPES


{-| ユーザー情報

ログイン中のユーザーを表す。
GET /auth/me のレスポンスから構築される。

-}
type alias User =
    { id : String
    , email : String
    , name : String
    , tenantId : String
    , roles : List String
    }


{-| 共有状態

アプリケーション全体で共有される状態。
未認証時は user が Nothing となる。

-}
type alias Shared =
    { user : Maybe User
    , tenantId : String
    , csrfToken : Maybe String
    , apiBaseUrl : String
    , timeZone : Time.Zone
    }



-- CONSTRUCTORS


{-| 共有状態を初期化

開発環境では仮のテナント ID を使用。
本番環境では GET /auth/me でテナント情報を取得する。

timezoneOffsetMinutes: JavaScript の getTimezoneOffset() を反転した値。
JST なら 540（= +9 \* 60）。

-}
init : { apiBaseUrl : String, timezoneOffsetMinutes : Int } -> Shared
init { apiBaseUrl, timezoneOffsetMinutes } =
    { user = Nothing
    , tenantId = "00000000-0000-0000-0000-000000000001" -- 開発用テナント
    , csrfToken = Nothing
    , apiBaseUrl = apiBaseUrl
    , timeZone = Time.customZone timezoneOffsetMinutes []
    }


{-| CSRF トークンを設定
-}
withCsrfToken : String -> Shared -> Shared
withCsrfToken token shared =
    { shared | csrfToken = Just token }


{-| ユーザー情報を設定

ログイン後、User から取得した tenantId で Shared.tenantId を更新する。

-}
withUser : User -> Shared -> Shared
withUser user shared =
    { shared
        | user = Just user
        , tenantId = user.tenantId
    }



-- HELPERS


{-| API リクエスト設定に変換

Api モジュールの関数で使用する RequestConfig を生成。

    shared
        |> Shared.toRequestConfig
        |> Api.Workflow.listMyWorkflows

-}
toRequestConfig : Shared -> RequestConfig
toRequestConfig shared =
    { baseUrl = shared.apiBaseUrl
    , tenantId = Just shared.tenantId
    , csrfToken = shared.csrfToken
    }


{-| 現在のユーザーが管理者かどうかを判定

roles に "admin" が含まれているかで判定する。
未ログイン時は False を返す。

-}
isAdmin : Shared -> Bool
isAdmin shared =
    case shared.user of
        Just user ->
            List.member "admin" user.roles

        Nothing ->
            False


{-| ログイン中のユーザー ID を取得

未ログイン時は Nothing を返す。

-}
getUserId : Shared -> Maybe String
getUserId shared =
    shared.user |> Maybe.map .id


{-| タイムゾーンを取得

日付・時刻のフォーマットに使用する。

-}
zone : Shared -> Time.Zone
zone shared =
    shared.timeZone
