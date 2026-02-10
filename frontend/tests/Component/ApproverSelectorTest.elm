module Component.ApproverSelectorTest exposing (suite)

{-| Component.ApproverSelector の handleKeyDown テスト

純粋関数のキーボードナビゲーションロジックを検証する。

-}

import Component.ApproverSelector as ApproverSelector exposing (KeyResult(..))
import Data.UserItem exposing (UserItem)
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Component.ApproverSelector"
        [ handleKeyDownTests
        , initTests
        ]



-- ────────────────────────────────────
-- テストヘルパー
-- ────────────────────────────────────


testUser1 : UserItem
testUser1 =
    { id = "u-001"
    , displayId = "U-1"
    , displayNumber = 1
    , name = "山田太郎"
    , email = "yamada@example.com"
    }


testUser2 : UserItem
testUser2 =
    { id = "u-002"
    , displayId = "U-2"
    , displayNumber = 2
    , name = "山田次郎"
    , email = "yamada2@example.com"
    }


twoCandidates : List UserItem
twoCandidates =
    [ testUser1, testUser2 ]



-- ────────────────────────────────────
-- handleKeyDown
-- ────────────────────────────────────


handleKeyDownTests : Test
handleKeyDownTests =
    describe "handleKeyDown"
        [ test "ArrowDown で Navigate を返す" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "ArrowDown"
                    , candidates = twoCandidates
                    , highlightIndex = 0
                    }
                    |> Expect.equal (Navigate 1)
        , test "ArrowUp で循環 Navigate を返す（index 0 → 末尾）" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "ArrowUp"
                    , candidates = twoCandidates
                    , highlightIndex = 0
                    }
                    |> Expect.equal (Navigate 1)
        , test "Enter で候補があれば Select を返す" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "Enter"
                    , candidates = twoCandidates
                    , highlightIndex = 0
                    }
                    |> Expect.equal (Select testUser1)
        , test "Enter で候補なしなら NoChange を返す" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "Enter"
                    , candidates = []
                    , highlightIndex = 0
                    }
                    |> Expect.equal NoChange
        , test "Escape で Close を返す" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "Escape"
                    , candidates = twoCandidates
                    , highlightIndex = 0
                    }
                    |> Expect.equal Close
        , test "候補0件の ArrowDown で NoChange を返す" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "ArrowDown"
                    , candidates = []
                    , highlightIndex = 0
                    }
                    |> Expect.equal NoChange
        , test "不明キーで NoChange を返す" <|
            \_ ->
                ApproverSelector.handleKeyDown
                    { key = "Tab"
                    , candidates = twoCandidates
                    , highlightIndex = 0
                    }
                    |> Expect.equal NoChange
        ]



-- ────────────────────────────────────
-- init
-- ────────────────────────────────────


initTests : Test
initTests =
    describe "init"
        [ test "初期状態が NotSelected, 空文字, False, 0" <|
            \_ ->
                let
                    state =
                        ApproverSelector.init
                in
                Expect.all
                    [ \s -> s.selection |> Expect.equal ApproverSelector.NotSelected
                    , \s -> s.search |> Expect.equal ""
                    , \s -> s.dropdownOpen |> Expect.equal False
                    , \s -> s.highlightIndex |> Expect.equal 0
                    ]
                    state
        ]
