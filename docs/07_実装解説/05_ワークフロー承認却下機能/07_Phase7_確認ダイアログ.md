# Phase 7: 確認ダイアログ

## 概要

承認/却下ボタンクリック時に確認ダイアログを表示し、誤操作を防止する。

対応 Issue: [#176](https://github.com/ka2kama/ringiflow/issues/176)

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`Component/ConfirmDialog.elm`](../../../frontend/src/Component/ConfirmDialog.elm) | ステートレスな確認ダイアログ view コンポーネント |
| [`Util/KeyEvent.elm`](../../../frontend/src/Util/KeyEvent.elm) | ESC キーデコーダー |
| [`Page/Task/Detail.elm`](../../../frontend/src/Page/Task/Detail.elm) | タスク詳細ページ（確認フロー統合） |
| [`Page/Workflow/Detail.elm`](../../../frontend/src/Page/Workflow/Detail.elm) | 申請詳細ページ（確認フロー統合） |
| [`Main.elm`](../../../frontend/src/Main.elm) | subscriptions ルーティング |

## 実装内容

### メッセージフローの変更

```
変更前: ボタン → ClickApprove → 即 API 呼び出し
変更後: ボタン → ClickApprove → ダイアログ表示 → ConfirmAction → API 呼び出し
```

### 型定義

各ページに `PendingAction` 型をローカル定義:

```elm
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
```

Model に `pendingAction : Maybe PendingAction` を追加。
`Nothing` はダイアログ非表示、`Just` はダイアログ表示中を表す。

### ConfirmDialog コンポーネント

```elm
type ActionStyle = Positive | Destructive

view :
    { title : String
    , message : String
    , confirmLabel : String
    , cancelLabel : String
    , onConfirm : msg
    , onCancel : msg
    , actionStyle : ActionStyle
    }
    -> Html msg
```

- `Positive`: 承認 → `success-600` 系ボタン
- `Destructive`: 却下 → `error-600` 系ボタン
- オーバーレイクリックで `onCancel` を発行

### ESC キー対応

`Browser.Events.onKeyDown` で subscription 方式を採用:

```elm
subscriptions : Model -> Sub Msg
subscriptions model =
    case model.pendingAction of
        Just _ ->
            Browser.Events.onKeyDown (KeyEvent.escKeyDecoder CancelAction)

        Nothing ->
            Sub.none
```

## テスト

### 自動テスト

| テストケース | ファイル |
|-------------|---------|
| ESC キーで指定メッセージが返る | [`tests/Util/KeyEventTest.elm`](../../../frontend/tests/Util/KeyEventTest.elm) |
| ESC 以外のキーでは fail する | 同上 |

```bash
cd frontend && pnpm run test -- tests/Util/KeyEventTest.elm
```

### 手動テスト

- 承認ボタン → ダイアログ表示 → 「承認する」→ API 呼び出し成功
- 却下ボタン → ダイアログ表示 → 「却下する」→ API 呼び出し成功
- ダイアログ表示 → 「キャンセル」→ ダイアログ閉じる
- ダイアログ表示 → ESC キー → ダイアログ閉じる
- ダイアログ表示 → オーバーレイクリック → ダイアログ閉じる

## 関連ドキュメント

- [Phase 6: フロントエンド](06_全体フロー.md) — 承認/却下 UI の初期実装
- [Elm アーキテクチャ](../../../docs/06_ナレッジベース/elm/Elmアーキテクチャ.md) — TEA パターン

---

## 設計解説

### 1. ステートレス view コンポーネントパターン

場所: [`Component/ConfirmDialog.elm`](../../../frontend/src/Component/ConfirmDialog.elm)

```elm
view :
    { title : String, ..., onConfirm : msg, onCancel : msg, actionStyle : ActionStyle }
    -> Html msg
```

なぜこの設計か:
- `MessageAlert` と同じパターンで一貫性がある
- 型変数 `msg` により、コンポーネント自体はメッセージ型を定義しない
- 状態管理は親ページが担う（`Maybe PendingAction`）。ダイアログ自体がモデルを持つ必要がない

代替案:
- ダイアログ内にモデルを持つ stateful コンポーネント → 親との状態同期が複雑になる。Elm ではステートレスが推奨
- `Html.map` で包む child component パターン → 単純なモーダルには過剰

### 2. PendingAction のページローカル定義

場所: [`Page/Task/Detail.elm`](../../../frontend/src/Page/Task/Detail.elm)、[`Page/Workflow/Detail.elm`](../../../frontend/src/Page/Workflow/Detail.elm)

```elm
-- 各ページに同一の型を個別定義
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
```

なぜこの設計か:
- 各ページの承認フローは微妙に異なる（Task: コメントあり、Workflow: コメントなし）
- 共有すると、全ページの差異を吸収する汎用型が必要になり、過度な抽象化を招く
- 「3回繰り返すまでは重複を許容」の原則に合致（現在は 2 ページのみ）

代替案:
- 共有モジュールに定義 → 現時点では 2 ページのみで、将来の拡張時に検討すれば十分

### 3. 条件付き subscription

場所: [`Page/Task/Detail.elm`](../../../frontend/src/Page/Task/Detail.elm)（subscriptions 関数）

```elm
subscriptions model =
    case model.pendingAction of
        Just _ ->
            Browser.Events.onKeyDown (KeyEvent.escKeyDecoder CancelAction)

        Nothing ->
            Sub.none
```

なぜこの設計か:
- Elm の subscription は宣言的。現在の状態に応じて「あるべき購読」を返す
- ダイアログ非表示時に ESC キーを購読する必要がない
- グローバルにキャッチでき、フォーカス管理が不要

代替案:
- `Html.Events.on "keydown"` を HTML 属性で指定 → ダイアログにフォーカスが必要。フォーカス管理は Elm では副作用（Cmd）が必要で複雑になる
- 常に購読して `update` 内で `pendingAction` を確認 → 不要なメッセージが発行される
