# 計画: ワークフロー詳細 API にステップデータを含める修正 + 改善記録

## 背景

Issue #36 Phase 5 実装時に、OpenAPI 仕様（200 OK + steps 含む WorkflowResponse）に反して
204 No Content が実装された。これにより：
- 詳細画面でステップ一覧・承認/却下ボタンが表示されない
- approve/reject API のレスポンスがフロントエンドのデコーダーと不一致

フロントエンド（Phase 6）は既に対応済み。バックエンド 4 レイヤーの修正が必要。

## タスク

### タスク 1: 改善記録の作成

`prompts/improvements/2026-01/2026-01-29_1445_Phase5実装時のOpenAPI仕様突合漏れ.md`

パターン B（構造化型）で以下を記載:
- 事象: Phase 5 で 204 No Content を実装、steps/version フィールド欠落
- 原因分析: OpenAPI 仕様との突合未実施、Phase 間の結合検証なし
- 対策: Phase 完了基準に OpenAPI 突合を追加
- 教訓: Silent Failure パターンの危険性

### タスク 2: バックエンド修正（4 Phase）

#### Phase 1: Core Service ユースケース

**ファイル**: `backend/apps/core-service/src/usecase/workflow.rs`

1. `WorkflowWithSteps` 構造体を追加（行 27 付近）
   ```rust
   pub struct WorkflowWithSteps {
       pub instance: WorkflowInstance,
       pub steps: Vec<WorkflowStep>,
   }
   ```
2. `get_workflow` 戻り値: `Result<WorkflowInstance, _>` → `Result<WorkflowWithSteps, _>`
   - `find_by_id` 後に `step_repo.find_by_instance()` でステップ取得
3. `approve_step` 戻り値: `Result<(), _>` → `Result<WorkflowWithSteps, _>`
   - save 後に `step_repo.find_by_instance()` でステップ再取得
4. `reject_step` も同様

#### Phase 2: Core Service ハンドラ DTO

**ファイル**: `backend/apps/core-service/src/handler/workflow.rs`

1. `WorkflowStepDto` 構造体を新規追加（Serialize）
   - OpenAPI の WorkflowStep スキーマに合わせたフィールド
   - `From<WorkflowStep>` 実装（既存パターン: `format!("{:?}", ...)` で enum を文字列化）
2. `WorkflowInstanceDto` に `version`, `steps`, `completed_at` フィールド追加
3. `From<WorkflowWithSteps> for WorkflowInstanceDto` 実装
4. 既存の `From<WorkflowInstance>` は一覧 API 用に残す（steps: Vec::new()）
5. `get_workflow` ハンドラ: `WorkflowWithSteps` を使ってレスポンス構築
6. `approve_step` / `reject_step`: 204 → 200 OK + WorkflowResponse

#### Phase 3: BFF クライアント

**ファイル**: `backend/apps/bff/src/client/core_service.rs`

1. `WorkflowStepDto` 追加（Deserialize）
2. `WorkflowInstanceDto` に `version`, `steps`（`#[serde(default)]`）, `completed_at` 追加
3. `approve_step` / `reject_step` 戻り値: `Result<(), _>` → `Result<WorkflowResponse, _>`
4. トレイト定義と実装の両方を修正

#### Phase 4: BFF ハンドラ

**ファイル**: `backend/apps/bff/src/handler/workflow.rs`

1. `WorkflowStepData` 追加（Serialize）
2. `WorkflowData` に `version`, `steps`, `completed_at` 追加
3. `From<WorkflowInstanceDto> for WorkflowData` に steps マッピング追加
4. `approve_step` / `reject_step`: 204 → 200 OK + WorkflowResponse

## 設計判断

- **`WorkflowWithSteps` 構造体**: タプルではなく名前付き構造体を採用。プロジェクト理念「型で表現できるものは型で表現する」に合致
- **配置場所**: ユースケースモジュール内。ドメインモデルではなくユースケース出力の集約
- **一覧 API のスコープ外**: 一覧 API のステップ返却は今回修正しない（`steps: Vec::new()` or `#[serde(default)]`）
- **OpenAPI 修正不要**: 仕様は正しい。実装を仕様に合わせる

## 検証

1. `just check-all` で lint + テスト通過
2. `just dev-all` → ブラウザで `/workflows/33333333-3333-3333-3333-333333333333` を開き、承認/却下ボタンが表示されることを確認
3. 承認ボタンをクリックして、画面が正常に更新されることを確認
