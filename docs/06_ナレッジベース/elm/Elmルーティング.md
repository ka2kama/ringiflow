# Elm ルーティング

Elm SPA におけるルーティングの仕組みと実装パターンを解説する。

## 概要

Elm では `Browser.application` を使用して SPA を構築する。
ルーティングは以下の要素で構成される：

| 要素 | 役割 |
|------|------|
| `Url` | ブラウザの URL を表す型 |
| `Route` | アプリケーション固有のルート型（カスタム型） |
| `Url.Parser` | URL を Route に変換するパーサー |
| `Browser.Navigation` | URL 操作（pushUrl, load など） |

## 全体フロー

```mermaid
flowchart TB
    subgraph Browser["ブラウザ"]
        URL["URL バー"]
        DOM["DOM"]
        History["History API"]
    end

    subgraph Elm["Elm ランタイム"]
        subgraph Init["初期化"]
            Flags["Flags"]
            InitUrl["初期 URL"]
        end

        subgraph TEA["TEA サイクル"]
            Model["Model"]
            Update["update"]
            View["view"]
            Msg["Msg"]
        end

        subgraph Routing["ルーティング"]
            Route["Route.elm"]
            Parser["Url.Parser"]
        end
    end

    URL -->|"ページロード"| InitUrl
    InitUrl -->|"Route.fromUrl"| Route
    Route -->|"Route 型"| Model

    DOM -->|"リンククリック"| Msg
    Msg -->|"LinkClicked"| Update
    Update -->|"Nav.pushUrl"| History
    History -->|"URL 変更イベント"| Msg
    Msg -->|"UrlChanged"| Update
    Update -->|"Route.fromUrl"| Route

    Model --> View
    View --> DOM
```

## ルーティングフロー詳細

### 1. 初期ロード時

ユーザーが URL に直接アクセスした場合のフロー。

```mermaid
sequenceDiagram
    participant User as ユーザー
    participant Browser as ブラウザ
    participant Elm as Elm App
    participant Route as Route.elm

    User->>Browser: URL にアクセス<br/>/workflows/new
    Browser->>Elm: init(flags, url, key)
    Note over Elm: url = { path = "/workflows/new", ... }
    Elm->>Route: Route.fromUrl(url)
    Route->>Route: Parser.parse(parser, url)
    Route-->>Elm: WorkflowNew
    Elm->>Elm: Model 初期化<br/>{ route = WorkflowNew, ... }
    Elm->>Browser: view(model) → HTML
    Browser->>User: ページ表示
```

### 2. 内部リンククリック時

SPA 内でのリンク遷移。ページリロードは発生しない。

```mermaid
sequenceDiagram
    participant User as ユーザー
    participant Browser as ブラウザ
    participant Elm as Elm App
    participant Route as Route.elm
    participant Nav as Browser.Navigation

    User->>Browser: リンクをクリック<br/><a href="/workflows/new">
    Browser->>Elm: onUrlRequest イベント
    Elm->>Elm: Msg = LinkClicked (Internal url)

    rect rgb(230, 245, 255)
        Note over Elm: update 関数
        Elm->>Nav: Nav.pushUrl key "/workflows/new"
    end

    Nav->>Browser: History API で URL 更新
    Note over Browser: URL バーが変わる<br/>（ページリロードなし）

    Browser->>Elm: onUrlChange イベント
    Elm->>Elm: Msg = UrlChanged url

    rect rgb(230, 245, 255)
        Note over Elm: update 関数
        Elm->>Route: Route.fromUrl(url)
        Route-->>Elm: WorkflowNew
        Elm->>Elm: Model 更新<br/>{ route = WorkflowNew }
    end

    Elm->>Browser: view(model) → 新しい HTML
    Browser->>User: 新しいページ表示
```

### 3. ブラウザの戻る/進むボタン

ブラウザの履歴操作時のフロー。

```mermaid
sequenceDiagram
    participant User as ユーザー
    participant Browser as ブラウザ
    participant Elm as Elm App
    participant Route as Route.elm

    User->>Browser: 戻るボタンをクリック
    Browser->>Browser: History API が URL を復元
    Browser->>Elm: onUrlChange イベント
    Elm->>Elm: Msg = UrlChanged url
    Elm->>Route: Route.fromUrl(url)
    Route-->>Elm: 前のページの Route
    Elm->>Elm: Model 更新
    Elm->>Browser: view(model)
    Browser->>User: 前のページ表示
```

## Route 型とページの対応

```mermaid
flowchart LR
    subgraph URLs["URL パス"]
        U1["/"]
        U2["/workflows/new"]
        U3["/about"]
        U4["/xyz"]
    end

    subgraph Routes["Route 型"]
        R1["Home"]
        R2["WorkflowNew"]
        R3["NotFound"]
    end

    subgraph Pages["ページ表示"]
        P1["ホーム画面"]
        P2["申請フォーム"]
        P3["404 画面"]
    end

    U1 --> R1
    U2 --> R2
    U3 --> R3
    U4 --> R3

    R1 --> P1
    R2 --> P2
    R3 --> P3
```

## 実装パターン

### Route.elm

```elm
module Route exposing (Route(..), fromUrl, toString)

import Url exposing (Url)
import Url.Parser as Parser exposing (Parser, oneOf, s, top)


{-| アプリケーションのルート（画面）を表す型

カスタム型として定義することで：
1. 存在しないルートを参照できない（型安全）
2. case 式で全ルートを処理しないとコンパイルエラー（網羅性チェック）

-}
type Route
    = Home
    | WorkflowNew
    | NotFound


{-| URL パーサー

パーサーコンビネータで URL パターンを定義する。
-}
parser : Parser (Route -> a) a
parser =
    oneOf
        [ Parser.map Home top                           -- /
        , Parser.map WorkflowNew (s "workflows" </> s "new")  -- /workflows/new
        ]


{-| URL を Route に変換
-}
fromUrl : Url -> Route
fromUrl url =
    Parser.parse parser url
        |> Maybe.withDefault NotFound


{-| Route を URL 文字列に変換（リンク生成用）
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
```

### Main.elm

```elm
import Browser
import Browser.Navigation as Nav
import Route exposing (Route)
import Url exposing (Url)


main : Program Flags Model Msg
main =
    Browser.application
        { init = init
        , view = view
        , update = update
        , subscriptions = subscriptions
        , onUrlRequest = LinkClicked   -- リンククリック時
        , onUrlChange = UrlChanged     -- URL 変更時
        }


type Msg
    = LinkClicked Browser.UrlRequest
    | UrlChanged Url


update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        -- Step 1: リンクがクリックされた
        LinkClicked urlRequest ->
            case urlRequest of
                Browser.Internal url ->
                    -- 内部リンク → URL を変更（リロードなし）
                    ( model, Nav.pushUrl model.key (Url.toString url) )

                Browser.External href ->
                    -- 外部リンク → 通常の遷移（リロードあり）
                    ( model, Nav.load href )

        -- Step 2: URL が変更された（pushUrl の結果 or 戻る/進む）
        UrlChanged url ->
            -- Route を更新して Model に反映
            ( { model | url = url, route = Route.fromUrl url }
            , Cmd.none
            )
```

## 主要な関数・型

| 関数/型 | 説明 |
|--------|------|
| `Browser.application` | SPA を構築するプログラム型 |
| `Nav.Key` | URL 操作に必要なキー（init で取得） |
| `Nav.pushUrl` | URL を変更（履歴に追加） |
| `Nav.replaceUrl` | URL を変更（履歴を置換） |
| `Nav.load` | 外部 URL にリダイレクト（リロード） |
| `Browser.UrlRequest` | `Internal Url` または `External String` |
| `Url.Parser` | URL パターンマッチング用のパーサー |

## 2段階処理の理由

Elm のルーティングが `LinkClicked` → `UrlChanged` の2段階になっている理由：

1. **制御の分離**: リンククリックと実際の URL 変更を分離
2. **柔軟性**: `LinkClicked` で遷移を中断・変更可能（例: 未保存データの確認）
3. **統一的な処理**: 戻る/進むボタンも `UrlChanged` で同じ処理

```elm
LinkClicked urlRequest ->
    case urlRequest of
        Browser.Internal url ->
            if model.hasUnsavedChanges then
                -- 未保存データがあれば確認ダイアログを表示
                ( { model | showConfirmDialog = True, pendingUrl = Just url }
                , Cmd.none
                )
            else
                -- 通常の遷移
                ( model, Nav.pushUrl model.key (Url.toString url) )

        Browser.External href ->
            ( model, Nav.load href )
```

## パーサーコンビネータ

`Url.Parser` の主要なコンビネータ：

| コンビネータ | 説明 | 例 |
|-------------|------|-----|
| `top` | ルートパス `/` | `Parser.map Home top` |
| `s "path"` | 固定文字列 | `s "workflows"` → `/workflows` |
| `</>` | パス連結 | `s "users" </> int` → `/users/123` |
| `int` | 整数をキャプチャ | `/users/123` → `123` |
| `string` | 文字列をキャプチャ | `/users/abc` → `"abc"` |
| `oneOf` | 複数パーサーを試行 | 最初にマッチしたものを採用 |

## 関連ドキュメント

- [Elm アーキテクチャ](./Elmアーキテクチャ.md)
- [Elm ポート](./Elmポート.md)
- [申請フォーム UI 設計](../../03_詳細設計書/10_ワークフロー申請フォームUI設計.md)

## 参考

- [Elm Guide: Navigation](https://guide.elm-lang.org/webapps/navigation.html)
- [Url.Parser ドキュメント](https://package.elm-lang.org/packages/elm/url/latest/Url-Parser)
