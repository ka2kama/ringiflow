# Phase 4: ユースケース層

## 目的

承認/却下のビジネスロジックを実装し、楽観的ロックによる競合検出を行う。

## 変更内容

### 1. エラー型の追加

```rust
// apps/core-service/src/error.rs
pub enum CoreError {
    // ... 既存 ...
    Forbidden(String),  // 403: 権限不足
    Conflict(String),   // 409: 楽観的ロック失敗
}
```

### 2. approve_step / reject_step メソッド

```rust
pub async fn approve_step(
    &self,
    input: ApproveRejectInput,
    step_id: WorkflowStepId,
    tenant_id: TenantId,
    user_id: UserId,
) -> Result<(), CoreError> {
    // 1. ステップを取得
    let step = self.step_repo.find_by_id(&step_id, &tenant_id).await?
        .ok_or_else(|| CoreError::NotFound(...))?;

    // 2. 権限チェック
    if step.assigned_to() != Some(&user_id) {
        return Err(CoreError::Forbidden(...));
    }

    // 3. 楽観的ロック
    if step.version() != input.version {
        return Err(CoreError::Conflict(...));
    }

    // 4. ステップを承認
    let approved_step = step.approve(input.comment)?;

    // 5. インスタンスを完了に遷移
    let instance = self.instance_repo.find_by_id(...).await?...;
    let completed_instance = instance.complete_with_approval()?;

    // 6. 保存
    self.step_repo.save(&approved_step).await?;
    self.instance_repo.save(&completed_instance).await?;

    Ok(())
}
```

## 設計判断

### なぜユースケース層で楽観的ロックを検証するか

```rust
// 良い: ユースケース層で検証
if step.version() != input.version {
    return Err(CoreError::Conflict(...));
}

// 悪い: リポジトリ層で検証
// → ビジネスルールがリポジトリに漏れる
```

楽観的ロックは「同時更新を防ぐ」というビジネスルール。
リポジトリ層は CRUD に徹し、ビジネスルールはユースケース層で表現する。

### なぜステップとインスタンスを別々に保存するか

```rust
// 現在の実装（MVP）
self.step_repo.save(&approved_step).await?;
self.instance_repo.save(&completed_instance).await?;

// 将来的にはトランザクションで包む
// tx.step_repo.save(&approved_step).await?;
// tx.instance_repo.save(&completed_instance).await?;
// tx.commit().await?;
```

MVP では 2 つの save を別々に呼んでいる。
本来はトランザクションで包むべきだが、以下の理由で延期:

1. トランザクション管理の抽象化が必要（別 Issue）
2. 単一テナント・単一ユーザーの MVP では競合リスクが低い
3. 楽観的ロックで競合を検出できる

### TDD 実践

テストリスト:
1. 正常系: 承認/却下が成功する
2. 権限エラー: 担当者以外は 403
3. 状態エラー: Active 以外は 400
4. 競合エラー: バージョン不一致は 409

各テストケースで Mock リポジトリを使用し、ユースケース層のロジックを isolated にテスト。

## 次のステップ

Phase 5: API 層（Core Service と BFF のエンドポイント実装）
