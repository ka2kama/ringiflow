# #805 FormField.elm に for/id 属性を追加してアクセシビリティを改善する

## Context

`Component.FormField.elm` の4つのビュー関数（viewTextField, viewTextArea, viewSelectField, viewReadOnlyField）に `<label for>` / `<input id>` の明示的関連付けがない。WCAG 2.1 AA 違反であり、Playwright の `getByLabel` が動作しない原因。

#780（E2E テスト）で表層対策（`getByPlaceholder` フォールバック）を実施済み。本 Issue は根本対策として、FormField コンポーネントに `for`/`id` を追加し、E2E テストのロケーターを `getByLabel` に戻す。

参照パターン: `Page/WorkflowDefinition/List.elm` の `viewFormField`/`viewFormTextarea`（L626-663）が正しく `for`/`id` を使用している。`Form/DynamicForm.elm` も同様に実装済み。

## 対象

- `frontend/src/Component/FormField.elm` — 4関数の API 変更
- `frontend/src/Page/User/New.elm` — 3箇所
- `frontend/src/Page/User/Edit.elm` — 3箇所
- `frontend/src/Page/Role/New.elm` — 2箇所
- `frontend/src/Page/Role/Edit.elm` — 4箇所（読み取り専用2 + 編集2）
- `frontend/src/Page/WorkflowDefinition/Designer.elm` — 3箇所
- `tests/e2e/tests/user-management.spec.ts` — ロケーター移行

## 対象外

- `Form/DynamicForm.elm` — 既に `for`/`id` 実装済み
- `Page/WorkflowDefinition/List.elm` — 既に `for`/`id` 実装済み
- `Page/Workflow/Detail.elm` — `Component.FormField` を使用していない

## Phase 1: FormField API 変更 + 呼び出し元更新 + E2E ロケーター移行

### 設計判断

fieldId は呼び出し元が明示的に指定する（auto-generate しない）。理由:
- `WorkflowDefinition/List.elm` の既存パターンと一致
- 呼び出し元がセマンティクスを制御できる
- ID の一意性が保証しやすい

### 確認事項

- 型: `Component.FormField.elm` の現在のシグネチャ → `frontend/src/Component/FormField.elm`
- パターン: `for`/`id` の既存使用パターン → `Page/WorkflowDefinition/List.elm` L626-663
- ライブラリ: `Html.Attributes.for`, `Html.Attributes.id` → Grep で既存使用確認

### 変更内容

#### 1. FormField.elm API 変更

**viewTextField**: config に `fieldId : String` 追加

```elm
viewTextField :
    { label : String
    , value : String
    , onInput : String -> msg
    , error : Maybe String
    , inputType : String
    , placeholder : String
    , fieldId : String          -- 追加
    }
    -> Html msg
viewTextField config =
    div []
        [ label [ for config.fieldId, class "..." ] [ text config.label ]
        , input
            [ id config.fieldId   -- 追加
            , type_ config.inputType
            , ...
            ]
            []
        , viewError config.error
        ]
```

**viewTextArea**: config に `fieldId : String` 追加（同様のパターン）

**viewSelectField**: config に `fieldId : String` 追加（同様のパターン）

**viewReadOnlyField**: 引数に `fieldId` 追加

```elm
viewReadOnlyField : String -> String -> String -> Html msg
viewReadOnlyField fieldId labelText fieldValue =
    div []
        [ label [ for fieldId, class "..." ] [ text labelText ]
        , div [ id fieldId, class "..." ] [ text fieldValue ]
        ]
```

注: `for` は labelable element（input/select/textarea）向けの属性だが、div に `id` を付与してもブラウザは壊れない。スクリーンリーダーの近接パターンで関連付けを認識できる。テスタビリティ（`getByLabel` で div テキストを取得）にも貢献する。

#### 2. 呼び出し元の更新（fieldId 値の一覧）

| ファイル | 関数 | fieldId |
|---------|------|---------|
| User/New.elm:311 | viewTextField | `"user-email"` |
| User/New.elm:319 | viewTextField | `"user-name"` |
| User/New.elm:327 | viewSelectField | `"user-role"` |
| User/Edit.elm:317 | viewReadOnlyField | `"user-email"` |
| User/Edit.elm:318 | viewTextField | `"user-name"` |
| User/Edit.elm:326 | viewSelectField | `"user-role"` |
| Role/New.elm:251 | viewTextField | `"role-name"` |
| Role/New.elm:259 | viewTextArea | `"role-description"` |
| Role/Edit.elm:313 | viewReadOnlyField | `"role-name"` |
| Role/Edit.elm:314 | viewReadOnlyField | `"role-description"` |
| Role/Edit.elm:346 | viewTextField | `"role-name"` |
| Role/Edit.elm:354 | viewTextArea | `"role-description"` |
| Designer.elm:1556 | viewTextField | `"step-name"` |
| Designer.elm:1589 | viewReadOnlyField | `"step-approver"` |
| Designer.elm:1592 | viewSelectField | `"step-end-status"` |

#### 3. E2E テストのロケーター移行

| 行 | 現在 | 変更後 |
|----|------|--------|
| L22 | `getByPlaceholder("user@example.com")` | `getByLabel("メールアドレス")` |
| L23 | `getByPlaceholder("山田 太郎")` | `getByLabel("名前")` |
| L24 | `locator("select")` | `getByLabel("ロール")` |
| L49 | `getByPlaceholder("user@example.com")` | `getByLabel("メールアドレス")` |
| L50 | `getByPlaceholder("山田 太郎")` | `getByLabel("名前")` |
| L51 | `locator("select")` | `getByLabel("ロール")` |
| L69 | `getByPlaceholder("山田 太郎")` | `getByLabel("名前")` |
| L70 | `getByPlaceholder("山田 太郎")` | `getByLabel("名前")` |
| L86 | `getByPlaceholder("user@example.com")` | `getByLabel("メールアドレス")` |
| L87 | `getByPlaceholder("山田 太郎")` | `getByLabel("名前")` |
| L88 | `locator("select")` | `getByLabel("ロール")` |

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ラベルをクリックして入力フィールドにフォーカスが移動する | 正常系 | E2E |
| 2 | ラベルで入力フィールドを特定してフォーム操作を完了する | 正常系 | E2E |

### テストリスト

ユニットテスト（該当なし — Elm の view 関数に対する DOM テストはプロジェクトに導入していない）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト:
- [ ] `getByLabel("メールアドレス")` でメール入力フィールドを特定できる（user-management.spec.ts 既存テストのロケーター変更で暗黙的に検証）
- [ ] `getByLabel("名前")` で名前入力フィールドを特定できる
- [ ] `getByLabel("ロール")` でロール選択フィールドを特定できる

注: 既存の E2E テスト 3 件のロケーターを `getByLabel` に変更することで、`for`/`id` 属性の動作を検証する。テストが通れば、ラベルとフィールドの関連付けが正しいことが証明される。

## 検証

1. `just fmt` — フォーマット
2. `just check` — Elm コンパイル + lint
3. `just check-all` — 全テスト（E2E 含む）で `getByLabel` ロケーターが通ることを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | viewReadOnlyField は div を表示するため `for`/`id` の HTML セマンティクスが不完全 | 技術的前提 | `for` は labelable element 向けだが、`id` の付与自体は有効。スクリーンリーダーの近接パターンとテスタビリティのために採用。注記を計画に追加 |
| 1回目 | `locator("select")` も `getByLabel("ロール")` に変更すべき | 操作パス網羅漏れ | E2E ロケーター表に select の変更を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | FormField.elm の4関数 + 全呼び出し元5ファイル + E2E テスト1ファイル。DynamicForm.elm/List.elm は既に実装済みで対象外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全 fieldId 値と全ロケーター変更を具体的に列挙済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | fieldId の命名方式（呼び出し元指定）、viewReadOnlyField の for/id セマンティクスについて判断記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象・対象外セクションに明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | HTML for 属性の仕様、viewReadOnlyField の div 要素への適用について確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | E2E テストルール（getByLabel 優先）、デザインガイドラインと整合 |
