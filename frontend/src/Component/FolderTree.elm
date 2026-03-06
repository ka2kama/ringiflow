module Component.FolderTree exposing (FolderNode, buildTree, childrenOf, folderOf)

{-| フォルダツリーコンポーネント

フラットなフォルダリストからツリー構造を構築し、
展開/折りたたみ可能なツリー UI を提供する。

-}

import Data.Folder exposing (Folder)
import Dict exposing (Dict)


{-| ツリーノード

再帰的なデータ構造のため `type` で定義する。
`type alias` は再帰を許容しないため。

-}
type FolderNode
    = FolderNode
        { folder : Folder
        , children : List FolderNode
        }


{-| ノードからフォルダを取得
-}
folderOf : FolderNode -> Folder
folderOf (FolderNode node) =
    node.folder


{-| ノードから子ノードを取得
-}
childrenOf : FolderNode -> List FolderNode
childrenOf (FolderNode node) =
    node.children


{-| フラットなフォルダリストからツリー構造を構築する

parentId が Nothing のフォルダをルートとし、
parentId で親子関係を組み立てる。

-}
buildTree : List Folder -> List FolderNode
buildTree folders =
    let
        -- parentId → 子フォルダのリスト のマップを構築
        childrenMap : Dict String (List Folder)
        childrenMap =
            List.foldl
                (\folder acc ->
                    case folder.parentId of
                        Just pid ->
                            Dict.update pid
                                (\existing ->
                                    case existing of
                                        Just list ->
                                            Just (list ++ [ folder ])

                                        Nothing ->
                                            Just [ folder ]
                                )
                                acc

                        Nothing ->
                            acc
                )
                Dict.empty
                folders

        -- 再帰的にノードを構築
        toNode : Folder -> FolderNode
        toNode folder =
            let
                children =
                    Dict.get folder.id childrenMap
                        |> Maybe.withDefault []
                        |> List.map toNode
            in
            FolderNode { folder = folder, children = children }

        -- ルートフォルダ（parentId が Nothing）を抽出
        roots =
            List.filter (\f -> f.parentId == Nothing) folders
    in
    List.map toNode roots
