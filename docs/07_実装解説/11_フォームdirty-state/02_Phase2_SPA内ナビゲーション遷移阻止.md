# Phase 2: SPA 内ナビゲーション遷移阻止

## 概要

`Main.elm` でサイドバーリンクのクリック時に dirty チェックを行い、入力中のデータがある場合は ConfirmDialog で確認を求める機能を実装した。

### 対応 Issue

[#177 フォーム dirty-state 検出による未保存データ損失防止](https://github.com/ka2kama/ringiflow/issues/177)

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`frontend/src/Main.elm`](../../../frontend/src/Main.elm) | ナビゲーションインターセプト、ConfirmDialog 表示、ESC キーハンドリング |

## 実装内容

### Model の拡張

`pendingNavigation : Maybe Url` を追加。ConfirmDialog 表示中に遷移先 URL を保持する。

### Msg の追加

| Msg | 説明 |
|-----|------|
| `ConfirmNavigation` | ConfirmDialog で「ページを離れる」を選択 |
| `CancelNavigation` | ConfirmDialog で「このページに留まる」を選択 |
| `NoOp` | focusDialogCancel の Task.attempt コールバック |

### LinkClicked ハンドラの修正

`Browser.Internal url` の場合、`isCurrentPageDirty` をチェック:

- dirty → `pendingNavigation = Just url` + `focusDialogCancel`
- not dirty → `Nav.pushUrl`（従来通り）

### isCurrentPageDirty ヘルパー

```elm
isCurrentPageDirty : Model -> Bool
isCurrentPageDirty model =
    case model.page of
        WorkflowNewPage subModel ->
            WorkflowNew.isDirty subModel
        _ ->
            False
```

現在は `WorkflowNewPage` のみ対応。将来的に他のページにも `isDirty` を追加する場合はここを拡張する。

### ConfirmDialog

`Component.ConfirmDialog` を使用し、Destructive スタイルで表示:

- タイトル: 「ページを離れますか？」
- メッセージ: 「入力中のデータは保存されません。このページを離れてもよろしいですか？」
- 確認ボタン: 「ページを離れる」
- キャンセルボタン: 「このページに留まる」

### subscriptions

`pendingNavigation` が `Just` の場合、ESC キーで `CancelNavigation` を発火するサブスクリプションを追加。

## テスト

Main.elm はアプリケーション統合層のためユニットテスト不適。手動テストで検証する。

手動テストシナリオ:

| # | シナリオ | 期待結果 |
|---|---------|---------|
| 1 | フォーム入力 → サイドバーリンク | 確認ダイアログ表示 |
| 2 | 確認ダイアログ → 「ページを離れる」 | ナビゲーション実行 |
| 3 | 確認ダイアログ → 「このページに留まる」 | ページに留まる |
| 4 | 確認ダイアログ → ESC キー | ダイアログ閉じ、ページに留まる |
| 5 | フォーム未入力 → サイドバーリンク | 確認なしにナビゲーション |
| 6 | 下書き保存成功 → サイドバーリンク | 確認なしにナビゲーション |

## 関連ドキュメント

- [Phase 1: isDirty フラグと Port 連携](01_Phase1_isDirtyフラグとPort連携.md)
- [ナレッジベース: Elm アーキテクチャ](../../06_ナレッジベース/elm/Elmアーキテクチャ.md)

---

## 設計解説

### 1. ConfirmDialog の配置場所: Main.elm

場所: [`frontend/src/Main.elm:290-293`](../../../frontend/src/Main.elm)

```elm
Browser.Internal url ->
    if isCurrentPageDirty model then
        ( { model | pendingNavigation = Just url }
        , focusDialogCancel
        )
    else
        ( model, Nav.pushUrl model.key (Url.toString url) )
```

なぜこの設計か:

Elm の `Browser.application` では `Nav.Key` は `Main.elm` の `init` で受け取り、`Main.elm` のみが保持する。ナビゲーションの実行（`Nav.pushUrl`）は `Nav.Key` が必要なため、ナビゲーション制御は Main の責務。

- **責務の一貫性**: `LinkClicked` → dirty チェック → ConfirmDialog → `Nav.pushUrl` の全フローが Main 内で完結
- **ページの独立性**: 各ページは `isDirty` 関数を公開するだけでよく、ナビゲーション制御の知識を持たない

代替案:

- ページ側で ConfirmDialog を表示: `Nav.Key` をページに渡す必要があり、責務の漏洩。Elm の Nested TEA パターンでは、子が親の Key を持つのは一般的でない
- Custom Event + Port: JavaScript 側で `window.confirm()` を呼ぶ。UX が劣り、カスタマイズ不可

### 2. Detail.elm の PendingAction パターンの踏襲

場所: [`frontend/src/Main.elm:365-379`](../../../frontend/src/Main.elm)

```elm
ConfirmNavigation ->
    case model.pendingNavigation of
        Just url ->
            ( { model | pendingNavigation = Nothing }
            , Cmd.batch
                [ Nav.pushUrl model.key (Url.toString url)
                , Ports.setBeforeUnloadEnabled False
                ]
            )
        Nothing ->
            ( model, Cmd.none )

CancelNavigation ->
    ( { model | pendingNavigation = Nothing }
    , Cmd.none
    )
```

なぜこの設計か:

`Page/Workflow/Detail.elm` で既に確立されている PendingAction パターン（保留中アクション + ConfirmDialog + ESC + focusDialogCancel）をそのまま適用した。

- **一貫性**: 既存パターンに従うことで、コードベース全体の予測可能性が高まる
- **アクセシビリティ**: `focusDialogCancel` でキャンセルボタンにフォーカスを移す。誤操作（Enter で即確定）を防ぐ
- **ESC キーのサポート**: モーダルダイアログの標準的な UX。`Browser.Events.onKeyDown` で `pendingNavigation` が `Just` のときのみ ESC を購読

### 3. ConfirmNavigation 時の beforeunload 解除

場所: [`frontend/src/Main.elm:368-372`](../../../frontend/src/Main.elm)

```elm
Cmd.batch
    [ Nav.pushUrl model.key (Url.toString url)
    , Ports.setBeforeUnloadEnabled False
    ]
```

なぜこの設計か:

ナビゲーションを確定した時点で beforeunload リスナーを解除する。`Nav.pushUrl` はページ遷移を発生させるが、遷移先のページの `init` が実行されるまでの間にブラウザイベントが発火する可能性がある。明示的に解除することで、遷移後に不要な警告が表示されることを防ぐ。

代替案:

- 解除しない（遷移先の init で自然に解消される想定）: New.elm の `init` は `isDirty_ = False` で開始するため、beforeunload の明示的解除が行われない。遷移先が New.elm 以外の場合、JS 側のリスナーが残留するリスクがある
