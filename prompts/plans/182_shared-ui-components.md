# Issue #182: 共有UIコンポーネントの抽出

## モジュール構成

```
frontend/src/
├── Component/                       # 新規ディレクトリ
│   ├── LoadingSpinner.elm           # ローディングスピナー
│   └── MessageAlert.elm             # 成功/エラーメッセージアラート
├── Api/
│   └── ErrorMessage.elm             # 新規: ApiError → ユーザー向けメッセージ
├── Data/
│   └── WorkflowInstance.elm         # 既存: stepStatusToCssClass を追加
└── Util/                            # 新規ディレクトリ
    └── DateFormat.elm               # 日付フォーマットユーティリティ
```

命名根拠:
- `Component/` — 既存の `Data/`, `Api/`, `Form/`, `Page/` と同じ役割ベース命名。`View/` は Page の `view` 関数と紛らわしい
- `Api.ErrorMessage` — `ApiError` を扱うので `Api` 名前空間が自然
- `Util.DateFormat` — ドメインにもUIにも依存しない純粋ユーティリティ
- `Badge.elm` は不要 — `stepStatusToCssClass` は既存の `Data.WorkflowInstance` に追加する方が自然

## 公開API設計

### Component.LoadingSpinner
```elm
view : Html msg    -- 型変数 msg でどのページからも呼べる
```

### Component.MessageAlert
```elm
view :
    { onDismiss : msg
    , successMessage : Maybe String
    , errorMessage : Maybe String
    }
    -> Html msg
```
レコード引数で呼び出し側の意図を明確にし、将来の拡張にも対応。

### Api.ErrorMessage
```elm
toUserMessage : { entityName : String } -> ApiError -> String
```
エンティティ名（「ワークフロー」「タスク」）を引数でパラメータ化。

### Util.DateFormat
```elm
formatDate          : String -> String         -- "2026-01-15T..." → "2026-01-15"
formatMaybeDate     : Maybe String -> String   -- Nothing → "-"
formatDateTime      : String -> String         -- "2026-01-15T10:30:00Z" → "2026-01-15 10:30"
formatMaybeDateTime : Maybe String -> String   -- Nothing → "-"
```
基本形（`String -> String`）と Maybe ラッパーを分離し、単一責務に。

### Data.WorkflowInstance（追加）
```elm
stepStatusToCssClass : StepStatus -> String
```

## Phase 分割

### Phase 1: Util.DateFormat の抽出
純粋関数でテストが書きやすい。TDD で開始。

新規:
- `frontend/src/Util/DateFormat.elm`
- `frontend/tests/Util/DateFormatTest.elm`

変更:
- `frontend/src/Page/Workflow/List.elm` — `formatDate` → `Util.DateFormat.formatDate`
- `frontend/src/Page/Workflow/Detail.elm` — `formatDateTime` → `Util.DateFormat.formatMaybeDateTime`
- `frontend/src/Page/Task/List.elm` — `formatMaybeDate` → `Util.DateFormat.formatMaybeDate`
- `frontend/src/Page/Task/Detail.elm` — `formatDateTime` → `Util.DateFormat.formatMaybeDateTime`

### Phase 2: Api.ErrorMessage の抽出
純粋関数でテスト可能。エンティティ名のパラメータ化。

新規:
- `frontend/src/Api/ErrorMessage.elm`
- `frontend/tests/Api/ErrorMessageTest.elm`

変更:
- `frontend/src/Page/Workflow/Detail.elm` — `apiErrorToMessage error` → `Api.ErrorMessage.toUserMessage { entityName = "ワークフロー" } error`
- `frontend/src/Page/Task/Detail.elm` — 同上（entityName = "タスク"）

### Phase 3: stepStatusToCssClass の集約
既存モジュールへの追加。`statusToCssClass` のパターンを踏襲。

変更:
- `frontend/src/Data/WorkflowInstance.elm` — `stepStatusToCssClass` を追加、exposing 更新
- `frontend/tests/Data/WorkflowInstanceTest.elm` — テスト追加
- `frontend/src/Page/Task/List.elm` — ローカル関数を削除、`WorkflowInstance.stepStatusToCssClass` に置換
- `frontend/src/Page/Task/Detail.elm` — 同上

### Phase 4: Component.LoadingSpinner の抽出
7箇所のインラインHTML を置換。

新規:
- `frontend/src/Component/LoadingSpinner.elm`

変更（7箇所）:
- `frontend/src/Page/Home.elm`
- `frontend/src/Page/Workflow/List.elm`
- `frontend/src/Page/Workflow/Detail.elm`（2箇所: viewContent + viewFormData）
- `frontend/src/Page/Workflow/New.elm`
- `frontend/src/Page/Task/List.elm`
- `frontend/src/Page/Task/Detail.elm`

### Phase 5: Component.MessageAlert の抽出
Msg 型パラメータ化が必要なコンポーネント。

新規:
- `frontend/src/Component/MessageAlert.elm`

変更:
- `frontend/src/Page/Workflow/Detail.elm` — `viewMessages model` → `Component.MessageAlert.view { onDismiss = DismissMessage, ... }`
- `frontend/src/Page/Task/Detail.elm` — 同上

対象外: Workflow/New.elm の `viewSaveMessage` は `SaveMessage` 型を使う別パターン。無理に統合しない。

## テスト方針

| 対象 | テスト | 理由 |
|------|--------|------|
| Util.DateFormat | ✅ TDD | 純粋関数、境界値テスト可能 |
| Api.ErrorMessage | ✅ TDD | 純粋関数、全 ApiError バリアントをテスト |
| stepStatusToCssClass | ✅ テスト追加 | 全 StepStatus バリアントの CSS クラス出力 |
| LoadingSpinner | ❌ テスト不要 | HTML を返すだけ。コンパイル通過が検証 |
| MessageAlert | ❌ テスト不要 | 同上。型チェックで安全性担保 |

## 検証方法

各 Phase 完了後に:
```bash
just check-all    # lint + 全テスト
```

最終確認:
```bash
just dev-all      # 開発サーバー起動
# ブラウザで以下を確認:
# - 申請一覧/詳細/新規作成のローディング表示
# - 承認/却下後のメッセージ表示と×ボタン
# - 日付/日時の表示フォーマット
# - タスク一覧/詳細のステータスバッジ
```
