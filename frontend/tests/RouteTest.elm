module RouteTest exposing (suite)

{-| Route モジュールのテスト

URL パースと文字列変換の正確性を検証する。
クエリパラメータ対応（Issue #267）のテストを含む。

-}

import Data.WorkflowInstance exposing (Status(..))
import Expect
import Route exposing (Route(..), emptyWorkflowFilter)
import Test exposing (..)
import Url


suite : Test
suite =
    describe "Route"
        [ fromUrlTests
        , toStringTests
        , roundtripTests
        , isRouteActiveTests
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
        , test "/workflows → Workflows emptyWorkflowFilter" <|
            \_ ->
                parseUrl "/workflows"
                    |> Expect.equal (Workflows emptyWorkflowFilter)
        , test "/workflows/{display_number} → WorkflowDetail display_number" <|
            \_ ->
                parseUrl "/workflows/42"
                    |> Expect.equal (WorkflowDetail 42)
        , test "/workflows/{non-integer} → NotFound" <|
            \_ ->
                parseUrl "/workflows/abc-123-def"
                    |> Expect.equal NotFound
        , test "/tasks → Tasks" <|
            \_ ->
                parseUrl "/tasks"
                    |> Expect.equal Tasks
        , test "/unknown → NotFound" <|
            \_ ->
                parseUrl "/unknown/path"
                    |> Expect.equal NotFound
        , describe "クエリパラメータ"
            [ test "/workflows?status=in_progress → InProgress フィルタ" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "status=in_progress"
                        |> Expect.equal
                            (Workflows { status = Just InProgress, completedToday = False })
            , test "/workflows?status=draft → Draft フィルタ" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "status=draft"
                        |> Expect.equal
                            (Workflows { status = Just Draft, completedToday = False })
            , test "/workflows?completed_today=true → completedToday フィルタ" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "completed_today=true"
                        |> Expect.equal
                            (Workflows { status = Nothing, completedToday = True })
            , test "/workflows?status=approved&completed_today=true → 両方反映" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "status=approved&completed_today=true"
                        |> Expect.equal
                            (Workflows { status = Just Approved, completedToday = True })
            , test "/workflows?status=invalid → 無効値は無視" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "status=invalid"
                        |> Expect.equal (Workflows emptyWorkflowFilter)
            , test "/workflows?completed_today=false → False として扱う" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "completed_today=false"
                        |> Expect.equal (Workflows emptyWorkflowFilter)
            , test "/workflows?completed_today=invalid → False として扱う" <|
                \_ ->
                    parseUrlWithQuery "/workflows" "completed_today=invalid"
                        |> Expect.equal (Workflows emptyWorkflowFilter)
            ]
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
        , test "Workflows emptyWorkflowFilter → /workflows" <|
            \_ ->
                Route.toString (Workflows emptyWorkflowFilter)
                    |> Expect.equal "/workflows"
        , test "WorkflowDetail display_number → /workflows/{display_number}" <|
            \_ ->
                Route.toString (WorkflowDetail 42)
                    |> Expect.equal "/workflows/42"
        , test "NotFound → /not-found" <|
            \_ ->
                Route.toString NotFound
                    |> Expect.equal "/not-found"
        , describe "クエリパラメータ"
            [ test "status=InProgress → /workflows?status=in_progress" <|
                \_ ->
                    Route.toString (Workflows { status = Just InProgress, completedToday = False })
                        |> Expect.equal "/workflows?status=in_progress"
            , test "completedToday=True → /workflows?completed_today=true" <|
                \_ ->
                    Route.toString (Workflows { status = Nothing, completedToday = True })
                        |> Expect.equal "/workflows?completed_today=true"
            , test "両方指定 → 両パラメータ含む" <|
                \_ ->
                    Route.toString (Workflows { status = Just Approved, completedToday = True })
                        |> Expect.equal "/workflows?status=approved&completed_today=true"
            ]
        ]



-- ラウンドトリップ


roundtripTests : Test
roundtripTests =
    describe "ラウンドトリップ (fromUrl ∘ toString = identity)"
        [ test "Workflows emptyWorkflowFilter" <|
            \_ ->
                roundtrip (Workflows emptyWorkflowFilter)
                    |> Expect.equal (Workflows emptyWorkflowFilter)
        , test "Workflows { status = Just InProgress }" <|
            \_ ->
                let
                    route =
                        Workflows { status = Just InProgress, completedToday = False }
                in
                roundtrip route
                    |> Expect.equal route
        , test "Workflows { completedToday = True }" <|
            \_ ->
                let
                    route =
                        Workflows { status = Nothing, completedToday = True }
                in
                roundtrip route
                    |> Expect.equal route
        , test "Workflows { status = Just Approved, completedToday = True }" <|
            \_ ->
                let
                    route =
                        Workflows { status = Just Approved, completedToday = True }
                in
                roundtrip route
                    |> Expect.equal route
        ]



-- isRouteActive


isRouteActiveTests : Test
isRouteActiveTests =
    describe "isRouteActive"
        [ test "Workflows はフィルタを無視して比較" <|
            \_ ->
                let
                    navRoute =
                        Workflows emptyWorkflowFilter

                    currentRoute =
                        Workflows { status = Just InProgress, completedToday = True }
                in
                Route.isRouteActive navRoute currentRoute
                    |> Expect.equal True
        , test "Workflows ナビは WorkflowNew でもアクティブ" <|
            \_ ->
                Route.isRouteActive (Workflows emptyWorkflowFilter) WorkflowNew
                    |> Expect.equal True
        , test "Workflows ナビは WorkflowDetail でもアクティブ" <|
            \_ ->
                Route.isRouteActive (Workflows emptyWorkflowFilter) (WorkflowDetail 1)
                    |> Expect.equal True
        ]



-- Helpers


{-| テスト用の URL パースヘルパー（クエリなし）
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


{-| テスト用の URL パースヘルパー（クエリあり）
-}
parseUrlWithQuery : String -> String -> Route
parseUrlWithQuery path queryString =
    { protocol = Url.Http
    , host = "localhost"
    , port_ = Just 3000
    , path = path
    , query = Just queryString
    , fragment = Nothing
    }
        |> Route.fromUrl


{-| ラウンドトリップテスト用ヘルパー

toString で URL 文字列に変換し、fromUrl で元の Route に戻す。
toString は path のみ返すため、パース時に query を抽出する必要がある。

-}
roundtrip : Route -> Route
roundtrip route =
    let
        urlString =
            Route.toString route

        -- "/workflows?status=in_progress" → path="/workflows", query="status=in_progress"
        ( path, query ) =
            case String.split "?" urlString of
                [ p ] ->
                    ( p, Nothing )

                [ p, q ] ->
                    ( p, Just q )

                _ ->
                    ( urlString, Nothing )
    in
    { protocol = Url.Http
    , host = "localhost"
    , port_ = Just 3000
    , path = path
    , query = query
    , fragment = Nothing
    }
        |> Route.fromUrl
