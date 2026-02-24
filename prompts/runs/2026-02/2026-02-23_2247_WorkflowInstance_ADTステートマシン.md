# WorkflowInstance ADT ステートマシンリファクタリング

## 概要

Issue #819 に基づき、WorkflowInstance ドメインモデルを ADT ベースステートマシンパターン（ADR-054）でリファクタリングした。フラットな `status` + `Option` フィールドの構造から、状態ごとに固有フィールドを持つ enum 型の構造へ移行し、不正な状態を型レベルで防止する設計とした。

## 実施内容

### Phase 1: ドメインモデルの ADT 化

`WorkflowInstanceState` enum を導入し、7 つの状態バリアントを定義:

- `Draft`: フィールドなし（INV-I4: submitted_at IS NULL を型で強制）
- `Pending(PendingState)`: submitted_at
- `InProgress(InProgressState)`: current_step_id, submitted_at
- `Approved(CompletedState)` / `Rejected(CompletedState)`: current_step_id, submitted_at, completed_at（共有型）
- `Cancelled(CancelledState)`: current_step_id (Option), submitted_at (Option), completed_at
- `ChangesRequested(ChangesRequestedState)`: current_step_id, submitted_at

`from_db()` の戻り値を `Result<Self, DomainError>` に変更し、DB データの不変条件（INV-I1〜I9）を復元時に検証するようにした。

### Phase 2: 呼び出し元の更新

- リポジトリの `TryFrom` 実装を更新（`from_db()` が `Result` を返すため `.map_err()` を追加）
- ダッシュボードテストの `.approved()` 呼び出しを正規の遷移チェーン（`.submitted().with_current_step().complete_with_approval()`）に修正
- エンティティ影響マップに INV-I5〜I9 を追加、状態遷移図に Cancelled 遷移を追加

### シードデータ修正

ADT の不変条件検証により、既存シードデータの `in_progress`/`approved`/`rejected` インスタンスに `current_step_id` が未設定であることが検出された。修正マイグレーション `20260224000001_fix_seed_current_step_id.sql` を追加して解消。

## 判断ログ

- `CompletedState` を Approved/Rejected で共有: 遷移元が同じ（InProgress のみ）で、保持するフィールドも同一のため、別の型を定義する必要がない
- `CancelledState` に `Option` フィールドを使用: 4 つの前状態（Draft/Pending/InProgress/ChangesRequested）から遷移可能で、各前状態で利用可能なフィールドが異なるため、型レベルでの強制が困難
- シードデータ修正にマイグレーションを追加: 既存マイグレーションの直接編集ではなく、新規マイグレーションで UPDATE することで、既存開発 DB にも適用可能にした
- `approved()` / `rejected()` メソッドを削除: テスト専用の直接遷移メソッドは ADT パターンと矛盾するため、正規の遷移チェーンに統一

## 成果物

### コミット

| コミット | 内容 |
|---------|------|
| `9ff3c5d` | WIP: 空コミット（Draft PR 作成用） |
| `a8d0480` | Phase 1: ドメインモデルの ADT 化（instance.rs 全面書き換え + テスト 10 件追加） |
| `ce3ac68` | Phase 2: リポジトリ TryFrom 更新 + ダッシュボードテスト修正 |
| `8460f9e` | エンティティ影響マップ更新（INV-I5〜I9 追加、状態遷移図更新） |
| `c42f2e6` | シードデータ修正マイグレーション |

### PR

- PR #852 (Draft): https://github.com/ka2kama/ringiflow/pull/852

### 更新ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/src/workflow/instance.rs` | ADT ステートマシンへの全面リファクタリング |
| `backend/crates/infra/src/repository/workflow_instance_repository.rs` | `from_db()` の Result 対応 |
| `backend/apps/core-service/src/usecase/dashboard.rs` | テスト修正 |
| `backend/migrations/20260224000001_fix_seed_current_step_id.sql` | 新規: シードデータ修正 |
| `docs/03_詳細設計書/エンティティ影響マップ/WorkflowInstance.md` | INV 拡充、状態遷移図更新 |
