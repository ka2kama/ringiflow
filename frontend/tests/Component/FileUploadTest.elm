module Component.FileUploadTest exposing (suite)

{-| Component.FileUpload モジュールのテスト

ファイルバリデーション（Content-Type、サイズ上限、ファイル数上限）を検証する。

-}

import Component.FileUpload exposing (FileError(..), validateFile, validateFileCount)
import Data.FormField exposing (FileConfig)
import Expect
import Test exposing (..)


suite : Test
suite =
    describe "Component.FileUpload"
        [ validateFileTests
        , validateFileCountTests
        ]



-- validateFile


validateFileTests : Test
validateFileTests =
    let
        config : FileConfig
        config =
            { maxFiles = 5
            , maxFileSize = 10485760
            , allowedTypes = [ "application/pdf", "image/png", "image/jpeg" ]
            }
    in
    describe "validateFile"
        [ test "許可された Content-Type のファイルを通過させる" <|
            \_ ->
                validateFile config { name = "document.pdf", size = 1024, mime = "application/pdf" }
                    |> Expect.equal []
        , test "許可されていない Content-Type を拒否する" <|
            \_ ->
                validateFile config { name = "script.exe", size = 1024, mime = "application/x-msdownload" }
                    |> Expect.equal [ InvalidType ]
        , test "サイズ上限超過を拒否する" <|
            \_ ->
                validateFile config { name = "large.pdf", size = 20971520, mime = "application/pdf" }
                    |> Expect.equal [ FileTooLarge ]
        , test "Content-Type とサイズの両方が不正な場合は両方のエラーを返す" <|
            \_ ->
                validateFile config { name = "large.exe", size = 20971520, mime = "application/x-msdownload" }
                    |> Expect.equal [ InvalidType, FileTooLarge ]
        , test "allowedTypes が空の場合は全形式を許可する" <|
            \_ ->
                let
                    openConfig =
                        { config | allowedTypes = [] }
                in
                validateFile openConfig { name = "any.exe", size = 1024, mime = "application/x-msdownload" }
                    |> Expect.equal []
        , test "サイズ上限ちょうどのファイルは通過する" <|
            \_ ->
                validateFile config { name = "exact.pdf", size = 10485760, mime = "application/pdf" }
                    |> Expect.equal []
        ]



-- validateFileCount


validateFileCountTests : Test
validateFileCountTests =
    let
        config : FileConfig
        config =
            { maxFiles = 3
            , maxFileSize = 10485760
            , allowedTypes = []
            }
    in
    describe "validateFileCount"
        [ test "ファイル数が上限以内なら Nothing" <|
            \_ ->
                validateFileCount config { existingCount = 1, newCount = 2 }
                    |> Expect.equal Nothing
        , test "既存 + 新規がちょうど上限なら Nothing" <|
            \_ ->
                validateFileCount config { existingCount = 2, newCount = 1 }
                    |> Expect.equal Nothing
        , test "既存 + 新規が上限を超える場合エラー" <|
            \_ ->
                validateFileCount config { existingCount = 2, newCount = 2 }
                    |> Expect.equal (Just TooManyFiles)
        , test "新規のみで上限を超える場合エラー" <|
            \_ ->
                validateFileCount config { existingCount = 0, newCount = 4 }
                    |> Expect.equal (Just TooManyFiles)
        ]
