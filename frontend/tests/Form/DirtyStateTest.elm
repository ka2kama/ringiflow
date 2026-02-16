module Form.DirtyStateTest exposing (suite)

{-| Form.DirtyState モジュールのテスト

モデルの isDirty\_ 状態変化を検証する。
Cmd 値は Elm で比較できないため、モデル更新のみテストする。

-}

import Expect
import Form.DirtyState as DirtyState
import Test exposing (..)


suite : Test
suite =
    describe "Form.DirtyState"
        [ isDirtyTests
        , markDirtyTests
        , clearDirtyTests
        ]



-- isDirty


isDirtyTests : Test
isDirtyTests =
    describe "isDirty"
        [ test "isDirty_ が True のモデルは True を返す" <|
            \_ ->
                DirtyState.isDirty { isDirty_ = True }
                    |> Expect.equal True
        , test "isDirty_ が False のモデルは False を返す" <|
            \_ ->
                DirtyState.isDirty { isDirty_ = False }
                    |> Expect.equal False
        ]



-- markDirty


markDirtyTests : Test
markDirtyTests =
    describe "markDirty"
        [ test "isDirty_ が False → True に更新される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        DirtyState.markDirty { isDirty_ = False, name = "test" }
                in
                newModel.isDirty_
                    |> Expect.equal True
        , test "isDirty_ が False → 他のフィールドは保持される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        DirtyState.markDirty { isDirty_ = False, name = "test" }
                in
                newModel.name
                    |> Expect.equal "test"
        , test "isDirty_ が既に True → True のまま変わらない" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        DirtyState.markDirty { isDirty_ = True, name = "test" }
                in
                newModel.isDirty_
                    |> Expect.equal True
        ]



-- clearDirty


clearDirtyTests : Test
clearDirtyTests =
    describe "clearDirty"
        [ test "isDirty_ が True → False に更新される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        DirtyState.clearDirty { isDirty_ = True, name = "test" }
                in
                newModel.isDirty_
                    |> Expect.equal False
        , test "isDirty_ が True → 他のフィールドは保持される" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        DirtyState.clearDirty { isDirty_ = True, name = "test" }
                in
                newModel.name
                    |> Expect.equal "test"
        , test "isDirty_ が既に False → False のまま変わらない" <|
            \_ ->
                let
                    ( newModel, _ ) =
                        DirtyState.clearDirty { isDirty_ = False, name = "test" }
                in
                newModel.isDirty_
                    |> Expect.equal False
        ]
