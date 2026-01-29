# Phase 1: ドメインモデル拡張

## 目的

`WorkflowInstance` と `WorkflowStep` に楽観的ロック用の `version` フィールドを追加し、
承認/却下の状態遷移メソッドを実装する。

## 変更内容

### 1. WorkflowInstance

#### version フィールド追加

```rust
pub struct WorkflowInstance {
    // ... 既存フィールド ...
    version: Version,  // 楽観的ロック用
    // ...
}
```

#### 状態遷移メソッド追加

```rust
impl WorkflowInstance {
    /// ステップ承認による完了処理
    pub fn complete_with_approval(self) -> Result<Self, DomainError> {
        if self.status != WorkflowInstanceStatus::InProgress {
            return Err(DomainError::Validation(...));
        }

        Ok(Self {
            status: WorkflowInstanceStatus::Approved,
            version: self.version.next(),  // インクリメント
            completed_at: Some(Utc::now()),
            updated_at: Utc::now(),
            ..self
        })
    }

    /// ステップ却下による完了処理
    pub fn complete_with_rejection(self) -> Result<Self, DomainError> {
        // 同様の実装
    }
}
```

### 2. WorkflowStep

#### version フィールド追加

```rust
pub struct WorkflowStep {
    // ... 既存フィールド ...
    version: Version,  // 楽観的ロック用
    // ...
}
```

#### approve/reject メソッド追加

```rust
impl WorkflowStep {
    /// ステップを承認する
    pub fn approve(self, comment: Option<String>) -> Result<Self, DomainError> {
        if self.status != WorkflowStepStatus::Active {
            return Err(DomainError::Validation(...));
        }

        Ok(Self {
            status: WorkflowStepStatus::Completed,
            version: self.version.next(),
            decision: Some(StepDecision::Approved),
            comment,
            completed_at: Some(Utc::now()),
            updated_at: Utc::now(),
            ..self
        })
    }

    /// ステップを却下する
    pub fn reject(self, comment: Option<String>) -> Result<Self, DomainError> {
        // 同様の実装
    }
}
```

## 設計判断

### なぜ楽観的ロックか

| 方式 | 特徴 | 採用理由 |
|------|------|---------|
| **楽観的ロック** | 更新時にバージョンを検証 | 読み取りが多く書き込みが少ない承認フローに最適 |
| 悲観的ロック | 読み取り時からロック | 同時更新が頻繁な場合に有効だが、デッドロックのリスク |

承認フローでは「読み取り（一覧表示）が多く、書き込み（承認操作）が少ない」ため、
楽観的ロックが適している。

### なぜ version をエンティティに持たせるか

```rust
// 良い: ドメインモデルが version を所有
pub struct WorkflowInstance {
    version: Version,
}

// 悪い: リポジトリ層で version を管理
// → ドメインロジックと分離され、整合性が取りにくい
```

version はエンティティの**不変条件**の一部であり、状態遷移と一緒に管理すべき。
リポジトリ層で管理すると、ドメインロジックとの整合性が取りにくくなる。

### なぜ状態チェックをメソッド内で行うか

```rust
// 良い: メソッド内でチェック
pub fn approve(self, ...) -> Result<Self, DomainError> {
    if self.status != WorkflowStepStatus::Active {
        return Err(...);
    }
    Ok(...)
}

// 悪い: 呼び出し側でチェック
// → 呼び出し忘れのリスク、DRY 違反
```

不変条件のチェックはエンティティ自身が行うべき。
呼び出し側に依存すると、チェック漏れのリスクがある。

## TDD の実践

### テストリスト（実装前に作成）

```markdown
### WorkflowInstance
- [ ] 新規作成時に version は 1
- [ ] 承認完了でステータスが Approved になる
- [ ] 承認完了で version がインクリメントされる
- [ ] InProgress 以外で承認完了するとエラー
...

### WorkflowStep
- [ ] 新規作成時に version は 1
- [ ] approve で Completed と Approved になる
- [ ] approve で version がインクリメントされる
- [ ] Active 以外で approve するとエラー
...
```

### Red → Green → Refactor

1. **Red**: テストを先に書き、コンパイルエラーを確認
2. **Green**: テストが通る最小限の実装
3. **Refactor**: 今回は既に整理済みでスキップ

### 実行結果

```
running 15 tests
test workflow::tests::workflow_instance::test_新規作成時にversionは1 ... ok
test workflow::tests::workflow_instance::test_承認完了でステータスがApprovedになる ... ok
...
test result: ok. 15 passed; 0 failed
```

## 改善記録

Phase 1 実装中に TDD フローを無視してプロダクションコードを先に書き始めるミスがあった。

→ [改善記録: TDD フロー未遵守](../../../prompts/improvements/2026-01/2026-01-28_1936_TDDフロー未遵守.md)

対策として CLAUDE.md に以下を追記：

> **禁止:** テストを書かずにプロダクションコードを書き始めること

## 次のステップ

Phase 2: マイグレーション（DB スキーマに version カラムを追加）
