module Example exposing (..)

{-| Route モジュールのテスト

このファイルは elm-test フレームワークを使用した
ユニットテストの例を示す。


## elm-test について

elm-test は Elm 公式のテストフレームワーク。
以下の特徴を持つ:

  - **純粋関数のテスト**: 副作用がないため、テストが簡潔
  - **Fuzzing**: プロパティベーステスト（ランダム入力）
  - **高速実行**: Node.js 上で直接実行


## テストの構造

```elm
suite : Test
suite =
    describe "モジュール名"
        [ describe "関数名"
            [ test "テストケース名" <|
                \_ ->
                    -- テストコード
                    actual
                        |> Expect.equal expected
            ]
        ]
```

  - `describe`: テストをグループ化
  - `test`: 個別のテストケース
  - `Expect.*`: アサーション関数


## なぜルーティングをテストするか

URL パーシングは:

1.  **ユーザー入力に依存**: 直接 URL 入力の可能性
2.  **リグレッションしやすい**: ルート追加時に既存が壊れやすい
3.  **純粋関数**: テストが容易

他のモジュール（例: view）は副作用がないため
テスト可能だが、視覚的な確認が重要な場合は
手動テストやスナップショットテストが適切。


## 代替案と不採用理由

  - **elm-program-test**: 統合テスト向け。
    Phase 0 ではユニットテストで十分
  - **elm-test-rs**: Rust 実装で高速だが、
    公式ツールの安定性を優先

-}

import Expect
import Route
import Test exposing (..)
import Url


{-| テストスイート

## 命名規約

テストスイートは `suite` という名前で定義するのが慣例。
elm-test は `tests/` ディレクトリ内の `.elm` ファイルを
自動検出し、`Test` 型の値を探して実行する。

## 構成

```
suite
├── describe "Route モジュール"
│   ├── describe "fromUrl"
│   │   ├── test "ルートパスは Home にマッチする"
│   │   └── test "不明なパスは NotFound にマッチする"
│   └── describe "toString"
│       └── test "Home は / を返す"
```

-}
suite : Test
suite =
    describe "Route モジュール"
        [ describe "fromUrl"
            [ test "ルートパスは Home にマッチする" <|
                {- ## テストの解説

                   このテストは `Route.fromUrl` が
                   ルートパス（`/`）を正しく `Route.Home` に
                   変換することを検証する。

                   ## Url 型の構築

                   Elm の Url 型は以下のフィールドを持つ:

                   ```elm
                   type alias Url =
                       { protocol : Protocol  -- Http | Https
                       , host : String
                       , port_ : Maybe Int
                       , path : String
                       , query : Maybe String
                       , fragment : Maybe String
                       }
                   ```

                   テストでは完全な Url を構築する必要がある。
                   実際のアプリケーションでは Browser が自動で構築。

                   ## `\_ ->` について

                   elm-test の test 関数は `() -> Expectation` を受け取る。
                   `\_ ->` は引数を無視するラムダ式。
                   `()` は Unit 型（意味のある値がない）。
                -}
                \_ ->
                    let
                        url =
                            { protocol = Url.Https
                            , host = "localhost"
                            , port_ = Nothing
                            , path = "/"
                            , query = Nothing
                            , fragment = Nothing
                            }
                    in
                    Route.fromUrl url
                        |> Expect.equal Route.Home
            , test "不明なパスは NotFound にマッチする" <|
                {- ## テストの解説

                   このテストは未定義のパスが
                   `Route.NotFound` にフォールバックすることを検証。

                   これは重要なエッジケース:
                   - ユーザーが直接 URL を入力
                   - 古いブックマーク
                   - 外部サイトからの不正なリンク

                   いずれの場合も、アプリケーションは
                   クラッシュせず 404 ページを表示すべき。
                -}
                \_ ->
                    let
                        url =
                            { protocol = Url.Https
                            , host = "localhost"
                            , port_ = Nothing
                            , path = "/unknown-path"
                            , query = Nothing
                            , fragment = Nothing
                            }
                    in
                    Route.fromUrl url
                        |> Expect.equal Route.NotFound
            ]
        , describe "toString"
            [ test "Home は / を返す" <|
                {- ## テストの解説

                   `Route.toString` の逆変換をテスト。
                   これにより、リンク生成が正しく動作することを保証。

                   ## 双方向テストの重要性

                   理想的には以下のプロパティを検証すべき:

                   ```elm
                   -- 任意のルートに対して
                   fromUrl (parseUrl (toString route)) == route
                   ```

                   これは Fuzzing（プロパティベーステスト）で
                   自動的に検証可能。Phase 1 以降で追加予定。
                -}
                \_ ->
                    Route.toString Route.Home
                        |> Expect.equal "/"
            ]
        ]



{- ## 将来のテスト拡張

### Fuzzing（プロパティベーステスト）

```elm
import Fuzz exposing (Fuzzer)

routeFuzzer : Fuzzer Route
routeFuzzer =
    Fuzz.oneOf
        [ Fuzz.constant Route.Home
        -- 他のルートを追加
        ]

fuzzSuite : Test
fuzzSuite =
    describe "プロパティベーステスト"
        [ fuzz routeFuzzer "toString と fromUrl は逆変換" <|
            \route ->
                route
                    |> Route.toString
                    |> parseUrl
                    |> Route.fromUrl
                    |> Expect.equal route
        ]
```


### HTTP テスト

```elm
import Http
import Test.Http

httpSuite : Test
httpSuite =
    describe "API テスト"
        [ test "ユーザー取得" <|
            \_ ->
                -- elm-program-test を使用
                ...
        ]
```


### View テスト

```elm
import Test.Html.Query as Query
import Test.Html.Selector as Selector

viewSuite : Test
viewSuite =
    describe "View テスト"
        [ test "ホームページにタイトルが表示される" <|
            \_ ->
                viewHome
                    |> Query.fromHtml
                    |> Query.find [ Selector.tag "h2" ]
                    |> Query.has [ Selector.text "ようこそ" ]
        ]
```

-}
