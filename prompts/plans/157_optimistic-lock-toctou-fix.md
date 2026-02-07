# Issue #157: 楽観的ロックの TOCTOU 問題を修正する

## 問題の整理

`workflow_step_repository.rs` と `workflow_instance_repository.rs` の `save` メソッドが UPSERT（`ON CONFLICT DO UPDATE`）を使用しており、WHERE 句でバージョンチェックを行っていない。

ユースケース層でバージョンチェック（`step.version() != input.version`）を行ってから `save` を呼ぶまでの間に別のリクエストが更新する可能性がある（TOCTOU 問題）。

## 方針: 新規作成と更新の分離

Issue 本文の「新規作成と更新の分離も検討」に従い、以下の方針で修正する。

1. `save`（UPSERT）を `insert`（INSERT only）に改名
2. `update_with_version_check` メソッドを新設（`UPDATE ... WHERE id = $x AND version = $expected`）
3. `rows_affected() == 0` で競合を検出し、`InfraError::Conflict` を返す

## 修正ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/src/error.rs` | `InfraError::Conflict` バリアントを追加 |
| `backend/crates/infra/src/repository/workflow_instance_repository.rs` | トレイト・実装の変更 |
| `backend/crates/infra/src/repository/workflow_step_repository.rs` | トレイト・実装の変更 |
| `backend/apps/core-service/src/usecase/workflow.rs` | `insert` / `update_with_version_check` への切り替え |
| `backend/crates/infra/tests/workflow_instance_repository_test.rs` | テスト追加・改修 |
| `backend/crates/infra/tests/workflow_step_repository_test.rs` | テスト追加・改修 |

## 実装計画（TDD）

### Phase 1: InfraError に Conflict バリアントを追加

`backend/crates/infra/src/error.rs`:

```rust
/// 楽観的ロック競合（バージョン不一致）
#[error("競合が発生しました: {entity}(id={id})")]
Conflict { entity: String, id: String },
```

### Phase 2: リポジトリに `update_with_version_check` を追加

テストリスト:

- バージョン一致で更新できる（WorkflowInstance）
- バージョン不一致で Conflict エラーを返す（WorkflowInstance）
- バージョン一致で更新できる（WorkflowStep）
- バージョン不一致で Conflict エラーを返す（WorkflowStep）

実装:

- 両トレイトに `update_with_version_check(&self, entity, expected_version: Version)` を追加
- PostgreSQL 実装: `UPDATE ... WHERE id = $x AND version = $expected`
- `rows_affected() == 0` → `InfraError::Conflict`

### Phase 3: `save` を `insert` に改名

テストリスト:

- 新規エンティティを INSERT できる（既存テストの改名）
- 重複 ID で DB エラーを返す（UPSERT しない確認）

実装:

- トレイトの `save` → `insert` に改名
- SQL から `ON CONFLICT DO UPDATE` を削除（INSERT のみ）
- 既存テストの `save` 呼び出しを `insert` / `update_with_version_check` に振り分け

### Phase 4: ユースケース層の修正

変更箇所:

| メソッド | 現在 | 修正後 |
|---------|------|--------|
| `create_workflow` | `instance_repo.save()` | `instance_repo.insert()` |
| `submit_workflow` | `instance_repo.save()` + `step_repo.save()` | `instance_repo.update_with_version_check()` + `step_repo.insert()` |
| `approve_step` | `step_repo.save()` + `instance_repo.save()` | `step_repo.update_with_version_check()` + `instance_repo.update_with_version_check()` |
| `reject_step` | `step_repo.save()` + `instance_repo.save()` | `step_repo.update_with_version_check()` + `instance_repo.update_with_version_check()` |

InfraError::Conflict → CoreError::Conflict への変換:

```rust
self.step_repo.update_with_version_check(&approved_step, expected_version).await
    .map_err(|e| match e {
        InfraError::Conflict { .. } => CoreError::Conflict(
            "ステップは既に更新されています。最新の情報を取得してください。".to_string(),
        ),
        other => CoreError::Internal(format!("ステップの保存に失敗: {}", other)),
    })?;
```

Mock リポジトリも `insert` / `update_with_version_check` に更新。

### Phase 5: sqlx-prepare と全体チェック

```bash
just sqlx-prepare
just check-all
```

## ユースケース層のバージョンチェックの扱い

リポジトリ層で DB レベルのバージョンチェックを行うが、ユースケース層の早期チェック（`step.version() != input.version`）は**残す**。

理由:

- 不要な DB アクセスを回避（早期フェイル）
- ドメイン用語でのエラーメッセージ
- 多層防御（リポジトリ層が最終防衛線）

## 検証方法

```bash
just check-all                    # リント + テスト
just test-rust-integration        # 統合テスト（DB 接続）
```
