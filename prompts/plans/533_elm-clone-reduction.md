# #533 Elm フロントエンドのクローン削減・ファイルサイズ削減

## Context

Epic #467「`just check` / `check-all` の警告をゼロにする」の Story 9。
Elm フロントエンドに jscpd コードクローンが 16 件（重複率 5.25%）、ファイルサイズ超過が 4 件ある。
クローンは Role/User ページ間のフォーム UI・バリデーション・Dirty フラグ管理の重複が中心。
ファイルサイズ超過はクローン対象とは別のファイル群（Workflow/Detail, Main, Workflow/New, Task/Detail）。

## スコープ

### 対象

1. **コードクローン 16 件の削減**（主目標）
   - User Edit/New フォーム重複（5 クローン）
   - User Detail/List 共通パターン（2 クローン）
   - Role Edit/New フォーム重複（4 クローン）
   - Role↔User 横断的重複（5 クローン）
2. **Main.elm のアイコン抽出によるファイルサイズ削減**（容易かつ効果的）

### 対象外

| ファイル | 行数 | 対象外の理由 |
|---------|------|-------------|
| Workflow/Detail.elm | 1367 | TEA ページ内のサブモジュール分割が必要。クローンなし。アーキテクチャ判断が必要なため別 Story とする |
| Workflow/New.elm | 1046 | 単一ページ内で責務が完結。クローンなし。分割の効果が限定的 |
| Task/Detail.elm | 609 | 超過が 109 行のみ。クローンなし。分割の効果が低い |

対象外のファイルは Issue #533 のチェックリストから除外し、必要に応じて別 Story を作成する。

## 設計判断

### DJ-1: 共有フォームフィールドコンポーネントの設計

**選択肢:**
- A) `Component.FormField` に全フィールド種別（TextField, TextArea, SelectField, ReadOnlyField）を集約
- B) `Component.TextField`, `Component.SelectField` 等を個別モジュールで作成
- C) 既存の `Component.Button` と同じレベルで個別ファイル

**選択: A（`Component.FormField` に集約）**

理由:
- フォームフィールドは共通の設計意図（ラベル + 入力 + エラー表示）を持つ
- エラー表示の Tailwind クラスが全フィールド種別で同一（`border-error-300` 等）→ 内部で共有可能
- 4 種別を個別ファイルにすると、過度な分割で認知負荷が増える
- 既存の `Component.PermissionMatrix` も複数関数を 1 ファイルに集約するパターン

### DJ-2: Dirty フラグ管理の共有方法

**選択肢:**
- A) Elm の extensible record 型を活用: `markDirty : { a | isDirty_ : Bool } -> ( { a | isDirty_ : Bool }, Cmd msg )`
- B) 値だけ返す関数: `markDirty : Bool -> ( Bool, Cmd msg )`（呼び出し側でモデル更新）
- C) Dirty 状態を別の型に切り出す: `type alias DirtyState = { isDirty : Bool }`

**選択: A（extensible record）**

理由:
- Elm の extensible record は「特定フィールドを持つレコード」を表現できる
- 呼び出し側の変更が最小限（`markDirty model` → `DirtyState.markDirty model`）
- 型安全にモデルの `isDirty_` フィールドの存在を保証
- B は呼び出し側の boilerplate が増える、C は全ページのモデル構造の変更が必要

### DJ-3: バリデーション関数の共有方法

**選択肢:**
- A) `Form.Validation` にパラメータ化した汎用バリデータを追加
- B) 別モジュール `Form.Validation.Common` を作成
- C) 各ページのバリデータはそのまま残し、共通化しない

**選択: A（既存 `Form.Validation` に追加）**

理由:
- 既存の `Form.Validation` は動的フォーム専用だが、バリデーションの責務としては同じモジュールが適切
- 新モジュールを作るほどの量ではない（関数 2-3 個の追加）
- `validateRequiredString` は動的フォームのバリデーションとも設計思想が一致

### DJ-4: statusToBadge の配置先

**選択: `Data.AdminUser` モジュールに移動**

理由:
- `statusToBadge` はユーザーのステータス値をドメイン知識に基づいて変換するロジック
- `Data.AdminUser` がすでに `AdminUserItem`, `UserDetail` 等のユーザーデータ型を定義
- User Detail/List 両方から import する共通の場所として最適

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: 共有フォームフィールドコンポーネント

`Component.FormField` を作成し、4 ページ（User Edit/New, Role Edit/New）のフォームフィールド view 関数を置き換える。

#### 確認事項
- [x] 型: 既存 Component の公開パターン → `Component/Button.elm`: record config 引数、テスト用にヘルパー関数公開（`variantClass`）
- [x] パターン: 既存 Component のテスト → `ButtonTest.elm`: 純粋関数テスト。elm-html-test 未導入のため CSS クラス計算関数を公開してテスト
- [x] ライブラリ: `Html.Attributes.attribute` → プロジェクト内未使用、不要

#### テストリスト

ユニットテスト:
- [ ] viewTextField: ラベル・入力値・プレースホルダーが正しく描画される
- [ ] viewTextField: エラーがある場合、エラーメッセージとエラースタイルが描画される
- [ ] viewTextField: エラーがない場合、通常スタイルが描画される
- [ ] viewTextField: inputType が指定どおりに設定される
- [ ] viewTextArea: ラベル・入力値・プレースホルダーが正しく描画される
- [ ] viewSelectField: ラベル・選択肢・選択値が正しく描画される
- [ ] viewSelectField: エラーがある場合、エラーメッセージが描画される
- [ ] viewSelectField: デフォルトオプション（プレースホルダー）が描画される
- [ ] viewReadOnlyField: ラベルと値が読み取り専用スタイルで描画される

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 作成ファイル

`frontend/src/Component/FormField.elm`:

```elm
module Component.FormField exposing
    ( viewTextField
    , viewTextArea
    , viewSelectField
    , viewReadOnlyField
    )

viewTextField :
    { label : String
    , value : String
    , onInput : String -> msg
    , error : Maybe String
    , inputType : String
    , placeholder : String
    }
    -> Html msg

viewTextArea :
    { label : String
    , value : String
    , onInput : String -> msg
    , placeholder : String
    }
    -> Html msg

viewSelectField :
    { label : String
    , value : String
    , onInput : String -> msg
    , error : Maybe String
    , options : List { value : String, label : String }
    , placeholder : String
    }
    -> Html msg

viewReadOnlyField : String -> String -> Html msg
```

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `Page/User/Edit.elm` | viewTextField, viewReadOnlyField, viewRoleSelect を削除し Component.FormField を import |
| `Page/User/New.elm` | viewTextField, viewRoleSelect を削除し Component.FormField を import |
| `Page/Role/Edit.elm` | viewTextField, viewTextArea, viewReadOnlyField を削除し Component.FormField を import |
| `Page/Role/New.elm` | viewTextField, viewTextArea を削除し Component.FormField を import |

### Phase 2: Dirty フラグ管理の共有化

`Form.DirtyState` を作成し、4 ページの markDirty/clearDirty を置き換える。

#### 確認事項
- [x] 型: extensible record → Elm 0.19 で `{ a | isDirty_ : Bool }` のレコード更新はサポート済み
- [x] パターン: Ports.setBeforeUnloadEnabled → 対象4ページ + Workflow/New + Main.elm で使用。markDirty は4ページで同一パターン

#### テストリスト

ユニットテスト:
- [ ] markDirty: isDirty_ が False のモデルに対し True に更新し setBeforeUnloadEnabled True を返す
- [ ] markDirty: isDirty_ が既に True のモデルに対し何もしない（Cmd.none）
- [ ] clearDirty: isDirty_ が True のモデルに対し False に更新し setBeforeUnloadEnabled False を返す
- [ ] clearDirty: isDirty_ が既に False のモデルに対し何もしない（Cmd.none）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 作成ファイル

`frontend/src/Form/DirtyState.elm`:

```elm
module Form.DirtyState exposing (clearDirty, isDirty, markDirty)

import Ports

isDirty : { a | isDirty_ : Bool } -> Bool
isDirty model =
    model.isDirty_

markDirty : { a | isDirty_ : Bool } -> ( { a | isDirty_ : Bool }, Cmd msg )
markDirty model =
    if model.isDirty_ then
        ( model, Cmd.none )
    else
        ( { model | isDirty_ = True }
        , Ports.setBeforeUnloadEnabled True
        )

clearDirty : { a | isDirty_ : Bool } -> ( { a | isDirty_ : Bool }, Cmd msg )
clearDirty model =
    if model.isDirty_ then
        ( { model | isDirty_ = False }
        , Ports.setBeforeUnloadEnabled False
        )
    else
        ( model, Cmd.none )
```

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `Page/User/Edit.elm` | markDirty を削除し Form.DirtyState を import。isDirty を DirtyState.isDirty に委譲 |
| `Page/User/New.elm` | markDirty, clearDirty を削除し Form.DirtyState を import |
| `Page/Role/Edit.elm` | markDirty を削除し Form.DirtyState を import |
| `Page/Role/New.elm` | markDirty を削除し Form.DirtyState を import |

### Phase 3: バリデーション共通化と statusToBadge 移動

`Form.Validation` に汎用バリデータを追加し、`Data.AdminUser` に `statusToBadge` を移動する。

#### 確認事項
- [x] 型: `Data.AdminUser` の exposing → 型 4 つ + デコーダー 5 つ。statusToBadge + BadgeConfig を追加
- [x] パターン: `Form.Validation.validateTitle` → `String -> ValidationResult` 型。validateRequiredString は Dict パイプライン型で設計

#### テストリスト

ユニットテスト:
- [ ] validateRequiredString: 空文字列でエラーを Dict に挿入する
- [ ] validateRequiredString: maxLength 超過でエラーを Dict に挿入する
- [ ] validateRequiredString: 正常値で errors をそのまま返す
- [ ] validateRequiredString: 空白のみの文字列でエラーを挿入する（trim 動作）
- [ ] statusToBadge: "active" で成功色とラベルを返す
- [ ] statusToBadge: "inactive" でセカンダリ色とラベルを返す
- [ ] statusToBadge: 未知の値でフォールバックを返す

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `Form/Validation.elm` | `validateRequiredString` を追加 |
| `Data/AdminUser.elm` | `statusToBadge` を追加（+ BadgeConfig 型） |
| `Page/User/Edit.elm` | validateName を Form.Validation.validateRequiredString で置き換え |
| `Page/User/New.elm` | validateName を Form.Validation.validateRequiredString で置き換え |
| `Page/Role/Edit.elm` | validateName を Form.Validation.validateRequiredString で置き換え |
| `Page/Role/New.elm` | validateName を Form.Validation.validateRequiredString で置き換え |
| `Page/User/Detail.elm` | statusToBadge を削除し Data.AdminUser.statusToBadge を import |
| `Page/User/List.elm` | statusToBadge を削除し Data.AdminUser.statusToBadge を import |

`Form.Validation` に追加する関数:

```elm
validateRequiredString :
    { fieldKey : String
    , fieldLabel : String
    , maxLength : Int
    }
    -> String
    -> Dict String String
    -> Dict String String
validateRequiredString config value errors =
    let
        trimmed = String.trim value
    in
    if String.isEmpty trimmed then
        Dict.insert config.fieldKey
            (config.fieldLabel ++ "を入力してください。")
            errors
    else if String.length trimmed > config.maxLength then
        Dict.insert config.fieldKey
            (config.fieldLabel ++ "は" ++ String.fromInt config.maxLength ++ "文字以内で入力してください。")
            errors
    else
        errors
```

### Phase 4: Main.elm アイコンモジュール抽出

SVG アイコン定義を `Component.Icons` に抽出し、Main.elm のファイルサイズを削減する。

#### 確認事項
- [x] パターン: Main.elm のアイコン → L1018-1140 の ICONS セクション。viewSidebar/viewAdminSection/viewTopBar から参照。7アイコン（dashboard, workflows, tasks, users, roles, auditLog, menu）

#### テストリスト

ユニットテスト（該当なし: 純粋な移動のみ、ロジックなし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 作成ファイル

`frontend/src/Component/Icons.elm`:

```elm
module Component.Icons exposing
    ( auditLog
    , dashboard
    , menu
    , roles
    , tasks
    , users
    , workflows
    )
```

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `Main.elm` | ICONS セクション（~122行）を削除し Component.Icons を import |

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | viewRoleSelect の汎用化: User ページでは `List RoleItem` を受け取るが、汎用 SelectField は `List { value, label }` を受け取るべき | 既存手段の見落とし | viewSelectField の options を `List { value : String, label : String }` に統一。呼び出し側で `List.map (\r -> { value = r.id, label = r.name }) roles` と変換する |
| 2回目 | Role Edit/New の viewTextField は `inputType` フィールドを持たない（"text" 固定）が、User Edit/New は持つ | 未定義 | Component.FormField.viewTextField は `inputType` を必須パラメータとし、Role ページは `inputType = "text"` を明示する |
| 3回目 | Role の validateName と User の validateName でエラーメッセージが異なる（「ロール名」vs「名前」）| 曖昧 | validateRequiredString のパラメータに fieldLabel を含め、メッセージを動的に生成する |
| 4回目 | Workflow/Detail, Workflow/New, Task/Detail のファイルサイズ削減が対象外になることの妥当性 | スコープ境界 | 3ファイルはクローンがなく、TEA ページ内サブモジュール分割というアーキテクチャ判断が必要。Issue コメントで対象外の理由を記録し、必要に応じて別 Story を作成する |
| 5回目 | clearDirty は User/New.elm にのみ存在。User/Edit, Role/Edit, Role/New は成功時に直接 `isDirty_ = False` + Port 呼び出しを行っている | 不完全なパス | clearDirty を共有モジュールで提供し、成功時の dirty クリアパターンも統一する。ただし各ページの成功時ロジック（ナビゲーション先等）は異なるため、clearDirty の呼び出しタイミングは各ページに委ねる |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 16 クローン全てに対策がある | OK | Phase 1 で viewTextField/viewTextArea/viewReadOnlyField/viewSelectField の重複を解消（クローン種別 B, C, F）。Phase 2 で markDirty/clearDirty の重複を解消（種別 A）。Phase 3 で validateName と statusToBadge を解消（種別 D, G）。残る種別 E（Submit パターン）と H（RemoteData パターン）はエンティティ固有部分が多く、抽象化の効果が限定的。共通部分（viewTextField 等）の抽出により、jscpd のトークン類似度が閾値を下回り、検出数が大幅に減る見込み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の作成/変更ファイル、関数シグネチャを具体的に記載。viewSelectField の options 型も明確に定義 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | DJ-1〜DJ-4 で主要な設計判断をカバー。inputType の統一、エラーメッセージのパラメータ化も記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（クローン 16 件 + Main.elm アイコン）と対象外（Workflow/Detail, Workflow/New, Task/Detail）を明示。対象外の理由を記載 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Elm の extensible record 型制約が markDirty/clearDirty で正しく動作すること（Phase 2 確認事項で検証予定）。Elm の型推論が `{ a | isDirty_ : Bool }` を正しく解決すること |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 既存の Component パターン（Button.elm）、Form パターン（Validation.elm）と整合。ADR の確認不要（既存アーキテクチャの範囲内）|

## 検証方法

1. `just check`（lint + test）で全テスト通過を確認
2. jscpd を実行しクローン数の減少を確認: `pnpm exec jscpd --min-lines 10 --min-tokens 50 --format "haskell" --formats-exts "haskell:elm" --gitignore --exitCode 0 frontend/src/`
3. `just check-all` で API テスト + E2E テストを含む全チェック通過を確認
4. 開発サーバー（`just dev-all`）でユーザー管理・ロール管理の新規作成/編集画面が正常に動作することを手動確認
