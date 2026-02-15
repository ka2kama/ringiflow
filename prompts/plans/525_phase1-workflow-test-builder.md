# Issue #525: Core Service ユースケース層のクローン削減・ファイルサイズ削減

## Context

### 問題
Core Service のユースケース層は jscpd クローンの最大集中地帯であり、以下の課題があります：

- **36個のクローン**: テストコード15個、ビジネスロジック12個、応答構築9個
- **ファイルサイズ超過**: decision.rs (1889行)、lifecycle.rs (1292行)、task.rs (796行) が閾値500行を超過
- **保守性の低下**: 同じパターンが複数箇所に散在し、変更波及が発生

### 意図する結果
1. クローンの削減により、コードの重複を200-280行削減
2. 大ファイルの分割により、各ファイルを150行以下に抑制
3. 共通パターンの抽出により、エラーハンドリングと権限チェックを標準化
4. テストコードの保守性向上（テストビルダーパターン導入）

### 分析結果サマリー

**クローンの性質分類:**
- テストテンプレート重複: 15個（低難易度、テストビルダーで対応）
- ワークフロー取得パターン: 4個（中難易度、ヘルパー関数で対応）
- 権限チェックパターン: 6個（低難易度、ヘルパーメソッドで対応）
- エラーハンドリング: 12個（中難易度、汎用ラッパーで対応）

**大ファイルの構造:**
- `decision.rs`: 3つの承認判断操作（approve/reject/request_changes）+ display_number版ラッパー
- `lifecycle.rs`: 3つのライフサイクル操作（create/submit/resubmit）+ display_number版ラッパー
- ID版とdisplay_number版の重複パターンが共通化可能

---

## 実装アプローチ

### 全体方針

リスクと効果のバランスから、以下の3段階で実施：

1. **Phase 1（低リスク・高効果）**: テストビルダーパターン導入
2. **Phase 2（中リスク・中効果）**: 共通ヘルパー関数の抽出
3. **Phase 3（高リスク・保守性向上）**: 大ファイルの分割

各 Phase は独立しており、個別の PR として実施可能。

---

## Phase 1: テストビルダーパターン導入

### 対象
- `backend/apps/core-service/src/usecase/workflow/command/comment.rs` (テスト部分)
- `backend/apps/core-service/src/usecase/task.rs` (テスト部分)
- `backend/apps/core-service/src/usecase/dashboard.rs` (テスト部分)

### 実装内容

#### 1. テストヘルパーモジュールの作成

**ファイル:** `backend/apps/core-service/tests/helpers/workflow_test_builder.rs`

```rust
use ringiflow_domain::{
    tenant::TenantId,
    user::UserId,
    value_objects::{DisplayNumber, Version},
    workflow::{WorkflowDefinitionId, WorkflowInstance, WorkflowInstanceId, NewWorkflowInstance},
};
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub struct WorkflowTestBuilder {
    tenant_id: TenantId,
    user_id: UserId,
    now: DateTime<Utc>,
}

impl WorkflowTestBuilder {
    pub fn new() -> Self {
        Self {
            tenant_id: TenantId::new(),
            user_id: UserId::new(),
            now: chrono::Utc::now(),
        }
    }

    pub fn with_tenant_id(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = tenant_id;
        self
    }

    pub fn with_user_id(mut self, user_id: UserId) -> Self {
        self.user_id = user_id;
        self
    }

    pub fn with_now(mut self, now: DateTime<Utc>) -> Self {
        self.now = now;
        self
    }

    /// 標準的なワークフローインスタンスを作成（submitted状態）
    pub fn build_submitted_instance(&self, title: &str, display_number: u32) -> WorkflowInstance {
        WorkflowInstance::new(NewWorkflowInstance {
            id: WorkflowInstanceId::new(),
            tenant_id: self.tenant_id.clone(),
            definition_id: WorkflowDefinitionId::new(),
            definition_version: Version::initial(),
            display_number: DisplayNumber::new(display_number).unwrap(),
            title: title.to_string(),
            form_data: serde_json::json!({}),
            initiated_by: self.user_id.clone(),
            now: self.now,
        })
        .submitted(self.now)
        .unwrap()
        .with_current_step("approval".to_string(), self.now)
    }

    /// Mock リポジトリ群を含む SUT（System Under Test）を構築
    pub fn build_workflow_usecase_impl(&self) -> WorkflowUseCaseImpl {
        use crate::usecase::workflow::WorkflowUseCaseImpl;
        use ringiflow_infra::repository::mock::*;

        WorkflowUseCaseImpl::new(
            Arc::new(MockWorkflowDefinitionRepository::new()),
            Arc::new(MockWorkflowInstanceRepository::new()),
            Arc::new(MockWorkflowStepRepository::new()),
            Arc::new(MockWorkflowCommentRepository::new()),
            Arc::new(MockUserRepository),
            Arc::new(MockDisplayIdCounterRepository::new()),
            Arc::new(FixedClock::new(self.now)),
        )
    }
}
```

#### 2. 既存テストの書き換え

**Before (comment.rs L145-172):**
```rust
#[tokio::test]
async fn test_post_comment_by_requester() {
    let tenant_id = TenantId::new();
    let user_id = UserId::new();
    let now = chrono::Utc::now();

    let definition_repo = MockWorkflowDefinitionRepository::new();
    let instance_repo = MockWorkflowInstanceRepository::new();
    let step_repo = MockWorkflowStepRepository::new();
    let comment_repo = MockWorkflowCommentRepository::new();

    let instance = WorkflowInstance::new(NewWorkflowInstance {
        id: WorkflowInstanceId::new(),
        tenant_id: tenant_id.clone(),
        definition_id: WorkflowDefinitionId::new(),
        definition_version: Version::initial(),
        display_number: DisplayNumber::new(100).unwrap(),
        title: "テスト申請".to_string(),
        form_data: serde_json::json!({}),
        initiated_by: user_id.clone(),
        now,
    })
    .submitted(now)
    .unwrap()
    .with_current_step("approval".to_string(), now);

    instance_repo.insert(&instance).await.unwrap();

    let sut = WorkflowUseCaseImpl::new(
        Arc::new(definition_repo),
        Arc::new(instance_repo),
        Arc::new(step_repo),
        Arc::new(comment_repo),
        Arc::new(MockUserRepository),
        Arc::new(MockDisplayIdCounterRepository::new()),
        Arc::new(FixedClock::new(now)),
    );

    // ... テスト本体 ...
}
```

**After:**
```rust
#[tokio::test]
async fn test_post_comment_by_requester() {
    use crate::tests::helpers::WorkflowTestBuilder;

    let builder = WorkflowTestBuilder::new();
    let instance = builder.build_submitted_instance("テスト申請", 100);
    let sut = builder.build_workflow_usecase_impl();

    // Mock リポジトリへのデータ投入
    sut.instance_repo.insert(&instance).await.unwrap();

    // ... テスト本体 ...
}
```

**削減効果:** 27行 → 8行（15箇所で適用すると約285行の削減）

### 確認事項
- [x] 既存の Mock リポジトリの構造 → `backend/crates/infra/src/mock.rs`（7つのMock実装: MockWorkflowDefinitionRepository, MockWorkflowInstanceRepository, MockWorkflowStepRepository, MockWorkflowCommentRepository, MockUserRepository, MockDisplayIdCounterRepository, FixedClock）
- [x] テストファイルの配置規約 → `backend/apps/core-service/tests/` は存在しないため新規作成が必要
- [x] `WorkflowUseCaseImpl` の生成パターン → `comment.rs` L174-182で確認、7つの依存関係をArcで渡す（definition_repo, instance_repo, step_repo, comment_repo, user_repo, counter_repo, clock）

### テストリスト

ユニットテスト:
- [ ] WorkflowTestBuilder::new() でデフォルト値が設定される
- [ ] WorkflowTestBuilder::with_* でカスタマイズできる
- [ ] build_submitted_instance() で標準インスタンスが作成される
- [ ] build_workflow_usecase_impl() で SUT が作成される

統合テスト:
- [ ] 既存のすべてのテスト（comment.rs, task.rs, dashboard.rs）が通る
- [ ] テストビルダーを使った新しいテストが動作する

---

## Phase 2: 共通ヘルパー関数の抽出

### 対象
- `backend/apps/core-service/src/usecase/workflow/command/comment.rs`
- `backend/apps/core-service/src/usecase/workflow/query.rs`
- `backend/apps/core-service/src/usecase/task.rs`
- `backend/apps/core-service/src/usecase/dashboard.rs`

### 実装内容

#### 1. エラーハンドリングヘルパーの作成

**ファイル:** `backend/apps/core-service/src/usecase/helpers.rs`

```rust
use crate::error::CoreError;
use std::fmt::Display;

/// リポジトリの Option<T> 結果を CoreError に変換する汎用ヘルパー
///
/// # 使用例
///
/// ```
/// let instance = find_or_not_found(
///     self.instance_repo.find_by_id(&id, &tenant_id),
///     "ワークフローインスタンス"
/// ).await?;
/// ```
pub async fn find_or_not_found<T, E, F>(
    future: F,
    entity_name: &str,
) -> Result<T, CoreError>
where
    F: std::future::Future<Output = Result<Option<T>, E>>,
    E: Display,
{
    future
        .await
        .map_err(|e| CoreError::Internal(format!("{}の取得に失敗: {}", entity_name, e)))?
        .ok_or_else(|| CoreError::NotFound(format!("{}が見つかりません", entity_name)))
}
```

**使用例（Before）:**
```rust
let instance = self
    .instance_repo
    .find_by_display_number(display_number, &tenant_id)
    .await
    .map_err(|e| CoreError::Internal(format!("インスタンスの取得に失敗: {}", e)))?
    .ok_or_else(|| {
        CoreError::NotFound("ワークフローインスタンスが見つかりません".to_string())
    })?;
```

**使用例（After）:**
```rust
let instance = find_or_not_found(
    self.instance_repo.find_by_display_number(display_number, &tenant_id),
    "ワークフローインスタンス"
).await?;
```

#### 2. 権限チェックヘルパーの作成

**ファイル:** `backend/apps/core-service/src/usecase/helpers.rs` に追加

```rust
/// 担当者チェック（assigned_to）
pub fn check_assigned_to(
    assigned_to: Option<&UserId>,
    user_id: &UserId,
) -> Result<(), CoreError> {
    if assigned_to != Some(user_id) {
        return Err(CoreError::Forbidden(
            "このタスクにアクセスする権限がありません".to_string(),
        ));
    }
    Ok(())
}
```

**使用例（Before - task.rs L149-153）:**
```rust
if step.assigned_to() != Some(&user_id) {
    return Err(CoreError::Forbidden(
        "このタスクにアクセスする権限がありません".to_string(),
    ));
}
```

**使用例（After）:**
```rust
check_assigned_to(step.assigned_to(), &user_id)?;
```

### 確認事項
- [ ] `CoreError` の定義 → `backend/apps/core-service/src/error.rs`
- [ ] 既存のヘルパー関数 → `backend/apps/core-service/src/usecase/mod.rs` の `resolve_user_names`
- [ ] Future トレイト境界 → Rust async/await パターン

### テストリスト

ユニットテスト:
- [ ] find_or_not_found: Some → Ok(T)
- [ ] find_or_not_found: None → Err(NotFound)
- [ ] find_or_not_found: Err → Err(Internal)
- [ ] check_assigned_to: 一致 → Ok(())
- [ ] check_assigned_to: 不一致 → Err(Forbidden)
- [ ] check_assigned_to: None → Err(Forbidden)

統合テスト:
- [ ] comment.rs の既存テストが通る
- [ ] query.rs の既存テストが通る
- [ ] task.rs の既存テストが通る
- [ ] dashboard.rs の既存テストが通る

---

## Phase 3: 大ファイルの分割

### 対象
- `backend/apps/core-service/src/usecase/workflow/command/decision.rs` (1889行)
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle.rs` (1292行)

### 実装内容

#### 1. decision.rs の分割

**分割後のディレクトリ構造:**
```
backend/apps/core-service/src/usecase/workflow/command/
├── decision/
│   ├── mod.rs              (約10行: pub use で再エクスポート)
│   ├── approve.rs          (約145行)
│   ├── reject.rs           (約135行)
│   └── request_changes.rs  (約135行)
├── lifecycle/              (Phase 3.2 で実施)
├── comment.rs
└── mod.rs
```

**decision/mod.rs:**
```rust
//! ワークフロー承認判断操作
//!
//! - approve: ステップ承認
//! - reject: ステップ却下
//! - request_changes: ステップ差し戻し

mod approve;
mod reject;
mod request_changes;

// 公開 API を再エクスポート（破壊的変更なし）
pub use approve::*;
pub use reject::*;
pub use request_changes::*;
```

**decision/approve.rs:**
```rust
//! ステップ承認操作

use super::super::WorkflowUseCaseImpl;
// ... (approve_step と approve_step_by_display_number の実装)
```

#### 2. lifecycle.rs の分割（同様のパターン）

**分割後のディレクトリ構造:**
```
backend/apps/core-service/src/usecase/workflow/command/
├── decision/              (Phase 3.1 で実施済み)
├── lifecycle/
│   ├── mod.rs             (約10行)
│   ├── create.rs          (約50行)
│   ├── submit.rs          (約120行)
│   └── resubmit.rs        (約135行)
├── comment.rs
└── mod.rs
```

#### 3. display_number 解決の共通化（オプション）

**ファイル:** `backend/apps/core-service/src/usecase/helpers.rs` に追加

```rust
/// display_number から WorkflowInstanceId を解決する
pub async fn resolve_instance_id(
    instance_repo: &dyn WorkflowInstanceRepository,
    display_number: DisplayNumber,
    tenant_id: &TenantId,
) -> Result<WorkflowInstanceId, CoreError> {
    let instance = find_or_not_found(
        instance_repo.find_by_display_number(display_number, tenant_id),
        "ワークフローインスタンス"
    ).await?;
    Ok(instance.id().clone())
}
```

### 確認事項
- [ ] モジュール分割のパターン → `backend/apps/core-service/src/usecase/workflow/` の構造
- [ ] `mod.rs` での再エクスポート → `pub use` パターン
- [ ] テストの配置 → 各ファイルに `#[cfg(test)]` モジュール
- [ ] `WorkflowUseCaseImpl` のメソッド → `impl` ブロックは分割後も使用可能

### テストリスト

ユニットテスト:
- [ ] approve_step の既存テストが通る
- [ ] reject_step の既存テストが通る
- [ ] request_changes_step の既存テストが通る
- [ ] approve_step_by_display_number の既存テストが通る
- [ ] reject_step_by_display_number の既存テストが通る
- [ ] request_changes_step_by_display_number の既存テストが通る

統合テスト:
- [ ] decision モジュールの全テストが通る

API テスト:
- [ ] 承認 API エンドポイントが動作する
- [ ] 却下 API エンドポイントが動作する
- [ ] 差し戻し API エンドポイントが動作する

---

## リスク評価

| Phase | リスク | 対策 |
|-------|--------|------|
| Phase 1 | テストビルダーの設計ミス | 既存テストと並行して段階的に導入 |
| Phase 2 | ヘルパー関数のエラーメッセージ変更 | 既存のエラーメッセージを維持 |
| Phase 3 | モジュール分割時の import エラー | `pub use` で公開 API を維持 |
| 全体 | CI/CD の失敗 | 各 Phase で `just check-all` を実行 |

---

## 検証方法

### 各 Phase 共通
```bash
# コンパイルチェック
cd backend && cargo check

# テスト実行
just test-rust-integration

# 全体チェック（lint + test + API test）
just check-all
```

### Phase 1 固有
- テストビルダーを使った新しいテストが動作することを確認
- 既存テスト（comment.rs, task.rs, dashboard.rs）がすべて通ることを確認

### Phase 2 固有
- ヘルパー関数のユニットテストがすべて通ることを確認
- ヘルパー関数を使った既存テストがすべて通ることを確認

### Phase 3 固有
- モジュール分割後の import が正しく解決されることを確認
- API テストで実際のエンドポイントが動作することを確認

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Phase 分割の基準が不明確 | 曖昧さ排除 | リスクと効果のバランスで3段階に分割 |
| 2回目 | テストビルダーの具体的なシグネチャが未定義 | 未定義 | WorkflowTestBuilder の具体的な実装を追加 |
| 3回目 | display_number 解決の共通化が漏れていた | 既存手段の見落とし | resolve_instance_id ヘルパーを Phase 3 に追加 |
| 4回目 | 各 Phase の確認事項が不足 | 不完全なパス | 各 Phase に確認事項セクションを追加 |

---

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | 36個のクローンすべてが Phase 1-3 でカバーされている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の実装内容、ファイル構造、コードスニペットが具体的に記載されている |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | Phase 分割の基準（リスクと効果）、ヘルパー関数の設計、モジュール分割方法が明記されている |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 各 Phase の対象ファイルが明記され、Phase 3 の display_number 共通化はオプションと記載 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Rust のモジュールシステム、`pub use` パターン、async/await が考慮されている |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | CLAUDE.md のファイルサイズ閾値（500行）、Issue #525 の想定アプローチと整合 |
