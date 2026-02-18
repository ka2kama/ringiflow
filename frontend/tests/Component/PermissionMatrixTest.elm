module Component.PermissionMatrixTest exposing (suite)

{-| Component.PermissionMatrix のテスト

view のアクセシビリティ属性（aria-label）を検証する。

-}

import Component.PermissionMatrix as PermissionMatrix
import Html.Attributes
import Set
import Test exposing (..)
import Test.Html.Query as Query
import Test.Html.Selector as Selector


type TestMsg
    = NoOp


defaultConfig :
    { selectedPermissions : Set.Set String
    , onToggle : String -> TestMsg
    , onToggleAll : String -> TestMsg
    , disabled : Bool
    }
defaultConfig =
    { selectedPermissions = Set.empty
    , onToggle = always NoOp
    , onToggleAll = always NoOp
    , disabled = False
    }


suite : Test
suite =
    describe "Component.PermissionMatrix"
        [ viewTests
        ]



-- view


viewTests : Test
viewTests =
    describe "view"
        [ test "「すべて」チェックボックスに aria-label が存在する" <|
            \_ ->
                PermissionMatrix.view defaultConfig
                    |> Query.fromHtml
                    |> Query.has
                        [ Selector.attribute (Html.Attributes.attribute "aria-label" "ワークフロー すべて") ]
        , test "個別チェックボックスに aria-label が存在する" <|
            \_ ->
                PermissionMatrix.view defaultConfig
                    |> Query.fromHtml
                    |> Query.has
                        [ Selector.attribute (Html.Attributes.attribute "aria-label" "ワークフロー 閲覧") ]
        ]
