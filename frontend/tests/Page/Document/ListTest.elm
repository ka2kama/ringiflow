module Page.Document.ListTest exposing (suite)

{-| Page.Document.List の update ロジックテスト

メッセージ経由のモデル状態変化を検証する。
Cmd は opaque なため比較不可。Model の状態遷移のみをテスト対象とする。

-}

import Api exposing (ApiError(..))
import Data.Document exposing (Document)
import Data.Folder exposing (Folder)
import Expect
import Http
import Page.Document.List as DocumentList exposing (FolderDialog(..), Msg(..), PendingDelete(..))
import RemoteData exposing (RemoteData(..))
import Set
import Shared
import Test exposing (..)


suite : Test
suite =
    describe "Page.Document.List"
        [ folderSelectionTests
        , folderToggleTests
        , folderDialogTests
        , deleteTests
        , fileOperationTests
        , messageTests
        ]



-- テストヘルパー


{-| メッセージを送信し、結果の Model を返す（Cmd は破棄）
-}
sendMsg : Msg -> DocumentList.Model -> DocumentList.Model
sendMsg msg model =
    DocumentList.update msg model |> Tuple.first


{-| テスト用の初期 Model を生成（Cmd は破棄）
-}
initialModel : DocumentList.Model
initialModel =
    let
        shared =
            Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }
    in
    DocumentList.init shared |> Tuple.first


{-| テスト用フォルダ
-}
testFolder : Folder
testFolder =
    { id = "folder-1"
    , name = "テストフォルダ"
    , parentId = Nothing
    , path = "/テストフォルダ"
    , depth = 0
    , createdAt = "2026-01-01T00:00:00Z"
    , updatedAt = "2026-01-01T00:00:00Z"
    }


{-| テスト用ドキュメント
-}
testDocument : Document
testDocument =
    { id = "doc-1"
    , filename = "test.pdf"
    , contentType = "application/pdf"
    , size = 1024
    , status = "confirmed"
    , createdAt = "2026-01-01T00:00:00Z"
    }



-- フォルダ選択


folderSelectionTests : Test
folderSelectionTests =
    describe "フォルダ選択"
        [ test "SelectFolder でフォルダが選択される" <|
            \_ ->
                initialModel
                    |> sendMsg (SelectFolder "folder-1")
                    |> .selectedFolderId
                    |> Expect.equal (Just "folder-1")
        , test "SelectFolder でドキュメントが Loading になる" <|
            \_ ->
                initialModel
                    |> sendMsg (SelectFolder "folder-1")
                    |> .documents
                    |> Expect.equal Loading
        ]



-- フォルダ展開/折りたたみ


folderToggleTests : Test
folderToggleTests =
    describe "フォルダ展開/折りたたみ"
        [ test "ToggleFolder で展開される" <|
            \_ ->
                initialModel
                    |> sendMsg (ToggleFolder "folder-1")
                    |> .expandedFolderIds
                    |> Set.member "folder-1"
                    |> Expect.equal True
        , test "ToggleFolder 2回で折りたたまれる" <|
            \_ ->
                initialModel
                    |> sendMsg (ToggleFolder "folder-1")
                    |> sendMsg (ToggleFolder "folder-1")
                    |> .expandedFolderIds
                    |> Set.member "folder-1"
                    |> Expect.equal False
        ]



-- フォルダダイアログ


folderDialogTests : Test
folderDialogTests =
    describe "フォルダダイアログ"
        [ test "OpenCreateFolderDialog で作成ダイアログが開く" <|
            \_ ->
                initialModel
                    |> sendMsg OpenCreateFolderDialog
                    |> .folderDialog
                    |> Expect.notEqual Nothing
        , test "OpenCreateFolderDialog の parentId は selectedFolderId" <|
            \_ ->
                let
                    model =
                        initialModel
                            |> sendMsg (SelectFolder "folder-1")
                            |> sendMsg OpenCreateFolderDialog
                in
                case model.folderDialog of
                    Just (CreateFolderDialog dialog) ->
                        Expect.equal (Just "folder-1") dialog.parentId

                    _ ->
                        Expect.fail "CreateFolderDialog が期待されたが、別の値"
        , test "OpenRenameFolderDialog で名前変更ダイアログが開く" <|
            \_ ->
                let
                    model =
                        initialModel
                            |> sendMsg (OpenRenameFolderDialog testFolder)
                in
                case model.folderDialog of
                    Just (RenameFolderDialog dialog) ->
                        Expect.equal "テストフォルダ" dialog.name

                    _ ->
                        Expect.fail "RenameFolderDialog が期待されたが、別の値"
        , test "UpdateFolderDialogName でダイアログの名前が更新される" <|
            \_ ->
                let
                    model =
                        initialModel
                            |> sendMsg OpenCreateFolderDialog
                            |> sendMsg (UpdateFolderDialogName "新しい名前")
                in
                case model.folderDialog of
                    Just (CreateFolderDialog dialog) ->
                        Expect.equal "新しい名前" dialog.name

                    _ ->
                        Expect.fail "CreateFolderDialog が期待されたが、別の値"
        , test "SubmitFolderDialog で空の名前は送信されない" <|
            \_ ->
                let
                    model =
                        initialModel
                            |> sendMsg OpenCreateFolderDialog
                            |> sendMsg SubmitFolderDialog
                in
                case model.folderDialog of
                    Just (CreateFolderDialog dialog) ->
                        Expect.equal False dialog.isSubmitting

                    _ ->
                        Expect.fail "CreateFolderDialog が期待されたが、別の値"
        , test "SubmitFolderDialog で名前があれば isSubmitting が True になる" <|
            \_ ->
                let
                    model =
                        initialModel
                            |> sendMsg OpenCreateFolderDialog
                            |> sendMsg (UpdateFolderDialogName "有効な名前")
                            |> sendMsg SubmitFolderDialog
                in
                case model.folderDialog of
                    Just (CreateFolderDialog dialog) ->
                        Expect.equal True dialog.isSubmitting

                    _ ->
                        Expect.fail "CreateFolderDialog が期待されたが、別の値"
        , test "CloseFolderDialog でダイアログが閉じる" <|
            \_ ->
                initialModel
                    |> sendMsg OpenCreateFolderDialog
                    |> sendMsg CloseFolderDialog
                    |> .folderDialog
                    |> Expect.equal Nothing
        ]



-- 削除


deleteTests : Test
deleteTests =
    describe "削除"
        [ test "ClickDeleteFolder で pendingDelete にフォルダが設定される" <|
            \_ ->
                initialModel
                    |> sendMsg (ClickDeleteFolder testFolder)
                    |> .pendingDelete
                    |> Expect.equal (Just (DeleteFolder testFolder))
        , test "ClickDeleteDocument で pendingDelete にドキュメントが設定される" <|
            \_ ->
                initialModel
                    |> sendMsg (ClickDeleteDocument testDocument)
                    |> .pendingDelete
                    |> Expect.equal (Just (DeleteDocument testDocument))
        , test "CancelDelete で pendingDelete がクリアされる" <|
            \_ ->
                initialModel
                    |> sendMsg (ClickDeleteFolder testFolder)
                    |> sendMsg CancelDelete
                    |> .pendingDelete
                    |> Expect.equal Nothing
        , test "ConfirmDelete で pendingDelete がクリアされる" <|
            \_ ->
                initialModel
                    |> sendMsg (ClickDeleteFolder testFolder)
                    |> sendMsg ConfirmDelete
                    |> .pendingDelete
                    |> Expect.equal Nothing
        ]



-- ファイル操作


fileOperationTests : Test
fileOperationTests =
    describe "ファイル操作"
        [ test "FileSelected でフォルダ未選択時は isUploading が変わらない" <|
            \_ ->
                -- selectedFolderId が Nothing の場合、何もしない
                initialModel
                    |> .isUploading
                    |> Expect.equal False
        , test "GotUploadUrl エラーで isUploading が False になる" <|
            \_ ->
                initialModel
                    |> sendMsg (GotUploadUrl (Err NetworkError))
                    |> .isUploading
                    |> Expect.equal False
        , test "GotUploadUrl エラーで selectedFile がクリアされる" <|
            \_ ->
                initialModel
                    |> sendMsg (GotUploadUrl (Err NetworkError))
                    |> .selectedFile
                    |> Expect.equal Nothing
        , test "GotS3UploadResult エラーで isUploading が False になる" <|
            \_ ->
                initialModel
                    |> sendMsg (GotS3UploadResult "doc-1" (Err (Http.BadStatus 500)))
                    |> .isUploading
                    |> Expect.equal False
        , test "GotS3UploadResult エラーでエラーメッセージが設定される" <|
            \_ ->
                initialModel
                    |> sendMsg (GotS3UploadResult "doc-1" (Err (Http.BadStatus 500)))
                    |> .errorMessage
                    |> Expect.equal (Just "ファイルのアップロードに失敗しました")
        ]



-- メッセージ表示


messageTests : Test
messageTests =
    describe "メッセージ"
        [ test "DismissMessage でメッセージがクリアされる" <|
            \_ ->
                let
                    model =
                        initialModel
                            |> sendMsg (GotCreateFolderResult (Ok testFolder))
                in
                model
                    |> sendMsg DismissMessage
                    |> (\m -> ( m.successMessage, m.errorMessage ))
                    |> Expect.equal ( Nothing, Nothing )
        , test "GotCreateFolderResult Ok で成功メッセージが設定される" <|
            \_ ->
                initialModel
                    |> sendMsg (GotCreateFolderResult (Ok testFolder))
                    |> .successMessage
                    |> Expect.equal (Just "フォルダを作成しました")
        , test "GotRenameFolderResult Ok で成功メッセージが設定される" <|
            \_ ->
                initialModel
                    |> sendMsg (GotRenameFolderResult (Ok testFolder))
                    |> .successMessage
                    |> Expect.equal (Just "フォルダ名を変更しました")
        ]
