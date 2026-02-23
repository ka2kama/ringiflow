# WorkflowStep ADT ステートマシンリファクタリング

## 概要

Issue #820 に基づき、`WorkflowStep` のドメインモデルを ADT（代数的データ型）ベースのステートマシンにリファクタリングした。ADR-054 Pattern A（外側共通 + 状態 enum）を適用し、エンティティ不変条件（INV-S2〜S4）を型レベルで強制する構造に変換した。

## 実施内容

### Phase 1: Domain model ADT リファクタリング

対象: `backend/crates/domain/src/workflow/step.rs`

- `WorkflowStepState` enum（Pending / Active / Completed / Skipped）を新規定義
- `ActiveStepState`（`started_at` 必須）、`CompletedStepState`（`decision`, `started_at`, `completed_at` 必須）を定義
- `WorkflowStep` 構造体から `status`, `decision`, `comment`, `started_at`, `completed_at` フィールドを除去し、`state: WorkflowStepState` に集約
- `from_db()` を `Result<Self, DomainError>` に変更し、不変条件を検証
- 全 getter メソッドを `state` からの導出に変更（戻り値型は維持）
- `state()` getter を新規追加（パターンマッチ用）
- 状態遷移メソッド（`approve`, `reject`, `request_changes`, `completed`, `activated`, `skipped`）をパターンマッチベースに変更
- `is_overdue()` を getter 委譲に変更
- 既存テスト 15 件を更新（`.unwrap()` 追加）、新規テスト 4 件を追加（from_db 不変条件バリデーション）
- 全 19 テストパス

### Phase 2: Repository・依存クレートの修正

対象: `backend/crates/infra/src/repository/workflow_step_repository.rs`

- `TryFrom<WorkflowStepRow>` の `from_db()` 呼び出しを `Ok()` ラッパーから `.map_err()` チェーンに変更
- `DomainError` → `InfraError::Unexpected` のレイヤー境界エラー変換
- infra テストヘルパー（`assert_step_invariants`）は getter 経由のため変更不要
- `just check-all` 全テストパス

## 判断ログ

- `assigned_to` の Active/Completed 必須化はスコープ外とした。`activated()` の署名変更が大きく、別 Issue で検討する方が適切と判断
- `from_db()` は `expect()` / panic ではなく `Result` を選択。破損 DB データに対するグレースフルなエラーハンドリングのため
- `is_overdue()` を `self.completed_at.is_none()` から `self.completed_at().is_none()` に変更。フィールドが直接存在しなくなったため getter 経由に委譲

## 成果物

コミット:
- `d30198a` #820 Refactor WorkflowStep to ADT-based state machine

変更ファイル:
- `backend/crates/domain/src/workflow/step.rs` — ADT 型定義 + 全メソッド・テスト更新
- `backend/crates/infra/src/repository/workflow_step_repository.rs` — TryFrom のエラーハンドリング更新
- `prompts/plans/joyful-snacking-cookie.md` — 計画ファイル

PR: #840（Draft）
