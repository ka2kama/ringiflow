module Component.Icons exposing
    ( auditLog
    , dashboard
    , menu
    , roles
    , tasks
    , users
    , workflows
    )

{-| サイドバー用 SVG アイコン

Main.elm のファイルサイズ削減のために抽出。

-}

import Html exposing (Html)
import Svg exposing (svg)
import Svg.Attributes as SvgAttr


{-| ダッシュボード（グリッド）
-}
dashboard : Html msg
dashboard =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.rect [ SvgAttr.x "3", SvgAttr.y "3", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        , Svg.rect [ SvgAttr.x "14", SvgAttr.y "3", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        , Svg.rect [ SvgAttr.x "3", SvgAttr.y "14", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        , Svg.rect [ SvgAttr.x "14", SvgAttr.y "14", SvgAttr.width "7", SvgAttr.height "7", SvgAttr.rx "1" ] []
        ]


{-| 申請一覧（ドキュメント）
-}
workflows : Html msg
workflows =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8Z" ] []
        , Svg.path [ SvgAttr.d "M14 2v6h6" ] []
        , Svg.line [ SvgAttr.x1 "16", SvgAttr.y1 "13", SvgAttr.x2 "8", SvgAttr.y2 "13" ] []
        , Svg.line [ SvgAttr.x1 "16", SvgAttr.y1 "17", SvgAttr.x2 "8", SvgAttr.y2 "17" ] []
        ]


{-| タスク一覧（チェックリスト）
-}
tasks : Html msg
tasks =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M9 11l3 3L22 4" ] []
        , Svg.path [ SvgAttr.d "M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" ] []
        ]


{-| ユーザー管理（People）
-}
users : Html msg
users =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" ] []
        , Svg.circle [ SvgAttr.cx "9", SvgAttr.cy "7", SvgAttr.r "4" ] []
        , Svg.path [ SvgAttr.d "M23 21v-2a4 4 0 0 0-3-3.87" ] []
        , Svg.path [ SvgAttr.d "M16 3.13a4 4 0 0 1 0 7.75" ] []
        ]


{-| ロール管理（Shield）
-}
roles : Html msg
roles =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" ] []
        ]


{-| 監査ログ（ClipboardList）
-}
auditLog : Html msg
auditLog =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-5 w-5"
        ]
        [ Svg.path [ SvgAttr.d "M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" ] []
        , Svg.rect [ SvgAttr.x "8", SvgAttr.y "2", SvgAttr.width "8", SvgAttr.height "4", SvgAttr.rx "1" ] []
        , Svg.line [ SvgAttr.x1 "8", SvgAttr.y1 "10", SvgAttr.x2 "16", SvgAttr.y2 "10" ] []
        , Svg.line [ SvgAttr.x1 "8", SvgAttr.y1 "14", SvgAttr.x2 "16", SvgAttr.y2 "14" ] []
        , Svg.line [ SvgAttr.x1 "8", SvgAttr.y1 "18", SvgAttr.x2 "12", SvgAttr.y2 "18" ] []
        ]


{-| ハンバーガーメニュー
-}
menu : Html msg
menu =
    svg
        [ SvgAttr.viewBox "0 0 24 24"
        , SvgAttr.fill "none"
        , SvgAttr.stroke "currentColor"
        , SvgAttr.strokeWidth "2"
        , SvgAttr.class "h-6 w-6"
        ]
        [ Svg.line [ SvgAttr.x1 "3", SvgAttr.y1 "6", SvgAttr.x2 "21", SvgAttr.y2 "6" ] []
        , Svg.line [ SvgAttr.x1 "3", SvgAttr.y1 "12", SvgAttr.x2 "21", SvgAttr.y2 "12" ] []
        , Svg.line [ SvgAttr.x1 "3", SvgAttr.y1 "18", SvgAttr.x2 "21", SvgAttr.y2 "18" ] []
        ]
