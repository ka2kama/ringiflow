# #613 状態表示パターンの共通化（Empty / Error state）

Issue: [#613](https://github.com/ka2kama/ringiflow/issues/613)
Epic: [#445](https://github.com/ka2kama/ringiflow/issues/445) Story 3

## コンテキスト

Error state（`rounded-lg bg-error-50 p-4 text-error-700`）が 17 箇所でページ直書きされており、Empty state も各ページで独自に実装されスタイルにばらつきがある。LoadingSpinner は既に共通コンポーネント化済みであり、同じ抽象レベルで Error/Empty state も共通化することで、RemoteData の全状態（NotAsked/Loading/Failure/Success+Empty）に対応するコンポーネントセットを完成させる。

## スコープ

### 対象

**ErrorState コンポーネント化 + リファクタリング（13 箇所、9 ファイル）:**

リフレッシュボタンあり（8 箇所）:
1. `Page/User/List.elm:157` — `ErrorMessage.toUserMessage { entityName = "ユーザー" } err`
2. `Page/Role/List.elm:185` — `ErrorMessage.toUserMessage { entityName = "ロール" } err`
3. `Page/AuditLog/List.elm:300` — `ErrorMessage.toUserMessage { entityName = "監査ログ" } err`
4. `Page/Task/List.elm:148` — "データの取得に失敗しました。"
5. `Page/Workflow/List.elm:248` — "データの取得に失敗しました。"
6. `Page/Workflow/Detail.elm:719` — "データの取得に失敗しました。"
7. `Page/Task/Detail.elm:359` — "データの取得に失敗しました。"
8. `Page/User/Detail.elm:231` — `ErrorMessage.toUserMessage { entityName = "ユーザー" } err`（品質ゲートで検出、追加対応）

リフレッシュボタンなし（5 箇所）:
8. `Page/Home.elm:110` — "統計情報の取得に失敗しました"
9. `Page/User/New.elm:286` — "ロール情報の取得に失敗しました。"
10. `Page/User/Edit.elm:294` — `ErrorMessage.toUserMessage { entityName = "ユーザー" } err`
11. `Page/User/Edit.elm:298` — "ロール情報の取得に失敗しました。"
12. `Page/Role/Edit.elm:296` — `ErrorMessage.toUserMessage { entityName = "ロール" } err`

**EmptyState コンポーネント化 + リファクタリング（3 箇所、3 ファイル）:**
1. `Page/User/List.elm:176` — "ユーザーが見つかりません。"（description なし）
2. `Page/AuditLog/List.elm:317` — "監査ログが見つかりません。"（description なし）
3. `Page/Task/List.elm:162` — "承認待ちのタスクはありません" + description あり

### 対象外

| 箇所 | 除外理由 |
|------|---------|
| `Workflow/New.elm:768` | flex レイアウト + mb-4 + dismissible（MessageAlert と同系統） |
| `Workflow/New.elm:810` | p-8 + text-center（ページ固有のレイアウト） |
| `Workflow/New.elm:1019` | `rounded`（lg なし）+ `text-error-600`（異なるカラー） |
| `Workflow/Detail.elm:1098` | p-3 + text-sm（セクション内のコンパクトエラー） |
| `Workflow/List.elm:268` | Empty state にアクションボタンあり（固有レイアウト） |
| `Role/List.elm:218` | セクションレベルの empty state（py-8、ページレベルではない） |
| `Page/NotFound.elm:18` | 404 ページ（データ空状態ではない） |
| `Data/WorkflowInstance.elm:235` | バッジスタイル（エラー状態表示ではない） |

## 設計

### ErrorState コンポーネント（`Component/ErrorState.elm`）

2 つの関数を公開。`view` は最も多い「エラーメッセージ + 再読み込みボタン」パターン、`viewSimple` はリフレッシュ不可の文脈向け。

```elm
module Component.ErrorState exposing (containerClass, view, viewSimple)

-- リフレッシュボタン付きエラー表示
view :
    { message : String
    , onRefresh : msg
    }
    -> Html msg

-- シンプルなエラー表示（リフレッシュボタンなし）
viewSimple : String -> Html msg

-- テスト用: コンテナの CSS クラス
containerClass : String
```

スタイル: `rounded-lg bg-error-50 p-4 text-error-700`
アクセシビリティ: `role="alert"`（既存の MessageAlert と同じパターン）
リフレッシュボタン: `Button.Outline` バリアント（既存パターン踏襲）

### EmptyState コンポーネント（`Component/EmptyState.elm`）

```elm
module Component.EmptyState exposing (containerClass, view)

view :
    { message : String
    , description : Maybe String
    }
    -> Html msg

-- テスト用: コンテナの CSS クラス
containerClass : String
```

スタイル: `py-12 text-center`
メッセージ: `text-secondary-500`
説明（任意）: `mt-2 text-sm text-secondary-400`

### 設計判断

| 判断 | 選択 | 理由 |
|------|------|------|
| ErrorState の API | `view` + `viewSimple` の 2 関数 | `viewSimple` は String 1 つで呼べて簡潔。`Maybe msg` で統一するより call site が明確 |
| EmptyState の `description` | `Maybe String` | 3 箇所中 1 箇所のみ使用するが、型で表現できるオプショナル性は String パラメータより安全 |
| `role="alert"` の追加 | 採用 | WCAG ベストプラクティス。MessageAlert と一貫。既存コードにはなかったが改善として追加 |
| テストアプローチ | CSS クラス定数のテスト | elm-html-test 未導入（FormFieldTest.elm コメント参照）。公開ヘルパー関数のテストが既存パターン |
| Empty state の padding | py-12 に統一 | User/List, AuditLog/List, Task/List すべて py-12。Role/List (py-8) はセクションレベルのため対象外 |

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: ErrorState コンポーネント作成 + 全ページリファクタリング

#### 確認事項
- [x] パターン: `Component/LoadingSpinner.elm` のモジュール構造 → module + doc comment + `view : Html msg`, `attribute "role" "status"` パターン
- [x] パターン: `Component/Button.elm` の `view` 関数シグネチャ → `{ variant, disabled, onClick } -> List (Html msg) -> Html msg`、config record パターン
- [x] ライブラリ: `Button.view` / `Button.Outline` の使い方 → Button.elm L79-87 確認済み、Outline は border + bg-white スタイル
- [x] パターン: テストファイルの構成 → `suite : Test`, `describe "Component.X"`, `String.contains` で CSS クラスを検証

#### テストリスト

ユニットテスト:
- [x] `containerClass` が `bg-error-50` を含む
- [x] `containerClass` が `rounded-lg` を含む
- [x] `containerClass` が `text-error-700` を含む

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 実装手順
1. `tests/Component/ErrorStateTest.elm` を作成（Red）
2. `src/Component/ErrorState.elm` を作成（Green）
3. 全 13 箇所をリファクタリング（Refactor）
   - `import Component.ErrorState as ErrorState` を追加
   - `ErrorState.view { message = ..., onRefresh = Refresh }` に置換（8 箇所）
   - `ErrorState.viewSimple "..."` に置換（5 箇所）
   - 不要になった `import Component.Button` を削除（該当ファイルで Button が ErrorState 以外で使われていなければ）
4. `just check` で全体コンパイル + テスト通過を確認
5. コミット

#### 変更ファイル
- 新規: `frontend/src/Component/ErrorState.elm`
- 新規: `frontend/tests/Component/ErrorStateTest.elm`
- 変更: `Page/User/List.elm`, `Page/User/Detail.elm`, `Page/Role/List.elm`, `Page/AuditLog/List.elm`, `Page/Task/List.elm`, `Page/Workflow/List.elm`, `Page/Workflow/Detail.elm`, `Page/Task/Detail.elm`, `Page/Home.elm`, `Page/User/New.elm`, `Page/User/Edit.elm`, `Page/Role/Edit.elm`

### Phase 2: EmptyState コンポーネント作成 + 全ページリファクタリング

#### 確認事項
- [x] パターン: Phase 1 で作成した ErrorState のモジュール構造 → module + doc comment + view + containerClass の構成、踏襲
- [x] パターン: Task/List.elm の description 付き empty state → `p [ class "text-secondary-500" ]` + `p [ class "mt-2 text-sm text-secondary-400" ]` の 2 段構成

#### テストリスト

ユニットテスト:
- [x] `containerClass` が `py-12` を含む
- [x] `containerClass` が `text-center` を含む

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 実装手順
1. `tests/Component/EmptyStateTest.elm` を作成（Red）
2. `src/Component/EmptyState.elm` を作成（Green）
3. 全 3 箇所をリファクタリング（Refactor）
   - `import Component.EmptyState as EmptyState` を追加
   - `EmptyState.view { message = "...", description = Nothing }` に置換（2 箇所）
   - `EmptyState.view { message = "...", description = Just "..." }` に置換（1 箇所）
4. `just check` で全体コンパイル + テスト通過を確認
5. コミット

#### 変更ファイル
- 新規: `frontend/src/Component/EmptyState.elm`
- 新規: `frontend/tests/Component/EmptyStateTest.elm`
- 変更: `Page/User/List.elm`, `Page/AuditLog/List.elm`, `Page/Task/List.elm`

## 検証

1. `just check` — リント + テスト通過
2. `just check-all` — 全テスト（API テスト + E2E テスト含む）通過
3. 手動確認: `bg-error-50` の Grep 結果が、対象外の箇所のみであること

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Workflow/New に 3 つの異なるエラーパターン（flex+dismissible, p-8+centered, rounded+error-600）があり、これらを一律に共通化すると不適切 | 既存手段の見落とし / アーキテクチャ不整合 | スコープ対象外に分類し、除外理由を明記 |
| 2回目 | Role/List の empty state が py-8 で他と異なるが、これはセクションレベルの空表示であり、ページレベルの py-12 に統一すべきではない | 競合・エッジケース | Role/List をスコープ対象外とし、EmptyState はページレベル（py-12）に限定 |
| 3回目 | ErrorState に `role="alert"` を追加すべきか — 既存コードにはないが MessageAlert には設定されている | ベストプラクティス | WCAG ベストプラクティスに従い追加。MessageAlert との一貫性も確保 |
| 4回目 | テストアプローチ — elm-html-test が未導入で HTML 構造テストが不可能。テスト手段の制約を明確化 | 技術的前提 | FormFieldTest.elm のパターン（公開ヘルパー関数のテスト）に従い、`containerClass` を公開してテスト |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | `bg-error-50` の Grep 結果を分類（対象 13 + 対象外 6 + MessageAlert 1）。品質ゲートで User/Detail.elm の漏れを検出し追加対応。Empty state 6 箇所も分類（対象 3 + 対象外 3） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各関数のシグネチャ、CSS クラス、対象ファイルの行番号を明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | API 設計（2 関数 vs Maybe）、テストアプローチ、アクセシビリティ、padding 統一の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「スコープ」セクションで対象 / 対象外を明記、各対象外に除外理由あり |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | elm-html-test 未導入の制約、WCAG `role="alert"` 要件を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #613 の完了基準と整合。Epic #445 の方向性と一致 |
