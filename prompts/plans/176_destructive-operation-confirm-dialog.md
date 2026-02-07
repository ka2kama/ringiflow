# Issue #176: 破壊的操作の確認ダイアログ追加

## 概要

承認/却下ボタンクリック時に確認ダイアログを表示し、誤操作を防止する。

## 設計方針

### Component.ConfirmDialog — 状態なしの view コンポーネント

MessageAlert と同じパターン。レコード型パラメータ + 型変数 `msg` で各ページ対応。

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

- Positive: 承認 → `success-600` 系ボタン
- Destructive: 却下 → `error-600` 系ボタン
- オーバーレイ: `fixed inset-0 z-40 bg-black/50`（クリックでキャンセル）
- ダイアログ: `z-50`, `role="dialog"`, `aria-modal="true"`

### ページ側の状態管理

各ページの Model に `pendingAction : Maybe PendingAction` を追加:

```elm
type PendingAction
    = ConfirmApprove WorkflowStep
    | ConfirmReject WorkflowStep
```

PendingAction は各ページにローカル定義（共有モジュールにはしない）。

### メッセージフロー変更

```
変更前: ボタン → ClickApprove → 即 API 呼び出し
変更後: ボタン → ClickApprove → ダイアログ表示 → ConfirmAction → API 呼び出し
```

既存の `ClickApprove`/`ClickReject` は「ダイアログ表示」に意味変更。
新 Msg `ConfirmAction`/`CancelAction` を追加。
`approveStep`/`rejectStep` 等の既存 API 呼び出しロジックは一切変更しない。

### ESC キー対応

`Browser.Events.onKeyDown` (subscription) 方式を採用。

- `Util.KeyEvent.escKeyDecoder` を新設（共有ユーティリティ）
- 各ページで `subscriptions` 関数をエクスポート
- Main.elm の `subscriptions` で各ページにルーティング

理由: グローバルにキャッチでき、フォーカス管理が不要。Elm の副作用管理に合致。

## 変更ファイル一覧

| ファイル | 操作 |
|---------|------|
| `frontend/src/Component/ConfirmDialog.elm` | 新規 |
| `frontend/src/Util/KeyEvent.elm` | 新規 |
| `frontend/tests/Util/KeyEventTest.elm` | 新規 |
| `frontend/src/Page/Task/Detail.elm` | 変更 |
| `frontend/src/Page/Workflow/Detail.elm` | 変更 |
| `frontend/src/Main.elm` | 変更 |

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: 共有ユーティリティとコンポーネント

ファイル: `Util/KeyEvent.elm`, `Component/ConfirmDialog.elm`, テスト

テストリスト（Util.KeyEvent）:
- [ ] `escKeyDecoder`: "Escape" キーで指定のメッセージが返る
- [ ] `escKeyDecoder`: "Escape" 以外のキー（例: "Enter"）では fail する

ConfirmDialog の view は elm-test で DOM テスト不可のため、Phase 3 の手動テストで確認。

### Phase 2: Page/Task/Detail.elm に確認ダイアログを統合

変更内容:
1. `PendingAction` 型を定義
2. Model に `pendingAction : Maybe PendingAction` を追加
3. Msg に `ConfirmAction`, `CancelAction` を追加
4. `ClickApprove`/`ClickReject` → ダイアログ表示に変更
5. `ConfirmAction` → 既存 API 呼び出しを実行
6. `subscriptions` 関数を追加しエクスポート
7. view に `viewConfirmDialog` を追加

テストリスト:
- [ ] `just check-all` が通ること

### Phase 3: Page/Workflow/Detail.elm に確認ダイアログを統合

Phase 2 と同一パターン。コメント入力がない点のみ異なる。

テストリスト:
- [ ] `just check-all` が通ること

### Phase 4: Main.elm の subscriptions 接続

変更内容:
- `subscriptions` を `Sub.none` → 各ページの subscriptions にルーティング
- `import Browser.Events` の追加

テストリスト:
- [ ] `just check-all` が通ること

## 検証方法

`just check-all` 通過後、手動テストで以下を確認:

- [ ] タスク詳細: 承認ボタン → ダイアログ表示 → 「承認する」→ API 呼び出し成功
- [ ] タスク詳細: 却下ボタン → ダイアログ表示 → 「却下する」→ API 呼び出し成功
- [ ] タスク詳細: ダイアログ表示 → 「キャンセル」→ ダイアログ閉じる
- [ ] タスク詳細: ダイアログ表示 → ESC キー → ダイアログ閉じる
- [ ] タスク詳細: ダイアログ表示 → オーバーレイクリック → ダイアログ閉じる
- [ ] 申請詳細: 上記と同等の動作確認
- [ ] ダイアログ非表示時は ESC キーが無反応であること
