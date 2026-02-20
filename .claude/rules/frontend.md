---
paths:
  - "frontend/**"
  - "**/elm.json"
  - "**/package.json"
---

# フロントエンド実装ルール

このルールはフロントエンド（`frontend/`）のファイルを編集する際に適用される。

## 依存関係

依存関係を追加する際は、最新の stable バージョンを使用する。

### npm パッケージ

```bash
# pnpm で追加（自動的に最新バージョン）
pnpm add <package>
pnpm add -D <package>  # devDependencies

# または npm view で確認して手動追加
npm view <package> version
```

### Elm パッケージ

```bash
# elm install で追加
elm install <author/package>
```

## 推奨パッケージ（Elm）

以下のパッケージはプロジェクトに追加済み。ボイラープレートコードを避けるため、積極的に活用する。

### krisajenkins/remotedata

API レスポンスの状態管理。自作実装は避け、このパッケージを使用する。

```elm
import RemoteData exposing (RemoteData(..))

-- API レスポンスの型
type alias Model =
    { users : RemoteData Http.Error (List User)
    }

-- 使用例
case model.users of
    NotAsked ->
        text "まだ読み込んでいません"

    Loading ->
        LoadingSpinner.view

    Failure err ->
        viewError err

    Success users ->
        viewUserList users
```

注意:
- `WebData a` は `RemoteData Http.Error a` の型エイリアス
- カスタムエラー型（`ApiError` など）を使う場合は `RemoteData ApiError a` を直接使用

### elm-community/list-extra

リスト操作の拡張。標準ライブラリで冗長になるパターンに活用する。

| 関数 | 用途 | 標準ライブラリでの代替 |
|------|------|----------------------|
| `List.Extra.find` | 最初にマッチする要素を取得 | `List.filter` + `List.head` |
| `List.Extra.findIndex` | インデックスを取得 | 手動で再帰 |
| `List.Extra.unique` | 重複排除 | `Set` 経由で変換 |
| `List.Extra.groupBy` | グループ化 | `Dict` を使った手動集計 |
| `List.Extra.updateAt` | 特定位置の要素を更新 | `List.indexedMap` |

```elm
import List.Extra

-- Good: list-extra で簡潔に
users
    |> List.Extra.find (\u -> u.id == targetId)

-- Bad: 標準ライブラリで冗長
users
    |> List.filter (\u -> u.id == targetId)
    |> List.head
```

### elm-community/maybe-extra

Maybe 操作の拡張。パイプラインで使いやすい関数を提供する。

| 関数 | 用途 | 標準ライブラリでの代替 |
|------|------|----------------------|
| `Maybe.Extra.unwrap` | デフォルト値 + 変換を同時に | `Maybe.map` + `Maybe.withDefault` |
| `Maybe.Extra.orElse` | フォールバック（Maybe を返す） | `case` 式 |
| `Maybe.Extra.isJust` | Just かどうか | `(/=) Nothing` |
| `Maybe.Extra.join` | `Maybe (Maybe a)` を `Maybe a` に | `Maybe.andThen identity` |

```elm
import Maybe.Extra

-- Good: unwrap でシンプルに
user.role
    |> Maybe.Extra.unwrap "未設定" .name

-- Bad: map + withDefault で冗長
user.role
    |> Maybe.map .name
    |> Maybe.withDefault "未設定"
```

### 使用しない場面

以下の場合は標準ライブラリを維持する:

- 単純な `Maybe.withDefault` で済む場合（変換が不要）
- `List.map`, `List.filter` など標準で十分表現できる場合
- パッケージの関数が逆に可読性を下げる場合

## API レスポンスデコーダの命名規約

BFF の API は `{ "data": ... }` 形式でレスポンスを返す。`"data"` ラッパーのデコード責務は **Data モジュール側**に配置する。

### 命名規則

| デコーダ | 責務 | 配置場所 |
|---------|------|---------|
| `decoder` | 内部デコーダ（`"data"` ラッパーなし） | `Data.*` モジュール |
| `detailDecoder` | 単一レスポンス用（`Decode.field "data" decoder`） | `Data.*` モジュール |
| `listDecoder` | 一覧レスポンス用（`Decode.field "data" (Decode.list decoder)`） | `Data.*` モジュール |

```elm
-- Data/WorkflowDefinition.elm（正しいパターン）
decoder : Decoder WorkflowDefinition
decoder =
    Decode.succeed WorkflowDefinition
        |> required "id" Decode.string
        ...

detailDecoder : Decoder WorkflowDefinition
detailDecoder =
    Decode.field "data" decoder

listDecoder : Decoder (List WorkflowDefinition)
listDecoder =
    Decode.field "data" (Decode.list decoder)
```

```elm
-- Api/WorkflowDefinition.elm（正しい使い方）
getDefinition { config, id, toMsg } =
    Api.get
        { config = config
        , url = "/api/v1/workflow-definitions/" ++ id
        , decoder = WorkflowDefinition.detailDecoder  -- Data モジュールのデコーダを使用
        , toMsg = toMsg
        }
```

### 禁止事項

Api モジュール内で `Decode.field "data"` をインラインで使用してはならない。

```elm
-- Bad: Api モジュールでインラインラッピング
Api.get { decoder = Decode.field "data" WorkflowInstance.decoder, ... }

-- Good: Data モジュールのレスポンスデコーダを使用
Api.get { decoder = WorkflowInstance.detailDecoder, ... }
```

改善の経緯: [散弾銃デバッグによるトラブルシューティング効率低下](../../process/improvements/2026-02/2026-02-20_0012_散弾銃デバッグによるトラブルシューティング効率低下.md)

## AI エージェントへの指示

1. 依存関係を追加する際は最新の stable バージョンを使用する
2. `pnpm add` または `elm install` で追加
3. 更新後は `pnpm install` でロックファイルを同期
4. API レスポンス状態には `krisajenkins/remotedata` を使用する
5. リスト操作が冗長になる場合は `elm-community/list-extra` を検討する
6. Maybe のチェーン処理には `elm-community/maybe-extra` を検討する
7. API レスポンスデコーダは Data モジュールの `detailDecoder` / `listDecoder` を使用する。Api モジュールで `Decode.field "data"` をインラインで書かない

禁止事項:
- RemoteData パターンの自作実装（`krisajenkins/remotedata` を使用する）
- Api モジュール内での `Decode.field "data"` インライン使用（Data モジュールの `detailDecoder` / `listDecoder` を使用する）

## 参照

- 最新プラクティス方針: [latest-practices.md](latest-practices.md)
