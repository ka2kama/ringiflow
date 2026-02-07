# #177 フォーム dirty-state 検出による未保存データ損失防止

## Context

新規申請フォーム (`Page/Workflow/New.elm`) で入力中にナビゲーションすると、入力データが警告なく失われる。2つのメカニズムで防止する:

1. **beforeunload（ブラウザレベル）**: タブ閉じ、リロード、外部遷移 → Ports + JS
2. **SPA 内ナビゲーション遷移阻止（Elm レベル）**: サイドバーリンク → Main.elm で ConfirmDialog 表示

## 設計判断

| # | 判断 | 採用 | 理由 |
|---|------|------|------|
| 1 | dirty 検出方法 | `isDirty : Bool` フラグ | Issue の実装案と一致。値比較は複雑すぎて YAGNI |
| 2 | Main.elm → ページの dirty 確認 | ページが `isDirty` 関数を公開 | 既存 Nested TEA パターンと一致 |
| 3 | Port 設計 | 専用 `setBeforeUnloadEnabled` ポート | 汎用 `sendMessage` と責務が異なる。型安全 |
| 4 | ConfirmDialog 配置 | Main.elm に `pendingNavigation` | ナビゲーション制御は Main の責務。`Nav.Key` は Main のみ保持 |

## スコープ

- 対象: `Page/Workflow/New.elm` のみ（Issue #177 の明示スコープ）
- 対象外: 他ページへの横展開、ブラウザの戻る/進むボタン（Elm の `Browser.application` ではこれらは `UrlChanged` のみを発火し `LinkClicked` を経由しないため、SPA 内の遷移阻止は不可。`beforeunload` でブラウザレベルはカバー）

## 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Page/Workflow/New.elm` | `isDirty` フィールド追加、`isDirty` 関数公開、dirty 管理、Port Cmd 発行 |
| `frontend/src/Ports.elm` | `setBeforeUnloadEnabled` ポート追加 |
| `frontend/src/main.js` | beforeunload リスナー管理 |
| `frontend/src/Main.elm` | `pendingNavigation` 追加、`LinkClicked` インターセプト、ConfirmDialog 統合、subscriptions 更新 |

## Phase 構成

### Phase 1: New.elm に isDirty フラグ + Port 連携

**対象:** `New.elm`, `Ports.elm`, `main.js`

**実装:**

1. `Ports.elm` に `setBeforeUnloadEnabled : Bool -> Cmd msg` ポート追加
2. `main.js` に beforeunload リスナー管理を追加
3. `New.elm` の Model に `isDirty : Bool` 追加（初期値 `False`）
4. `isDirty : Model -> Bool` 関数を公開（`exposing` に追加）
5. ヘルパー関数:

```elm
-- isDirty が False → True に変わるときのみ Port Cmd を発行
markDirty : Model -> ( Model, Cmd Msg )

-- isDirty が True → False に変わるときのみ Port Cmd を発行
clearDirty : Model -> ( Model, Cmd Msg )
```

6. `update` で dirty 管理:

| Msg | isDirty | 備考 |
|-----|---------|------|
| `SelectDefinition` | `markDirty` | 定義選択は入力行為 |
| `UpdateTitle` | `markDirty` | |
| `UpdateField` | `markDirty` | |
| `SelectApprover` | `markDirty` | |
| `ClearApprover` | `markDirty` | |
| `handleApproverKeyDown` Enter（選択時） | `markDirty` | `SelectApprover` Msg を経由しないため直接対応が必要 |
| `GotSaveResult Ok` | `clearDirty` | データ永続化完了 |
| `GotSaveAndSubmitResult Ok` | `clearDirty` | 保存成功（申請は続行中だがデータは永続化済み） |
| `GotSubmitResult Ok` | `clearDirty` | 申請完了 |
| `UpdateApproverSearch` | 変更なし | 検索テキストはフォーム値ではない |

**テスト:** `frontend/tests/Page/Workflow/NewTest.elm`

- `isDirty` 関数の基本動作（False/True の Model に対する結果）
- dirty 管理ロジック（update 呼び出し後の isDirty 状態確認）は、`init` が `Shared` を要求するため既存テストパターンでは困難。手動テストで検証。

### Phase 2: Main.elm での SPA 内ナビゲーション遷移阻止

**対象:** `Main.elm`

**実装:**

1. Model に `pendingNavigation : Maybe Url` 追加
2. Msg に `ConfirmNavigation | CancelNavigation | NoOp` 追加
3. `LinkClicked` ハンドラ修正:

```elm
Browser.Internal url ->
    if isCurrentPageDirty model then
        ( { model | pendingNavigation = Just url }
        , focusDialogCancel  -- アクセシビリティ: キャンセルボタンにフォーカス
        )
    else
        ( model, Nav.pushUrl model.key (Url.toString url) )
```

4. `isCurrentPageDirty` ヘルパー:

```elm
isCurrentPageDirty : Model -> Bool
isCurrentPageDirty model =
    case model.page of
        WorkflowNewPage subModel ->
            WorkflowNew.isDirty subModel
        _ ->
            False
```

5. `ConfirmNavigation` → `Nav.pushUrl` + `Ports.setBeforeUnloadEnabled False` + `pendingNavigation = Nothing`
6. `CancelNavigation` → `pendingNavigation = Nothing`
7. `view` に ConfirmDialog 描画を追加（`pendingNavigation` が `Just` のとき）
   - actionStyle: `ConfirmDialog.Destructive`
   - タイトル: 「ページを離れますか？」
   - メッセージ: 「入力中のデータは保存されません。このページを離れてもよろしいですか？」
8. `subscriptions` に ESC キーハンドラ追加（Main レベル、`pendingNavigation` が `Just` のとき）

**参照パターン:** `Page/Workflow/Detail.elm` の PendingAction + ConfirmDialog + ESC + focusDialogCancel

**テスト:** 手動テスト（Main.elm はアプリケーション統合層のためユニットテスト不適）

## 手動テスト計画

| # | シナリオ | 期待結果 |
|---|---------|---------|
| 1 | フォーム入力 → タブ閉じ | ブラウザの警告ダイアログ表示 |
| 2 | フォーム入力 → サイドバーリンククリック | 確認ダイアログ表示 |
| 3 | 確認ダイアログ → 「ページを離れる」 | ナビゲーション実行 |
| 4 | 確認ダイアログ → 「このページに留まる」 | ページに留まる |
| 5 | 確認ダイアログ → ESC キー | ダイアログ閉じ、ページに留まる |
| 6 | 下書き保存成功 → タブ閉じ | 警告なし |
| 7 | 下書き保存成功 → サイドバーリンククリック | 確認なしにナビゲーション |
| 8 | 下書き保存成功 → 再入力 → サイドバーリンククリック | 確認ダイアログ表示 |
| 9 | フォーム未入力 → サイドバーリンククリック | 確認なしにナビゲーション |
| 10 | 申請成功 → サイドバーリンククリック | 確認なしにナビゲーション |

## 検証方法

```bash
just check       # コンパイル + リント + テスト
just dev-all     # 開発サーバー起動 → 手動テスト実施
```

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | New.elm, Main.elm, Ports.elm, main.js の全変更対象を計画に含めた。dirty 設定対象の全 Msg を列挙し、`handleApproverKeyDown` Enter パスも漏れなく対応 |
| 2 | 曖昧さ排除 | OK | 各 Msg に対する isDirty 操作を表形式で明示。「必要に応じて」等の曖昧表現なし |
| 3 | 設計判断の完結性 | OK | dirty 検出方法、Main vs Page の責務、Port 設計、ConfirmDialog 配置の4判断を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象（New.elm のみ）と対象外（他ページ、ブラウザ戻る/進む）を明記 |
| 5 | 技術的前提 | OK | Browser.application の onUrlRequest/onUrlChange 挙動差異、beforeunload の SPA 内動作制約を確認 |
| 6 | 既存ドキュメント整合 | OK | ConfirmDialog の既存パターン（Detail.elm）、Ports 集約方針（Elmポート.md）と整合 |
