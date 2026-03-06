module Component.FolderTreeTest exposing (suite)

{-| フォルダツリー構築ロジックのテスト
-}

import Component.FolderTree exposing (buildTree, childrenOf, folderOf)
import Data.Folder exposing (Folder)
import Expect
import Test exposing (Test, describe, test)


suite : Test
suite =
    describe "FolderTree"
        [ describe "buildTree"
            [ test "空リストの場合は空リストを返す" <|
                \_ ->
                    buildTree []
                        |> Expect.equal []
            , test "ルートフォルダのみの場合" <|
                \_ ->
                    let
                        folders =
                            [ makeFolder "1" "経費精算" Nothing "/" 0
                            , makeFolder "2" "稟議書" Nothing "/" 0
                            ]
                    in
                    buildTree folders
                        |> List.map folderOf
                        |> List.map .name
                        |> Expect.equal [ "経費精算", "稟議書" ]
            , test "ルートフォルダのみの場合、子は空" <|
                \_ ->
                    let
                        folders =
                            [ makeFolder "1" "経費精算" Nothing "/" 0
                            ]
                    in
                    buildTree folders
                        |> List.concatMap childrenOf
                        |> Expect.equal []
            , test "2階層のネスト" <|
                \_ ->
                    let
                        folders =
                            [ makeFolder "1" "経費精算" Nothing "/" 0
                            , makeFolder "2" "2026年度" (Just "1") "/経費精算/" 1
                            , makeFolder "3" "2025年度" (Just "1") "/経費精算/" 1
                            ]

                        result =
                            buildTree folders
                    in
                    -- ルートは1つ
                    List.length result
                        |> Expect.equal 1
            , test "2階層のネスト: 子フォルダの名前" <|
                \_ ->
                    let
                        folders =
                            [ makeFolder "1" "経費精算" Nothing "/" 0
                            , makeFolder "2" "2026年度" (Just "1") "/経費精算/" 1
                            , makeFolder "3" "2025年度" (Just "1") "/経費精算/" 1
                            ]
                    in
                    buildTree folders
                        |> List.concatMap childrenOf
                        |> List.map folderOf
                        |> List.map .name
                        |> Expect.equal [ "2026年度", "2025年度" ]
            , test "3階層以上のネスト" <|
                \_ ->
                    let
                        folders =
                            [ makeFolder "1" "経費精算" Nothing "/" 0
                            , makeFolder "2" "2026年度" (Just "1") "/経費精算/" 1
                            , makeFolder "3" "Q1" (Just "2") "/経費精算/2026年度/" 2
                            ]

                        grandChildren =
                            buildTree folders
                                |> List.concatMap childrenOf
                                |> List.concatMap childrenOf
                    in
                    grandChildren
                        |> List.map folderOf
                        |> List.map .name
                        |> Expect.equal [ "Q1" ]
            , test "複数ルートフォルダそれぞれに子がある場合" <|
                \_ ->
                    let
                        folders =
                            [ makeFolder "1" "経費精算" Nothing "/" 0
                            , makeFolder "2" "稟議書" Nothing "/" 0
                            , makeFolder "3" "2026年度" (Just "1") "/経費精算/" 1
                            , makeFolder "4" "契約" (Just "2") "/稟議書/" 1
                            ]

                        result =
                            buildTree folders
                    in
                    -- ルートは2つ
                    List.length result
                        |> Expect.equal 2
            ]
        ]



-- HELPERS


makeFolder : String -> String -> Maybe String -> String -> Int -> Folder
makeFolder id name parentId path depth =
    { id = id
    , name = name
    , parentId = parentId
    , path = path
    , depth = depth
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }
