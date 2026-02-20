# #724 ワークフロー定義管理画面の実装計画

## コンテキスト

Epic #405（ワークフローデザイナー）の Story #724。テナント管理者がワークフロー定義を GUI で管理するための画面を実装する。

ブロッカーの #723（CRUD API）は PR #730 で完了済み。バックエンドは全8エンドポイント（一覧・詳細・作成・更新・削除・公開・アーカイブ・バリデーション）が実装済みのため、フロントエンド実装に集中する。

## スコープ

対象:
- ワークフロー定義一覧ページ（Draft/Published/Archived の全ステータス表示）
- 新規定義作成機能（ダイアログで名前・説明を入力 → Draft 作成 → 一覧に反映）
- ステータス操作 UI（公開・アーカイブ、ConfirmDialog で確認）
- Draft 定義の削除（ConfirmDialog で確認）
- ルーティング追加（`/workflow-definitions`）
- 管理者ナビゲーションへの追加

対象外:
- デザイナー画面（#725, #726 で実装）
- 定義の編集（PUT）UI — デザイナーの責務
- バリデーションエンドポイントの呼び出し — デザイナーの責務
- 一般ユーザー向けの定義一覧の変更（既存の申請時選択は変更なし）

### 設計判断

**新規作成の UX**: ダイアログ方式を採用（別ページではなく）。理由: 作成時の入力は名前と説明のみで、実質的な編集はデザイナー（#725/#726）で行う。別ページを設ける必要性がない。

**デザイナーへの遷移**: Issue に「Draft 作成 → デザイナーへ遷移」とあるが、デザイナー（#725/#726）は未実装。本 Story では作成後に一覧を更新し、Draft 定義を表示する。一覧からのデザイナーへの遷移リンクは #725 で追加する。Story 独立性を優先。

**URL 設計**: `/workflow-definitions` — 既存の `/workflows`（申請インスタンス）と明確に区別。

**ステータスフィルタ**: フィルタ UI を設ける。`select` で All/Draft/Published/Archived を選択可能。URL クエリパラメータ `?status=draft` に対応。

## 変更ファイル一覧

### 新規作成
| ファイル | 目的 |
|---------|------|
| `frontend/src/Page/WorkflowDefinition/List.elm` | 定義一覧ページ（メインの実装） |
| `frontend/tests/Data/WorkflowDefinitionStatusTest.elm` | ステータス型のテスト |
| `frontend/tests/RouteTest.elm` | ルートテスト（追記） |
| `tests/e2e/tests/workflow-definition-management.spec.ts` | E2E テスト |

### 変更
| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/Data/WorkflowDefinition.elm` | ステータス型、バッジ、エンコーダー追加 |
| `frontend/src/Api/WorkflowDefinition.elm` | CRUD API 関数追加 |
| `frontend/src/Route.elm` | `WorkflowDefinitions` ルート追加 |
| `frontend/src/Main.elm` | ページ統合（Page/Msg/update/view/等） |
| `frontend/src/Component/Icons.elm` | ワークフロー定義アイコン追加 |
| `docs/08_テスト/E2Eテスト突合表.md` | E2E テスト突合表更新 |

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: Data モデル拡張

`Data/WorkflowDefinition.elm` を拡張し、管理画面に必要な型とヘルパーを追加する。

#### 確認事項
- [x] 型: 既存の `WorkflowDefinition` 型定義 → `Data/WorkflowDefinition.elm` L36-56, status は String 型
- [x] パターン: 既存ステータス型パターン → `Data/WorkflowInstance.elm` の `Status` カスタム型 + `statusToJapanese` + `statusToCssClass`
- [x] パターン: Badge 表示パターン → `Data/AdminUser.elm` の `statusToBadge` 関数
- [x] パターン: JSON エンコーダー → `Api/Workflow.elm` の `encodeCreateWorkflowRequest` 等

追加する型とヘルパー:

```elm
-- ステータスカスタム型
type WorkflowDefinitionStatus
    = Draft
    | Published
    | Archived

-- ヘルパー関数
statusFromString : String -> WorkflowDefinitionStatus
statusToJapanese : WorkflowDefinitionStatus -> String
statusToBadge : WorkflowDefinitionStatus -> { colorClass : String, label : String }
definitionStatus : WorkflowDefinition -> WorkflowDefinitionStatus

-- エンコーダー
encodeCreateRequest : { name : String, description : String } -> Encode.Value
encodeVersionRequest : { version : Int } -> Encode.Value
```

`encodeCreateRequest` は name, description, 最小限のデフォルト definition（開始ステップのみ）を含む JSON を生成する。

#### テストリスト

ユニットテスト:
- [ ] `statusFromString` が "draft"/"published"/"archived" を正しく変換する
- [ ] `statusFromString` が不明な値に Draft を返す（フォールバック）
- [ ] `statusToJapanese` が日本語ラベルを返す
- [ ] `statusToBadge` がステータスに応じたバッジ設定を返す
- [ ] `definitionStatus` が WorkflowDefinition の status フィールドを型に変換する
- [ ] `encodeCreateRequest` が正しい JSON 構造を生成する
- [ ] `encodeVersionRequest` が version フィールドを含む JSON を生成する

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 2: API クライアント拡張

`Api/WorkflowDefinition.elm` に CRUD 操作用の API 関数を追加する。

#### 確認事項
- [ ] ライブラリ: `Api.post` の使い方 → `Api.elm` L141-158, `{ config, url, body, decoder, toMsg }`
- [ ] ライブラリ: `Api.deleteNoContent` の使い方 → `Api.elm` L241-256, `{ config, url, toMsg }`
- [ ] パターン: 既存 CRUD API パターン → `Api/Role.elm` の `createRole`, `deleteRole`

追加する関数:

```elm
createDefinition : { config, body, toMsg } -> Cmd msg
publishDefinition : { config, id, body, toMsg } -> Cmd msg
archiveDefinition : { config, id, body, toMsg } -> Cmd msg
deleteDefinition : { config, id, toMsg } -> Cmd msg
```

既存の `listDefinitions` のコメントも修正する（「公開済みのみ」→「全ステータス」）。

#### テストリスト

ユニットテスト（該当なし — API 関数は HTTP 呼び出しのラッパーのため、結合テストで検証）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 3: ルーティング

`Route.elm` に `WorkflowDefinitions` ルートを追加し、`RouteTest.elm` にテストを追加する。

#### 確認事項
- [ ] 型: 既存の Route 型 → `Route.elm` L81-96, 14バリアント
- [ ] パターン: ルート追加パターン → `Route.elm` の `parser`, `toString`, `isRouteActive`, `pageTitle`
- [ ] パターン: RouteTest のテストパターン → `RouteTest.elm` の `fromUrlTests`, `toStringTests`

追加するルート:

```elm
type Route
    = ...
    | WorkflowDefinitions  -- /workflow-definitions
```

`pageTitle`: "ワークフロー定義"

`isRouteActive`: `WorkflowDefinitions` 同士のみ（子ルートはこの Story では不要）

#### テストリスト

ユニットテスト:
- [ ] `/workflow-definitions` → `WorkflowDefinitions`
- [ ] `WorkflowDefinitions` → `/workflow-definitions`
- [ ] `isRouteActive` で `WorkflowDefinitions` 同士がアクティブ

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 4: 一覧ページ実装

`Page/WorkflowDefinition/List.elm` を実装する。既存の管理系一覧ページ（`Page/Role/List.elm`, `Page/User/List.elm`）のパターンに準拠する。

#### 確認事項
- [ ] パターン: 一覧ページの Model/Msg/init/update/view 構造 → `Page/Role/List.elm`
- [ ] パターン: ConfirmDialog の使い方 → `Page/Role/List.elm` L88-131, L294-309
- [ ] パターン: MessageAlert の使い方 → `Page/Role/List.elm` の viewContent 先頭
- [ ] パターン: ステータスフィルタ → `Page/User/List.elm` L132-144
- [ ] パターン: RemoteData 4状態処理 → `Page/User/List.elm` L149-165
- [ ] パターン: EmptyState → `Page/User/List.elm` L170-183
- [ ] ライブラリ: `Ports.showModalDialog` の使い方 → Grep で既存使用を確認

Model:

```elm
type alias Model =
    { shared : Shared
    , definitions : RemoteData ApiError (List WorkflowDefinition)
    , statusFilter : Maybe WorkflowDefinitionStatus
    , pendingAction : Maybe PendingAction       -- ConfirmDialog 用
    , isProcessing : Bool                       -- 操作中フラグ
    , successMessage : Maybe String
    , errorMessage : Maybe String
    , showCreateForm : Bool                     -- 作成ダイアログ表示
    , createName : String                       -- 作成フォーム: 名前
    , createDescription : String                -- 作成フォーム: 説明
    , createValidationErrors : Dict String String
    }

type PendingAction
    = ConfirmPublish WorkflowDefinition
    | ConfirmArchive WorkflowDefinition
    | ConfirmDelete WorkflowDefinition
```

View 構成:
- ヘッダー（タイトル + 新規作成ボタン）
- ステータスフィルタ（select: すべて/下書き/公開済み/アーカイブ済み）
- 定義テーブル（名前、ステータスバッジ、更新日時、操作ボタン）
- 操作ボタン（ステータスに応じて異なる）:
  - Draft: 「公開」「削除」
  - Published: 「アーカイブ」
  - Archived: （操作なし）
- 作成ダイアログ（`<dialog>` 要素、名前・説明の入力）
- 確認ダイアログ（ConfirmDialog コンポーネント再利用）

#### テストリスト

ユニットテスト（該当なし — ページモジュールのロジックは型とコンポーネントに委譲済み）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし — Phase 6 で実施）

### Phase 5: Main.elm 統合 & ナビゲーション

Main.elm にページを統合し、管理者セクションにナビゲーション項目を追加する。

#### 確認事項
- [ ] パターン: ページ統合パターン → Main.elm の Page 型(L83-98), initPage(L186-288), Msg(L355-377), update(L499-693), viewPage(L930-991), updatePageShared(L297-343)
- [ ] パターン: viewAdminSection → Main.elm L857-868
- [ ] パターン: Icons コンポーネント → `Component/Icons.elm` の既存アイコン関数

Main.elm の変更箇所（計8箇所）:

1. import 追加: `import Page.WorkflowDefinition.List as WorkflowDefinitionList`
2. Page 型: `| WorkflowDefinitionsPage WorkflowDefinitionList.Model`
3. initPage: `Route.WorkflowDefinitions ->` ケース追加
4. Msg 型: `| WorkflowDefinitionsMsg WorkflowDefinitionList.Msg`
5. update: `WorkflowDefinitionsMsg subMsg ->` ハンドラー追加
6. viewPage: `WorkflowDefinitionsPage subModel ->` ケース追加
7. updatePageShared: `WorkflowDefinitionsPage subModel ->` ケース追加
8. viewAdminSection: `viewNavItem currentRoute Route.WorkflowDefinitions "ワークフロー定義" Icons.workflowDefinitions` 追加

Icons.elm に `workflowDefinitions` アイコン（設計図/フロー図モチーフ）を追加する。

#### テストリスト

ユニットテスト（該当なし — Elm コンパイラが型安全に全分岐を強制するため、統合テストで十分）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし — Phase 6 で実施）

### Phase 6: E2E テスト

管理者が定義一覧の閲覧・作成・公開・アーカイブを完了できることを E2E テストで検証する。

#### 確認事項
- [ ] パターン: E2E テストパターン → `tests/e2e/tests/workflow.spec.ts`
- [ ] パターン: 認証セットアップ → `tests/e2e/tests/auth.setup.ts` + `helpers/test-data.ts`
- [ ] パターン: Playwright config → `tests/e2e/playwright.config.ts` の storageState
- [ ] ライブラリ: Playwright API → 既存テストの `getByRole`, `getByText`, `locator` パターン

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト:
- [ ] テナント管理者が定義一覧ページにアクセスできる
- [ ] テナント管理者が新しいワークフロー定義を作成できる（Draft 状態）
- [ ] テナント管理者が定義を公開・アーカイブできる

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | デザイナー遷移の扱いが未定義 | 不完全なパス | Story 独立性を優先し、作成後は一覧更新のみとする判断を設計判断セクションに記載 |
| 2回目 | 作成 UI のアプローチ（ダイアログ vs 別ページ）が未決定 | 曖昧 | ダイアログ方式を選択、理由を設計判断に記載 |
| 3回目 | フィルタの URL パラメータ対応が未検討 | 未定義 | URL クエリパラメータ `?status=draft` に対応するが、Phase 3（ルーティング）のスコープは単純な `/workflow-definitions` のみ。フィルタはページ内状態で管理し、URL パラメータ対応は過剰設計として見送り |
| 4回目 | ConfirmDialog の使い分け（公開/アーカイブ/削除で異なる ActionStyle）が未明記 | 曖昧 | PendingAction 型で操作種別を管理し、ActionStyle を Positive（公開）/Caution（アーカイブ）/Destructive（削除）にマッピング |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Issue の完了基準6項目すべてに対応する Phase がある。API は全8エンドポイント実装済みで、フロントエンドで使用する4操作（一覧/作成/公開/アーカイブ + 削除）をカバー |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | デザイナー遷移の扱い、作成 UI のアプローチ、フィルタ方式を全て判断済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | ダイアログ vs 別ページ、URL パラメータ対応、デザイナー遷移の3判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象/対象外セクションで明示。デザイナー関連は対象外 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | BFF の GET が全ステータスを返すこと（バックエンド調査で確認済み）、ConfirmDialog の `<dialog>` 要素は Ports 経由の showModal が必要 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 詳細設計書 15 のフロントエンド設計（Page/WorkflowDefinition/List.elm）と一致。ADR-053 の技術選定と矛盾なし |
