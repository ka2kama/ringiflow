module Route exposing (Route(..), fromUrl, toString)

{-| URL ルーティングモジュール

このモジュールは URL とアプリケーションの画面（Route）を
相互変換する責務を持つ。


## 設計方針

ルーティングを独立したモジュールに分離することで:

1.  **関心の分離**: Main.elm はルーティングの詳細を知らなくてよい
2.  **テスタビリティ**: ルーティングロジックを単独でテスト可能
3.  **拡張性**: 新しいルートの追加が容易


## Elm のルーティングアプローチ

Elm では URL パーサーを宣言的に定義する。
これは React Router や Vue Router とは異なるアプローチ:

  - **React/Vue**: コンポーネントベース（JSX 内でルート定義）
  - **Elm**: データ指向（パーサーコンビネータで定義）


## URL 解析の流れ

```text
URL 文字列
    ↓ (Browser がパース)
Url 型
    ↓ (Route.fromUrl)
Route 型
    ↓ (Main.view で分岐)
対応する画面
```


## 代替案と不採用理由

  - **elm-spa**: ルーティングを自動生成するが、
    学習目的で手動実装を選択
  - **Hash ベースルーティング**: SEO に不利、
    モダンな SPA では History API を使用するのが主流

-}

import Url exposing (Url)
import Url.Parser as Parser exposing ((</>), Parser, oneOf, s, top)


{-| アプリケーションのルート（画面）を表す型


## 設計意図

Route はカスタム型（Tagged Union / Sum Type）として定義。
これにより:

1.  **型安全**: 存在しないルートを参照できない
2.  **網羅性チェック**: case 式で全ルートを処理しないとコンパイルエラー
3.  **IDE サポート**: 補完やリファクタリングが容易


## 現在のルート

  - `Home`: トップページ（`/`）
  - `WorkflowNew`: 新規申請（`/workflows/new`）
  - `NotFound`: 存在しないパス

-}
type Route
    = Home
    | WorkflowNew
    | NotFound


{-| URL パーサー


## パーサーコンビネータについて

Elm の Url.Parser は「パーサーコンビネータ」パターンを使用。
小さなパーサーを組み合わせて複雑なパーサーを構築する。


## 主要なコンビネータ

  - `top`: ルートパス（`/`）にマッチ
  - `s "path"`: 固定文字列にマッチ
  - `</>`: パーサーを連結
  - `oneOf`: 複数のパーサーを試行
  - `int`: 整数にマッチしてキャプチャ
  - `string`: 文字列にマッチしてキャプチャ


## 例: 複数ルートの定義

    parser =
        oneOf
            [ Parser.map Home top
            , Parser.map Workflows (s "workflows")
            , Parser.map WorkflowDetail (s "workflows" </> int)
            , Parser.map Settings (s "settings")
            ]


## 型シグネチャの解説

`Parser (Route -> a) a` は「Route を受け取って a を返す関数」を
「a」に変換するパーサー。これは関数合成のためのトリックで、
複数のパーサーを `oneOf` で合成可能にしている。

-}
parser : Parser (Route -> a) a
parser =
    oneOf
        [ Parser.map Home top
        , Parser.map WorkflowNew (s "workflows" </> s "new")
        ]


{-| URL を Route に変換


## 処理フロー

1.  `Parser.parse` で URL をパース
2.  成功すれば `Just Route` を返す
3.  失敗すれば `Nothing` を返す
4.  `Maybe.withDefault` で失敗時は `NotFound` に変換


## なぜ Maybe を使うか

URL パースは失敗する可能性がある（ユーザーが直接 URL を入力など）。
Elm は例外を使わず、Maybe 型で失敗を表現する。

-}
fromUrl : Url -> Route
fromUrl url =
    Parser.parse parser url
        |> Maybe.withDefault NotFound


{-| Route を URL 文字列に変換


## 用途

  - リンクの生成
  - プログラム的なナビゲーション
  - ブックマーク URL の生成


## 双方向性

`fromUrl` と `toString` は互いに逆変換の関係にある。
これにより、URL とルートの一貫性が保証される:

    -- この等式が常に成り立つべき
    fromUrl (parseUrl (toString route)) == route

-}
toString : Route -> String
toString route =
    case route of
        Home ->
            "/"

        WorkflowNew ->
            "/workflows/new"

        NotFound ->
            "/not-found"
