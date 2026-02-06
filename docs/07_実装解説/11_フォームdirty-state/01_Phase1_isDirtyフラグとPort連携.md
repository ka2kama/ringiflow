# Phase 1: isDirty フラグと Port 連携

## 概要

新規申請フォームに `isDirty` フラグを追加し、入力中のタブ閉じ・リロードに対してブラウザの警告ダイアログを表示する機能を実装した。

### 対応 Issue

[#177 フォーム dirty-state 検出による未保存データ損失防止](https://github.com/ka2kama/ringiflow/issues/177)

### 設計書との対応

- [詳細設計書: エンティティ影響マップ](../../03_詳細設計書/エンティティ影響マップ/) — 今回は新規エンティティの追加なし

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`frontend/src/Ports.elm`](../../../frontend/src/Ports.elm) | `setBeforeUnloadEnabled` ポートの定義 |
| [`frontend/src/main.js`](../../../frontend/src/main.js) | beforeunload リスナーの登録/解除 |
| [`frontend/src/Page/Workflow/New.elm`](../../../frontend/src/Page/Workflow/New.elm) | isDirty フラグ管理、dirty 検出ロジック |

## 実装内容

### Ports.elm

`setBeforeUnloadEnabled : Bool -> Cmd msg` を追加。Elm から JavaScript へ beforeunload の有効/無効を通知する。

### main.js

ポートのサブスクライブで `beforeunload` イベントリスナーを管理する。`enabled = true` でリスナー登録、`false` で解除。リスナーの二重登録を防ぐため、ハンドラ変数で状態を追跡する。

### New.elm: isDirty フラグ

Model に `isDirty_ : Bool` フィールドを追加し、`isDirty : Model -> Bool` 関数を公開する。

dirty 管理は `markDirty`/`clearDirty` ヘルパーで行う:

| Msg | 操作 | 理由 |
|-----|------|------|
| `SelectDefinition` | markDirty | 定義選択は入力行為 |
| `UpdateTitle` | markDirty | タイトル入力 |
| `UpdateField` | markDirty | フィールド入力 |
| `SelectApprover` | markDirty | 承認者選択 |
| `ClearApprover` | markDirty | 承認者クリア |
| `handleApproverKeyDown` Enter | markDirty | キーボード操作での承認者選択 |
| `GotSaveResult Ok` | clearDirty | 下書き保存成功 |
| `GotSaveAndSubmitResult Ok` | clearDirty | 保存+申請成功 |
| `GotSubmitResult Ok` | clearDirty | 申請成功 |
| `UpdateApproverSearch` | 変更なし | 検索テキストはフォーム値ではない |

## テスト

フロントエンドのみの変更のため、Elm テスト 152 件の通過で確認。dirty 管理のユニットテストは、`init` が `Shared` を要求するため既存テストパターンでは困難。手動テストで検証する。

## 関連ドキュメント

- [ナレッジベース: Elm ポート](../../06_ナレッジベース/elm/Elmポート.md)

---

## 設計解説

### 1. 専用ポート vs 汎用メッセージポート

場所: [`frontend/src/Ports.elm:52`](../../../frontend/src/Ports.elm)

```elm
port setBeforeUnloadEnabled : Bool -> Cmd msg
```

なぜこの設計か:

既存の汎用 `sendMessage : Encode.Value -> Cmd msg` ポートに `{ type: "SET_BEFORE_UNLOAD", payload: true }` のような JSON を渡す設計も可能だったが、専用ポートを選択した。

- **型安全性**: `Bool -> Cmd msg` により、呼び出し側で JSON エンコードミスが起こりえない
- **責務の分離**: beforeunload 制御はメッセージ通信とは本質的に異なる責務。汎用ポートに混ぜると JavaScript 側の dispatch ロジックが複雑になる
- **Elm のポート集約方針**: プロジェクトのナレッジベース（Elm ポート.md）でも、論理的に独立した通信は専用ポートにすることを推奨している

代替案:

- `sendMessage` に統合: JavaScript 側で `switch(data.type)` が必要。型安全性が失われる
- `port` ではなく `Cmd` で HTTP API 呼び出し: beforeunload はブラウザ API なので不適切

### 2. markDirty/clearDirty の状態遷移ガード

場所: [`frontend/src/Page/Workflow/New.elm:179-202`](../../../frontend/src/Page/Workflow/New.elm)

```elm
markDirty : Model -> ( Model, Cmd Msg )
markDirty model =
    if model.isDirty_ then
        ( model, Cmd.none )
    else
        ( { model | isDirty_ = True }
        , Ports.setBeforeUnloadEnabled True
        )
```

なぜこの設計か:

`markDirty` は `isDirty_ = False` のときのみ Port Cmd を発行し、すでに `True` なら何もしない。これにより:

- **冗長な JS 通信の排除**: フォームへの入力のたびに毎回 Port を呼ぶ無駄を防ぐ
- **呼び出し側の簡潔さ**: 各 Msg ハンドラで状態を確認する必要がなく、単に `markDirty model` を呼ぶだけ
- **Cmd の合成**: `markDirty` が返す Cmd を他の Cmd と `Cmd.batch` で合成する使い方に対応

代替案:

- 毎回 Port を呼ぶ: シンプルだが冗長。JS 側でも idempotent にする必要がある
- `update` 関数の末尾で一括判定: コードが集中するが、各ハンドラの意図が不明確になる

### 3. フィールド名 `isDirty_`（アンダースコアサフィックス）

場所: [`frontend/src/Page/Workflow/New.elm:95`](../../../frontend/src/Page/Workflow/New.elm)

```elm
type alias Model =
    { ...
    , isDirty_ : Bool
    }
```

なぜこの設計か:

Elm ではレコードフィールド名がアクセサ関数（`.isDirty`）として自動生成される。公開関数 `isDirty : Model -> Bool` と名前が衝突するため、フィールド名にアンダースコアサフィックスを付けた。

- **公開 API の優先**: モジュール外から使う `isDirty` 関数をクリーンな名前に保つ
- **内部実装の隠蔽**: アンダースコアは「内部用」の慣習的シグナル。直接フィールドアクセスを抑制する意図

代替案:

- フィールド名を `dirty` にする: 関数名と衝突しないが、`model.dirty` は `model.isDirty_` より意味が曖昧
- 公開関数名を `getIsDirty` にする: Java 的な命名で Elm のイディオムに合わない
