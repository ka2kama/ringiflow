# 計画: #331 Elm テストカバレッジ戦略

## Context

Issue #331: Elm フロントエンドのテスト戦略検討とテスト追加。バックエンドのテストカバレッジ改善（#291）に続き、フロントエンドのテスト品質を向上させる。

現状: ソース 35 ファイル中 12 ファイルがテスト済み。特に外部境界（JSON デコーダ）とビジネスロジック（Page update）のテストが不足している。

## スコープ

**対象:**
- ADR-037: elm-coverage 採用見送り
- 優先度「高」: JSON デコーダテスト、Page update ロジックテスト
- 優先度「中」の一部: Data.Dashboard, Data.UserRef（Phase 2 に統合）

**対象外:**
- Main, Ports, Shared（アプリケーション配線）
- Component/* の View テスト（型が構造を保証、変更頻度低い）
- パススルー Api モジュール（Api.Dashboard, Api.Task, Api.User, Api.WorkflowDefinition）
  - 理由: Data モジュールのデコーダを呼ぶだけで、独自のデコード/エンコードロジックを持たない
- Form.DynamicForm（extractFormFields のみで、テスト価値が低い）

## 設計判断

### テスト対象の exposing 拡張

一部モジュールの内部関数（デコーダ、エンコーダ）がテストのために公開されていない。以下の方針で対応する:

| 選択肢 | メリット | デメリット |
|--------|---------|-----------|
| **A. exposing 拡張（採用）** | 最小変更、KISS | カプセル化が緩む |
| B. Internal モジュール | テスト専用の公開経路 | ファイル増加、過剰設計 |
| C. コード再構成 | アーキテクチャ改善 | スコープ肥大 |

選択: **A**。Elm コミュニティでは一般的なプラクティス。プロジェクト規模に対して B/C は過剰。

対象モジュール:
- `Api.Auth`: `csrfTokenDecoder`, `userDecoder` を追加
- `Api.Workflow`: `CreateWorkflowRequest`, `SubmitWorkflowRequest`, `encodeCreateRequest`, `encodeSubmitRequest`, `encodeApproveRejectRequest` を追加
- `Page.Workflow.New`: `Msg(..)`, `ApproverSelection(..)`, `SaveMessage(..)` を追加

---

## Phase 1: ADR-037 elm-coverage 採用見送り

#### 確認事項
- パターン: ADR 形式 → `docs/70_ADR/035_コードカバレッジ計測ツール選定.md`
- 型: ADR 番号 → 036 が最新、037 を使用

#### 成果物
- `docs/70_ADR/037_Elmコードカバレッジツール選定.md`

#### 内容の要点
- elm-coverage を採用しない
- 理由: 4年間メンテナンスなし、elm-test revision17 との互換性未検証、LCOV 未対応
- 代替: テスト品質は戦略的なテスト選定（Issue #331 の優先度表）で担保

---

## Phase 2: Data モジュールのデコーダテスト

#### 確認事項
- 型: `TaskItem`, `TaskDetail`, `WorkflowSummary` → `frontend/src/Data/Task.elm`
- 型: `DashboardStats` → `frontend/src/Data/Dashboard.elm`
- 型: `UserRef` → `frontend/src/Data/UserRef.elm`
- 型: `StepStatus(..)`, `WorkflowStep`, `WorkflowInstance` → `frontend/src/Data/WorkflowInstance.elm`（TaskDetail で使用）
- パターン: 既存デコーダテスト → `frontend/tests/Data/FormFieldTest.elm`, `frontend/tests/Data/WorkflowInstanceTest.elm`

#### テストリスト

**Data.TaskTest** (`frontend/tests/Data/TaskTest.elm`):

WorkflowSummary:
- [ ] 全フィールドをデコード（initiatedBy のネスト含む）
- [ ] submitted_at が null の場合 Nothing
- [ ] 必須フィールド欠落でエラー

TaskItem:
- [ ] 全フィールドをデコード（ネストされた WorkflowSummary 含む）
- [ ] optional フィールド（assigned_to, due_date, started_at）が null の場合
- [ ] version 省略時のデフォルト値（1）

listDecoder:
- [ ] data フィールドから一覧をデコード
- [ ] 空の一覧をデコード

detailDecoder:
- [ ] step と workflow をデコード

**Data.DashboardTest** (`frontend/tests/Data/DashboardTest.elm`):
- [ ] 全フィールドをデコード（data ラッパー含む）
- [ ] 必須フィールド欠落でエラー

**Data.UserRefTest** (`frontend/tests/Data/UserRefTest.elm`):
- [ ] 全フィールドをデコード
- [ ] 必須フィールド欠落でエラー

---

## Phase 3: Api モジュールのデコーダ/エンコーダテスト

#### 確認事項
- 型: `User` → `frontend/src/Shared.elm`（id, email, name, tenantId, roles）
- 型: `CreateWorkflowRequest`, `SubmitWorkflowRequest`, `ApproveRejectRequest` → `frontend/src/Api/Workflow.elm`
- パターン: エンコーダテスト → プロジェクト内に前例なし
- ライブラリ: `Json.Encode.encode` → Grep で既存使用確認
- ライブラリ: `Json.Decode.decodeString` → 既存テストで使用済み

#### エンコーダテスト方式の設計判断

エンコーダのテスト方法:

| 方式 | メリット | デメリット |
|------|---------|-----------|
| **A. エンコード → デコードで往復検証（採用）** | 型安全、構造的 | デコーダの正しさに依存 |
| B. JSON 文字列比較 | 直接的 | フィールド順序に依存、脆い |

選択: **A**。`Encode.encode 0 value` で JSON 文字列化し、`Decode.decodeString` で再パースして検証する。

#### コード変更
- `Api.Auth`: exposing に `csrfTokenDecoder`, `userDecoder` を追加
- `Api.Workflow`: exposing に `CreateWorkflowRequest`, `SubmitWorkflowRequest`, `encodeCreateRequest`, `encodeSubmitRequest`, `encodeApproveRejectRequest` を追加

#### テストリスト

**Api.AuthTest** (`frontend/tests/Api/AuthTest.elm`):

csrfTokenDecoder:
- [ ] data.token をデコード
- [ ] data フィールド欠落でエラー
- [ ] token フィールド欠落でエラー

userDecoder:
- [ ] 全フィールドをデコード（id, email, name, tenant_id, roles）
- [ ] roles が空配列
- [ ] 必須フィールド欠落でエラー

**Api.WorkflowTest** (`frontend/tests/Api/WorkflowTest.elm`):

encodeCreateRequest:
- [ ] 全フィールドをエンコード（definition_id, title, form_data）

encodeSubmitRequest:
- [ ] assigned_to をエンコード

encodeApproveRejectRequest:
- [ ] コメントありの場合 version + comment
- [ ] コメントなしの場合 version のみ（comment フィールド不在）

---

## Phase 4: Page.Workflow.New update テスト

#### 確認事項
- 型: `Model`, `Msg`, `ApproverSelection`, `SaveMessage` → `frontend/src/Page/Workflow/New.elm`
- パターン: Page update テスト → プロジェクト内に前例なし
- ライブラリ: `RemoteData` → Grep で既存使用確認
- ライブラリ: `Tuple.first` → テスト内で `(Model, Cmd Msg)` から Model 抽出に使用

#### コード変更
- `Page.Workflow.New`: exposing に `Msg(..)`, `ApproverSelection(..)`, `SaveMessage(..)` を追加

#### テスト方式

`init` で初期モデルを取得し、`update` にメッセージを送って結果の Model を検証する:

```elm
let
    shared = Shared.init { apiBaseUrl = "", timezoneOffsetMinutes = 540 }
    ( initialModel, _ ) = init shared
    updatedModel = update SomeMsg initialModel |> Tuple.first
in
Expect.equal updatedModel.someField expectedValue
```

#### テストリスト

**Page.Workflow.NewTest** (`frontend/tests/Page/Workflow/NewTest.elm`):

SaveDraft バリデーション:
- [ ] 定義未選択でエラーメッセージ（saveMessage = SaveError）
- [ ] タイトル空でバリデーションエラー（validationErrors に "title"）
- [ ] 定義選択済み + タイトル入力済みで submitting = True

Submit バリデーション:
- [ ] 承認者未選択でバリデーションエラー（validationErrors に "approver"）
- [ ] タイトル空 + 承認者未選択で複数エラー

承認者キーボード操作:
- [ ] ArrowDown でインデックス増加
- [ ] ArrowUp でインデックス減少（循環）
- [ ] Enter で候補選択（approverSelection = Selected）
- [ ] Escape でドロップダウン閉じる

Dirty 状態管理:
- [ ] 初期状態で isDirty = False
- [ ] UpdateTitle で isDirty = True
- [ ] GotSaveResult (Ok ...) で isDirty = False

---

## 検証方法

```bash
# Elm テスト実行
cd frontend && pnpm run test

# 特定のテストファイルのみ
cd frontend && pnpm run test -- --watch tests/Data/TaskTest.elm

# 全体チェック
just check-all
```

---

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → Api モジュールの実態確認 | Api モジュールのソースを読み、独自デコーダの有無を確認 | Api.Dashboard/Task/User/WorkflowDefinition はパススルーのみ → 対象外に。Api.Auth と Api.Workflow のみ独自ロジックあり |
| 2回目 | Page テストの feasibility | Page.Workflow.New の exposing を確認。Msg コンストラクタ非公開 → テスト方式を検討 | exposing 拡張方式を採用（Elm コミュニティの標準的プラクティス） |
| 3回目 | エンコーダテストのパターン | プロジェクト内に前例なし → テスト方式を設計 | エンコード → デコード往復検証方式を採用 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準に対応する Phase がある | OK | ADR → Phase 1、デコーダテスト → Phase 2-3、Page update → Phase 4。優先度「中」の Data.Dashboard/UserRef は Phase 2 に統合 |
| 2 | 曖昧さ排除 | テストリストが具体的で一意 | OK | 各テストケースの期待値を明記（フィールド名、エラー条件） |
| 3 | 設計判断の完結性 | exposing 拡張、エンコーダテスト方式が決定済み | OK | 選択肢・理由・トレードオフを記載 |
| 4 | スコープ境界 | 対象・対象外が明確 | OK | 対象外を理由付きで明記（パススルー Api、View コンポーネント、配線モジュール） |
| 5 | 技術的前提 | Elm の exposing による可視性制御を考慮 | OK | 各 Phase でテストに必要な exposing 変更を明記 |
| 6 | 既存ドキュメント整合 | Issue #331 の優先度表と一致 | OK | 優先度「高」→ Phase 2-4、「中」の一部 → Phase 2 に統合、「低」「対象外」→ スコープ外 |
