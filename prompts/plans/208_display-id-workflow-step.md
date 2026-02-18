# 実装計画: #208 表示用 ID: WorkflowStep への導入（Phase B）

## 概要

Phase A（WorkflowInstance）で確立したパターンを適用し、WorkflowStep に表示用 ID（`STEP-7` 形式）を導入する。

設計書: [docs/03_詳細設計書/12_表示用ID設計.md](../../docs/03_詳細設計書/12_表示用ID設計.md)

## 完了基準（Issue #208）

- [ ] `workflow_steps.display_number` カラムが追加されている
- [ ] 既存データに `display_number` が割り当てられている
- [ ] ステップ作成時に `display_number` が自動採番される
- [ ] API レスポンスにステップの `display_id` が含まれる
- [ ] OpenAPI 仕様書が更新されている
- [ ] フロントエンド: ワークフロー詳細のステップに表示用 ID が表示される（E2E）

---

## Phase 構成

### Phase 1: DB スキーマ変更

**マイグレーションファイル:**
1. `20260206000001_add_display_number_to_workflow_steps.sql`
   - カラム追加（NULLABLE）
   - ユニーク制約: `(instance_id, display_number)` — インスタンス内で一意
   - 既存データマイグレーション
   - カウンター初期化

2. `20260206000002_set_workflow_steps_display_number_not_null.sql`
   - NOT NULL 制約追加

**参考（Phase A）:**
- `backend/migrations/20260202000002_add_display_number_to_workflow_instances.sql`
- `backend/migrations/20260202000003_set_display_number_not_null.sql`

**設計判断:**
- 採番スコープ: テナント単位（`display_id_counters` の `entity_type = 'workflow_step'`）
- ユニーク制約: `(instance_id, display_number)` — 設計書の指定通り

### Phase 2: ドメインモデル

**変更ファイル:**
- `backend/crates/domain/src/workflow.rs`

**変更内容:**
1. `WorkflowStep` 構造体に `display_number: DisplayNumber` フィールド追加
2. `NewWorkflowStep` に `display_number: DisplayNumber` 追加
3. `WorkflowStepRecord` に `display_number: DisplayNumber` 追加
4. getter メソッド `display_number(&self) -> DisplayNumber` 追加

**参考（Phase A）:**
- `WorkflowInstance` の `display_number` フィールド（L339）

### Phase 3: インフラ層（リポジトリ）

**変更ファイル:**
- `backend/crates/infra/src/repository/workflow_step_repository.rs`

**変更内容:**
1. INSERT 文に `display_number` カラム追加
2. SELECT 文に `display_number` カラム追加
3. `from_row` で `display_number` を復元

**参考（Phase A）:**
- `workflow_instance_repository.rs` の display_number 処理

### Phase 4: ユースケース層（採番統合）

**変更ファイル:**
- `backend/apps/core-service/src/usecase/workflow.rs`

**変更内容:**
1. ステップ作成時（`submit_workflow` 内）で `counter_repo.next_display_number()` を呼び出し
2. `DisplayIdEntityType::WorkflowStep` を指定

**現状確認ポイント:**
- ステップは `submit_workflow` で作成される
- `WorkflowUseCaseImpl` には既に `counter_repo` が DI されている

### Phase 5: API + フロントエンド

**バックエンド変更:**
1. Core Service: `WorkflowStepDto` に `display_id: String` 追加
2. BFF: `StepData` に `display_id: String` 追加（そのまま通す）
3. OpenAPI: `WorkflowStep` スキーマに `display_id` プロパティ追加

**フロントエンド変更:**
1. `Data/WorkflowInstance.elm`:
   - `WorkflowStep` 型に `displayId : String` 追加
   - `stepDecoder` に `|> required "display_id" Decode.string` 追加
2. `Page/Workflow/Detail.elm`:
   - ステップ表示に `displayId` を追加

**参考ファイル:**
- `openapi/openapi.yaml` L1003-1087（WorkflowStep スキーマ）
- `frontend/src/Data/WorkflowInstance.elm` L58-66, L339-348

---

## 変更ファイル一覧

| Phase | ファイル | 変更種別 |
|-------|---------|---------|
| 1 | `backend/migrations/20260206000001_add_display_number_to_workflow_steps.sql` | 新規 |
| 1 | `backend/migrations/20260206000002_set_workflow_steps_display_number_not_null.sql` | 新規 |
| 2 | `backend/crates/domain/src/workflow.rs` | 修正 |
| 3 | `backend/crates/infra/src/repository/workflow_step_repository.rs` | 修正 |
| 4 | `backend/apps/core-service/src/usecase/workflow.rs` | 修正 |
| 5 | `backend/apps/core-service/src/handler/workflow.rs` | 修正 |
| 5 | `backend/apps/bff/src/handler/workflow.rs` | 修正 |
| 5 | `openapi/openapi.yaml` | 修正 |
| 5 | `frontend/src/Data/WorkflowInstance.elm` | 修正 |
| 5 | `frontend/src/Page/Workflow/Detail.elm` | 修正 |

---

## 検証方法

1. **単体テスト**: `just check` でドメイン・インフラのテストが通ること
2. **統合テスト**: `just test-rust-integration` でリポジトリテストが通ること
3. **API テスト**: `just check-all` で Hurl テストが通ること
4. **E2E 確認**: 開発サーバー起動 → ワークフロー詳細画面でステップに `STEP-N` が表示されること

---

## 自己検証（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue #208 の完了基準 6 項目すべてを Phase 1-5 でカバー。Phase A の実装パターンを全レイヤーで確認済み |
| 2 | 曖昧さ排除 | OK | 変更ファイル、変更内容、参考ファイルを具体的に列挙。「必要に応じて」等の不確定表現なし |
| 3 | 設計判断の完結性 | OK | 採番スコープ（テナント単位）、ユニーク制約（インスタンス単位）を設計書から引用して明記 |
| 4 | スコープ境界 | OK | Phase B の WorkflowStep のみが対象。Phase C（User）は別 Issue #203 |
| 5 | 技術的前提 | OK | マイグレーションの段階的適用（NULLABLE → データ移行 → NOT NULL）は Phase A と同じパターン |
| 6 | 既存ドキュメント整合 | OK | 設計書 `12_表示用ID設計.md` の Phase B セクション、OpenAPI 仕様変更箇所と整合 |
