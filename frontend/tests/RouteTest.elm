module RouteTest exposing (suite)

{-| Route モジュールのテスト

URL パースと文字列変換の正確性を検証する。

-}

import Expect
import Route exposing (Route(..))
import Test exposing (..)
import Url


suite : Test
suite =
    describe "Route"
        [ fromUrlTests
        , toStringTests
        ]



-- fromUrl


fromUrlTests : Test
fromUrlTests =
    describe "fromUrl"
        [ test "/ → Home" <|
            \_ ->
                parseUrl "/"
                    |> Expect.equal Home
        , test "/workflows/new → WorkflowNew" <|
            \_ ->
                parseUrl "/workflows/new"
                    |> Expect.equal WorkflowNew
        , test "/workflows → Workflows" <|
            \_ ->
                parseUrl "/workflows"
                    |> Expect.equal Workflows
        , test "/workflows/{display_number} → WorkflowDetail display_number" <|
            \_ ->
                parseUrl "/workflows/42"
                    |> Expect.equal (WorkflowDetail 42)
        , test "/workflows/{non-integer} → NotFound" <|
            \_ ->
                parseUrl "/workflows/abc-123-def"
                    |> Expect.equal NotFound
        , test "/unknown → NotFound" <|
            \_ ->
                parseUrl "/unknown/path"
                    |> Expect.equal NotFound
        ]



-- toString


toStringTests : Test
toStringTests =
    describe "toString"
        [ test "Home → /" <|
            \_ ->
                Route.toString Home
                    |> Expect.equal "/"
        , test "WorkflowNew → /workflows/new" <|
            \_ ->
                Route.toString WorkflowNew
                    |> Expect.equal "/workflows/new"
        , test "Workflows → /workflows" <|
            \_ ->
                Route.toString Workflows
                    |> Expect.equal "/workflows"
        , test "WorkflowDetail display_number → /workflows/{display_number}" <|
            \_ ->
                Route.toString (WorkflowDetail 42)
                    |> Expect.equal "/workflows/42"
        , test "NotFound → /not-found" <|
            \_ ->
                Route.toString NotFound
                    |> Expect.equal "/not-found"
        ]



-- Helpers


{-| テスト用の URL パースヘルパー
-}
parseUrl : String -> Route
parseUrl path =
    -- テスト用のダミー URL を構築
    { protocol = Url.Http
    , host = "localhost"
    , port_ = Just 3000
    , path = path
    , query = Nothing
    , fragment = Nothing
    }
        |> Route.fromUrl
