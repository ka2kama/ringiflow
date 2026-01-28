# Phase 2/3: マイグレーションとリポジトリ層

## 目的

DB スキーマに version カラムを追加し、リポジトリ層で読み書きできるようにする。

## 変更内容

### 1. マイグレーション

```sql
-- backend/migrations/20260128000002_add_version_to_workflows.sql
ALTER TABLE workflow_instances
ADD COLUMN version INTEGER NOT NULL DEFAULT 1;

ALTER TABLE workflow_steps
ADD COLUMN version INTEGER NOT NULL DEFAULT 1;
```

### 2. リポジトリ層

#### WorkflowInstanceRepository

```rust
// save() に version を追加
sqlx::query!(
    r#"
    INSERT INTO workflow_instances (
        ..., version, ...
    )
    VALUES (..., $8, ...)
    ON CONFLICT (id) DO UPDATE SET
        ...,
        version = EXCLUDED.version,
        ...
    "#,
    ...,
    instance.version().as_i32(),
    ...
)
```

#### WorkflowStepRepository

同様に save/find メソッドで version を読み書き。

## 設計判断

### なぜ Phase 2 と 3 を一緒に実装したか

マイグレーションを適用すると、リポジトリ層のコードがビルドエラーになる:

```
error[E0061]: this function takes 14 arguments but 13 arguments were supplied
   --> WorkflowInstance::from_db(...)
```

sqlx のコンパイル時チェックにより、スキーマとコードの不整合が即座に検出される。
この「壊れた状態」を短時間で解消するため、Phase 2 と 3 を連続で実施した。

### なぜ楽観的ロックの検証を Phase 4 に延期したか

Phase 3 では「version の読み書き」だけを実装し、「version チェック付き更新」は Phase 4 で実装する。

理由:

1. **責務の分離**: リポジトリ層は CRUD を担当、ビジネスルール（楽観的ロック）はユースケース層
2. **テスタビリティ**: ユースケース層でモックを使いやすくするため、リポジトリはシンプルに保つ
3. **段階的な実装**: 各 Phase を小さく保ち、確実に動作確認しながら進める

```rust
// Phase 4 で追加予定のユースケース層
async fn approve_step(
    &self,
    step: WorkflowStep,
    expected_version: Version,
) -> Result<(), UseCaseError> {
    // step.version と expected_version を比較
    if step.version() != expected_version {
        return Err(UseCaseError::VersionConflict);
    }

    let updated = step.approve(None)?;
    self.step_repo.save(&updated).await?;
    Ok(())
}
```

## sqlx キャッシュの更新

SQLクエリを変更したら `just sqlx-prepare` でキャッシュを更新する必要がある。

```bash
just sqlx-prepare
# → .sqlx/ ディレクトリのファイルが更新される
```

理由: sqlx はコンパイル時にクエリを検証するため、DB スキーマの変更を反映したキャッシュが必要。

## 次のステップ

Phase 4: ユースケース層（楽観的ロック付き更新の実装）
