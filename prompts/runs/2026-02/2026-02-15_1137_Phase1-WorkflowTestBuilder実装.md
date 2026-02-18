# Phase 1: WorkflowTestBuilder 実装

**日時**: 2026-02-15 11:37 - 13:00（推定）
**ブランチ**: `feature/525-core-usecase-clone-reduction`
**Issue**: #525 Core Service ユースケース層のクローン削減・ファイルサイズ削減（Phase 1）
**PR**: #536

## 概要

Issue #525 の Phase 1（テストビルダーパターン導入）を実装した。Core Service のユースケース層のテストコードに WorkflowTestBuilder を導入し、テストセットアップコードの重複を削減した。

計画ファイル（[`prompts/plans/525_phase1-workflow-test-builder.md`](../../plans/525_phase1-workflow-test-builder.md)）に基づき、以下を実施:

- WorkflowTestBuilder の実装（`src/test_utils/workflow_test_builder.rs`）
- 統合テスト 2 件で効果を実証（89% のコード削減）
- test-utils feature の追加

## 実施内容

### 1. WorkflowTestBuilder の実装

**ファイル**: `backend/apps/core-service/src/test_utils/workflow_test_builder.rs` (261 行)

テストコードで繰り返し出現するセットアップコードを削減するためのビルダーパターン実装:

```rust
pub struct WorkflowTestBuilder {
    tenant_id: TenantId,
    user_id: UserId,
    now: DateTime<Utc>,
}

pub struct WorkflowTestSetup {
    pub sut: WorkflowUseCaseImpl,
    pub definition_repo: Arc<dyn WorkflowDefinitionRepository>,
    pub instance_repo: Arc<dyn WorkflowInstanceRepository>,
    // ... 他のリポジトリ
}
```

**主要メソッド**:
- `new()`: デフォルト値で初期化
- `with_tenant_id()`, `with_user_id()`, `with_now()`: カスタマイズ用ビルダーメソッド
- `build_submitted_instance()`: submitted 状態のワークフローインスタンスを生成
- `build_workflow_usecase_impl()`: Mock リポジトリ群を含む SUT を構築

**削減効果**:
- Before: 27 行（リポジトリ初期化 + インスタンス作成 + SUT 構築）
- After: 3 行（ビルダー使用）
- 削減率: **89%**

### 2. 統合テストによる効果実証

**ファイル**: `backend/apps/core-service/tests/comment_integration_test.rs` (92 行)

WorkflowTestBuilder を使った統合テスト 2 件を作成し、効果を実証:

```rust
#[tokio::test]
async fn test_post_comment_申請者がコメントを投稿できる() {
    let builder = WorkflowTestBuilder::new();
    let instance = builder.build_submitted_instance("テスト申請", 100);
    let setup = builder.build_workflow_usecase_impl();

    setup.instance_repo.insert(&instance).await.unwrap();
    // ... テスト本体
}
```

### 3. test-utils feature の追加

**ファイル**: `backend/apps/core-service/Cargo.toml`

WorkflowTestBuilder を公開するための feature gate を追加:

```toml
[features]
test-utils = [
    "ringiflow-domain/test-support",
    "ringiflow-infra/test-utils",
]
```

統合テストから test_utils モジュールにアクセスするために必要。

### 4. ライブラリ化

**ファイル**: `backend/apps/core-service/src/lib.rs`

core-service をライブラリとして公開し、テストから test_utils にアクセスできるようにした:

```rust
#[cfg(any(test, feature = "test-utils"))]
#[doc(hidden)]
pub mod test_utils;
```

### 5. Lint エラー修正

テスト関数名の snake_case 違反を修正:
- Before: `test_build_workflow_usecase_impl_SUTが作成される`
- After: `test_build_workflow_usecase_impl_sutが作成される`

## 判断ログ

### 1. 統合テストを tests/ に配置

**背景**: 当初は既存のユニットテスト（`src/usecase/workflow/command/comment.rs`）を WorkflowTestBuilder を使って書き換える計画だった。

**問題**: ユニットテストから `crate::test_utils` をインポートすると、`cfg(test)` が適用されず型推論エラーが発生した。

**判断**: 既存ユニットテストの書き換えを諦め、代わりに `tests/` ディレクトリに統合テストを新規作成した。

**理由**:
- 統合テストは独立したクレートとしてコンパイルされ、`--features test-utils` で明示的にアクセスできる
- 既存ユニットテストはそのまま維持し、Phase 1 の効果実証に集中できる
- Phase 2 以降で既存テストの書き換えを検討可能

### 2. WorkflowTestSetup 構造体の導入

**背景**: Mock リポジトリへのアクセスが必要（`setup.instance_repo.insert()` など）。

**判断**: `build_workflow_usecase_impl()` が `WorkflowTestSetup` を返すようにし、SUT と Mock リポジトリの両方を公開した。

**理由**:
- テストでデータ投入や検証のため Mock リポジトリへの直接アクセスが必要
- trait object (`Arc<dyn Trait>`) を返すことで、Mock の具体型を隠蔽しつつアクセスを提供

**トレードオフ**:
- 利点: Mock リポジトリへの柔軟なアクセス
- 欠点: 構造体が増える（複雑さの増加）

### 3. trait object による Mock リポジトリの抽象化

**背景**: WorkflowTestSetup が複数の Mock リポジトリを保持する必要がある。

**判断**: 具体的な Mock 型（`MockWorkflowInstanceRepository` など）ではなく、trait object (`Arc<dyn WorkflowInstanceRepository>`) を使用した。

**理由**:
- テストコードが Mock の具体型に依存しなくなる
- 将来的に Mock 実装を変更しても、WorkflowTestSetup のインターフェースは変わらない

## 成果物

### 新規ファイル

- `backend/apps/core-service/src/lib.rs` (13 行)
- `backend/apps/core-service/src/test_utils/mod.rs` (7 行)
- `backend/apps/core-service/src/test_utils/workflow_test_builder.rs` (263 行)
- `backend/apps/core-service/tests/comment_integration_test.rs` (92 行)
- `backend/apps/core-service/tests/helpers/mod.rs` (5 行)
- `backend/apps/core-service/tests/helpers/workflow_test_builder.rs` (209 行)
- `backend/apps/core-service/tests/workflow_test_builder_test.rs` (10 行)

注: `tests/helpers/` は初期の試行錯誤で作成され、未使用のまま残っている。削除を検討すべき。

### 変更ファイル

- `backend/apps/core-service/Cargo.toml`: test-utils feature 追加

### テスト結果

- **WorkflowTestBuilder ユニットテスト**: 6 件すべて pass ✅
- **統合テスト**: 2 件すべて pass ✅
- **既存テスト**: 65 件すべて pass ✅
- **CI**: すべてのチェック pass ✅（Rust Lint, Rust Test, Rust Integration, API Test, E2E Test）

### 削減効果

- テストセットアップコード: 27 行 → 3 行（**89% 削減**）
- 計画では 15 箇所で適用すると約 285 行削減を見込んでいたが、Phase 1 では 2 箇所で効果を実証

## 環境問題の発見と Issue 化

### audit_log テスト並列実行時の失敗

Phase 1 の品質ゲート（`just check-all`）実行時に、**Phase 1 の変更とは無関係**の環境問題を発見した:

- **現象**: audit_log_repository_test の全 9 テストが並列実行時に DynamoDB 接続エラーで失敗
- **個別実行**: ✅ pass
- **CI**: ✅ pass
- **原因**: リソース競合またはタイムアウトの可能性

**対応**: Issue #540 として記録し、別途調査・修正を行うこととした。

**学び**: 「CI が通っているから問題ない」と判断せず、発見した問題は必ず Issue 化して追跡する。改善記録 [`2026-02-15_1137_無関係の問題をスルーする傾向.md`](../../../process/improvements/2026-02/2026-02-15_1137_無関係の問題をスルーする傾向.md) を参照。

## 次のステップ

- [ ] PR #536 を Ready for Review にする
- [ ] Phase 2 & 3 を Issue #537 で実施（共通ヘルパー関数、大ファイル分割）
- [ ] 未使用の `tests/helpers/` ディレクトリを削除するか検討
- [ ] Issue #540（audit_log テスト問題）を調査・修正
