# Elm TEA メインループ

自分の言葉で理解を整理するノート。

## Browser.application の5つの関数

`Main.elm` のエントリーポイント。TEA の全体を統括する。

```elm
main =
    Browser.application
        { init = init              -- 起動時に1回
        , view = view              -- Model → HTML
        , update = update          -- Msg → Model 更新
        , subscriptions = ...      -- 外部イベント購読
        , onUrlChange = UrlChanged
        , onUrlRequest = LinkClicked
        }
```

### Browser モジュールの4段階

| 関数 | URL 管理 | 用途 |
|------|----------|------|
| `sandbox` | なし | 閉じたウィジェット |
| `element` | なし | 既存ページへ埋め込み |
| `document` | なし | `<title>` も制御 |
| `application` | あり | SPA（RingiFlow はこれ） |

SPA では URL の変更を検知する必要があるため、`application` が必須。

### 型シグネチャ

```elm
main : Program Flags Model Msg
```

「`Flags` を受け取って初期化し、`Model` を状態として持ち、`Msg` でやり取りする」プログラム。

## subscriptions

現在は `Sub.none`（何も購読しない）。

```elm
subscriptions _ = Sub.none
```

`subscriptions` は「外部世界からの入力口」。`update` がユーザー操作に反応するのに対し、`subscriptions` はユーザー操作なしに発生するイベントを受け取る。

将来使いそうなケース:

- リアルタイム通知（WebSocket → JS → Port）
- セッションタイムアウト警告（`Time.every`）
- ダッシュボード自動更新（`Time.every`）

`Ports.elm` に `receiveMessage` ポートが定義済みで、JS → Elm の通信準備はできている。

## Flags と init

`Flags` は JavaScript から Elm に渡される初期データ。

```elm
type alias Flags =
    { apiBaseUrl : String
    , timestamp : Int
    }
```

`init` は起動時に1回呼ばれ、`( Model, Cmd Msg )` を返す。「新しい状態」と「次にやるべき副作用」のペア — これが TEA の核心パターン。

```elm
init : Flags -> Url -> Nav.Key -> ( Model, Cmd Msg )
```

起動時に `Cmd.batch` で3つのコマンドを並行発行:

1. `pageCmd` — ページ初期化の API 呼び出し
2. `csrfCmd` — CSRF トークン取得
3. `userCmd` — ユーザー情報取得

## Cmd の並列と update の逐次

`Cmd.batch : List (Cmd msg) -> Cmd msg` — 複数の Cmd を1つにまとめる。

重要な区別:

| レベル | 並列？ | 理由 |
|--------|--------|------|
| Cmd の実行（HTTP リクエスト） | 並列 | Elm Runtime が並行発行 |
| update の実行（状態更新） | 逐次 | 1つずつ、前の結果を踏まえて |

Cmd の直列実行は `Cmd.andThen` のような API はない。代わりに `update` の中で次の `Cmd` を返すことで実現する。すべての中間状態が `update` を通るので、各ステップの状態が必ず Model に反映される。

RingiFlow の `init` では3つのリクエストに依存関係がなく、かつそれぞれ Session の別フィールドを更新するので、どの順で返っても最終状態は同じ。`Cmd.batch` で並列が正解。

## Model と Page 型（Nested TEA）

```elm
type Page
    = HomePage
    | WorkflowsPage WorkflowList.Model
    | WorkflowNewPage WorkflowNew.Model
    | WorkflowDetailPage WorkflowDetail.Model
    | NotFoundPage
```

Custom Type なので、常に1つのページだけがアクティブ。「一覧と詳細が同時にアクティブ」という不正な状態を型で排除。

各ページが自分の `Model / Msg / update / view` を持つミニ TEA アプリ。Main はそれらを束ねる役割。

## update のメッセージ振り分け

Msg は2種類:

- グローバル（`UrlChanged`, `GotCsrfToken`, `GotUser`）→ Main が直接処理
- ページ固有（`WorkflowsMsg`, `WorkflowDetailMsg`）→ ページの update に委譲

委譲パターンの5ステップ:

1. `WorkflowsMsg subMsg` — ラップされた Msg を受け取る
2. `case model.page of WorkflowsPage subModel` — 現在のページを確認
3. `WorkflowList.update subMsg subModel` — ページの update に委譲
4. `WorkflowsPage newSubModel` — 結果を Page 型に包み直す
5. `Cmd.map WorkflowsMsg subCmd` — Cmd の型も合わせる

今のページと違う Msg が来たら無視。遅れて届いた API レスポンスなどを安全に捨てる。

## Session の2つの意味

同じ「Session」がフロントとバックエンドで別の意味:

| | フロント (`Session.elm`) | バックエンド (`SessionManager`) |
|---|---|---|
| 保存先 | Elm のメモリ | Redis |
| 寿命 | タブ閉じたら消える | 8時間 TTL |
| 実態 | アプリのグローバル状態 | サーバー側セッション |

フロントの Session は Elm コミュニティの慣習的な命名。実態は「認証済みユーザーのコンテキスト」。バックエンドの Redis セッションのキャッシュと考えるとわかりやすい。

リロード時は Cookie（`session_id`）が残っているので、`/auth/me` を叩いてフロントの Session を復元できる。

### ページとの同期

各ページは `session` フィールドを持つ。Main の Session が更新されたら `updatePageSession` で全ページに伝播する。新しいページ追加時に `case` 分岐を書き忘れるとコンパイルエラーになるので安全。

## view とページ描画

`view` は `Model -> Browser.Document Msg` を返す。`case model.page of` で現在のページの `view` を呼ぶ。

各ページの `view` が返す `Html WorkflowList.Msg` を `Html.map WorkflowsMsg` で `Html Msg`（Main の Msg）に変換する。`update` での `Cmd.map` と対になる処理。

### `Html msg` の型パラメータ

- `msg`（小文字）— 型変数。「この HTML から発生し得るメッセージの型」
- `Msg`（大文字）— 各モジュールで定義された具体的な型

全モジュールが `Msg` という名前なのは Elm コミュニティの慣習。モジュールが名前空間の役割を果たすので、外部からは `WorkflowList.Msg` のように参照する。

### Nested TEA の map ペア

| 方向 | 関数 | 変換 |
|------|------|------|
| Cmd | `Cmd.map WorkflowsMsg` | `Cmd WorkflowList.Msg → Cmd Msg` |
| Html | `Html.map WorkflowsMsg` | `Html WorkflowList.Msg → Html Msg` |

同じ `WorkflowsMsg` コンストラクタで包むことで、Main はページの内部 Msg を知らなくても中継できる。

## Route.elm — ルーティング

### Route と Page の違い

| 型 | 表すもの | 持つデータ |
|-----|---------|----------|
| Route | URL（どこにいるか） | URL パラメータだけ |
| Page | 画面の状態 | API データ、フォーム入力等 |

分離することで URL パースと画面初期化が独立する。

### パーサーコンビネータ

小さなパーサーの組み合わせで URL を宣言的に定義:

- `top` — `/` にマッチ
- `s "xxx"` — 固定文字列にマッチ（値はキャプチャしない）
- `string` — 任意文字列にマッチ（値をキャプチャする）
- `</>` — パーサー同士を連結（`Parser </> Parser`。文字列は直接使えない）
- `oneOf` — 上から順に試す

順序が重要: 具体的なパス（`s "new"`）を先に、汎用的なパス（`string`）を後に。逆だと `"new"` が `string` に食べられる。

### 双方向変換

- `fromUrl : Url -> Route` — URL → Route。マッチしなければ `NotFound`
- `toString : Route -> String` — Route → URL 文字列。リンク生成に使う

この2つの一貫性が崩れると「リンクをクリックしたのに違うページ」になる。parser と toString の case 分岐は手動で一致させる必要があり、コンパイラでは検出できない。
