module Util.Format exposing (formatFileSize)

{-| フォーマットユーティリティ

汎用的なフォーマット関数を提供する。

-}


{-| ファイルサイズを読みやすい形式にフォーマット

    formatFileSize 1024 --> "1 KB"

    formatFileSize 1048576 --> "1 MB"

    formatFileSize 512 --> "512 B"

-}
formatFileSize : Int -> String
formatFileSize bytes =
    if bytes >= 1048576 then
        String.fromFloat (toFloat (bytes * 10 // 1048576) / 10) ++ " MB"

    else if bytes >= 1024 then
        String.fromFloat (toFloat (bytes * 10 // 1024) / 10) ++ " KB"

    else
        String.fromInt bytes ++ " B"
