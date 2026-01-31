module RemoteData exposing
    ( RemoteData(..)
    , fromResult
    , isLoading
    , map
    , toMaybe
    , withDefault
    )

{-| リモートデータの状態を表す汎用型

API レスポンスのライフサイクルを型で表現する。
各ページモジュールで共通的に使用する。

詳細: [RemoteData パターン](../../docs/06_ナレッジベース/elm/RemoteData.md)

-}


{-| リモートデータの状態

  - `NotAsked` — まだリクエストしていない
  - `Loading` — リクエスト中
  - `Failure e` — 失敗（エラー情報を保持）
  - `Success a` — 成功（データを保持）

-}
type RemoteData e a
    = NotAsked
    | Loading
    | Failure e
    | Success a


{-| 成功値を変換する
-}
map : (a -> b) -> RemoteData e a -> RemoteData e b
map f remoteData =
    case remoteData of
        NotAsked ->
            NotAsked

        Loading ->
            Loading

        Failure e ->
            Failure e

        Success a ->
            Success (f a)


{-| デフォルト値を提供する

Success 以外の場合にデフォルト値を返す。

-}
withDefault : a -> RemoteData e a -> a
withDefault default remoteData =
    case remoteData of
        Success a ->
            a

        _ ->
            default


{-| Maybe に変換する

Success は Just、それ以外は Nothing。

-}
toMaybe : RemoteData e a -> Maybe a
toMaybe remoteData =
    case remoteData of
        Success a ->
            Just a

        _ ->
            Nothing


{-| Result から RemoteData に変換する

Ok → Success、Err → Failure。

-}
fromResult : Result e a -> RemoteData e a
fromResult result =
    case result of
        Ok a ->
            Success a

        Err e ->
            Failure e


{-| Loading かどうかを判定する
-}
isLoading : RemoteData e a -> Bool
isLoading remoteData =
    case remoteData of
        Loading ->
            True

        _ ->
            False
