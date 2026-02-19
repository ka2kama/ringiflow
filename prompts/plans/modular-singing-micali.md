# #689 ライフサイクル系ユースケースのトランザクション化

## Context

Epic #685（ワークフローユースケースのトランザクション整合性確保）の Story #689。
submit_workflow / resubmit_workflow の複数エンティティ更新が個別トランザクションで実行されており、部分更新による不整合リスクがある。#688 で判断系ユースケースのトランザクション統合が完了しており、同じパターンを適用する。

## 対象・対象外

対象:
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle/submit.rs`
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle/resubmit.rs`
- `backend/crates/infra/tests/transaction_concurrency_test.rs`（統合テスト追加）

対象外:
- リポジトリ層の変更（#687 で対応済み、insert は TxContext を受け取る形に移行済み）
- 判断系ユースケース（#688 で対応済み）
- MockTransactionManager の変更（既存のまま）

## 設計判断

**トランザクション境界**: #688 と同じパターンを採用。すべての読み取り・ドメインロジックをトランザクション外で実行し、書き込みのみを単一トランザクションで包む。

**ステップ INSERT のループ**: batch insert メソッドは存在しない。#688 の reject/request_changes と同様に、ループ内で個別 `insert` を同一 `&mut tx` に対して呼び出す。

**統合テスト**: 重複主キーによる INSERT 失敗を利用してトランザクション原子性を検証する。

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: submit_workflow のトランザクション統合

#### 確認事項
- [x] パターン: #688 の reject_step のループパターン → `reject.rs` 内 `for (skipped_step, ...) in &skipped_steps` ループが `&mut tx` を共有
- [x] 型: `step_repo.insert` のシグネチャ → `insert(&self, tx: &mut TxContext, step: &WorkflowStep, tenant_id: &TenantId) -> Result<(), InfraError>`

#### 変更内容

`submit.rs` L137-168 を修正。現在の3つの独立トランザクション（instance update 1回 + step insert N回）を単一トランザクションに統合。

Before（概要）:
```rust
let mut tx = begin(); instance_repo.update(...); tx.commit();
for step in &steps { let mut tx = begin(); step_repo.insert(...); tx.commit(); }
```

After:
```rust
let mut tx = begin();
instance_repo.update_with_version_check(&mut tx, ...);
for step in &steps {
    step_repo.insert(&mut tx, step, &tenant_id).map_err(...)?;
}
tx.commit();
```

#### テストリスト

ユニットテスト（該当なし — 既存テストが引き続きパス）

ハンドラテスト（該当なし）

API テスト（該当なし — 既存の `submit_workflow.hurl` が引き続きパス）

E2E テスト（該当なし）

### Phase 2: resubmit_workflow のトランザクション統合

#### 確認事項: なし（Phase 1 と同一パターン）

#### 変更内容

`resubmit.rs` L149-185 を修正。Phase 1 と同一の変更。

#### テストリスト

ユニットテスト（該当なし — 既存テストが引き続きパス）

ハンドラテスト（該当なし）

API テスト（該当なし — 既存の `resubmit_workflow.hurl` が引き続きパス）

E2E テスト（該当なし）

### Phase 3: 統合テスト追加

#### 確認事項
- [x] パターン: 既存の `transaction_concurrency_test.rs` のテスト構造 → `#[sqlx::test(migrations = ...)]` + `assert_workflow_invariants` ヘルパー
- [x] パターン: 初期データ挿入 → `instance_repo.insert(&mut tx, ...)` + `step_repo.insert(&mut tx, ...)`

#### テストリスト

統合テスト（`transaction_concurrency_test.rs` に追加）:
- [x] ステップ INSERT 失敗時にインスタンス更新もロールバックされる

テスト設計:
1. セットアップ: インスタンスを DB に挿入
2. TX_A: インスタンスを InProgress に更新 + ステップを INSERT → commit（submit の正常フロー模倣）
3. TX_B: インスタンスを更に更新（TX 内では成功）→ 同じステップ ID で INSERT（主キー重複 → エラー）
4. TX_B drop → 自動ロールバック
5. 検証: インスタンスが TX_A 完了時の状態のまま（TX_B の更新がロールバック）
6. `assert_workflow_invariants` で不変条件を検証

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | step insert のエラーハンドリング文言を #688 パターンと統一すべき | 既存パターン整合 | step insert の map_err 文言を確認し、既存パターンに合わせる |
| 2回目 | ギャップなし | — | — |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | submit + resubmit の両方が対象 | OK | Phase 1, 2 で両方をカバー。統合テストを Phase 3 で追加 |
| 2 | 曖昧さ排除 | 変更箇所が具体的に特定されている | OK | submit.rs L137-168, resubmit.rs L149-185 を特定。変更パターンを Before/After で明示 |
| 3 | 設計判断の完結性 | 全判断に理由がある | OK | トランザクション境界、ループパターン、テスト手法の各判断を記載 |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | 対象・対象外セクションで明示 |
| 5 | 技術的前提 | TxContext の仕様が確認済み | OK | insert のシグネチャ、ループ内での &mut tx 共有パターンを #688 で確認済み |
| 6 | 既存ドキュメント整合 | 矛盾なし | OK | repository.md のトランザクション必須ルール、#688 の判断系パターンと整合 |

## 検証方法

```bash
# ユニットテスト
cd backend && cargo test --package ringiflow-core-service submit
cd backend && cargo test --package ringiflow-core-service resubmit

# 統合テスト
just test-rust-integration

# 全体チェック
just check-all
```
