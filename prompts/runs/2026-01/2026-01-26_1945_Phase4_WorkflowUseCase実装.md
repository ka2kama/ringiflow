# Phase 4: WorkflowUseCase 実装セッション

## 概要

Issue #35 - Phase 4: ワークフロー作成ユースケースを実装した。

## 実装内容

### 1. ユースケース層の構築

**ファイル作成:**

- `backend/apps/core-service/src/usecase.rs`
  - ワークフローユースケーストレイトを定義
  - auth-service のパターンに倣ったトレイトベース設計

- `backend/apps/core-service/src/usecase/workflow.rs`
  - `WorkflowUseCaseImpl` を実装
  - `CreateWorkflowInput` と `SubmitWorkflowInput` を定義
  - Mock リポジトリを使ったユニットテストを実装

- `backend/apps/core-service/src/error.rs`
  - `CoreError` を定義
  - RFC 7807 Problem Details 形式のエラーレスポンス変換を実装

### 2. 実装したユースケース

#### CreateWorkflowUseCase

ワークフローインスタンスを下書き (draft) として作成する。

**処理フロー:**

1. ワークフロー定義が存在するか確認
2. 公開済み (published) であるか確認
3. WorkflowInstance を draft として作成
4. リポジトリに保存

**検証:**

- 定義の存在チェック
- 公開状態のチェック

#### SubmitWorkflowUseCase

ワークフローを申請し、ステップを作成する。

**処理フロー:**

1. ワークフローインスタンスが存在するか確認
2. draft 状態であるか確認
3. ワークフロー定義を取得
4. ステップを作成 (MVP では1段階承認のみ)
5. ステップを active に設定
6. ワークフローインスタンスを pending → in_progress に遷移
7. インスタンスとステップをリポジトリに保存

**検証:**

- インスタンスの存在チェック
- draft 状態のチェック

### 3. 依存関係の追加

`backend/apps/core-service/Cargo.toml` に `async-trait` を追加。

### 4. テスト

3つのユニットテストを実装:

- `test_create_workflow_正常系`
- `test_create_workflow_定義が見つからない`
- `test_submit_workflow_正常系`

すべてのテストがパス。

### 5. ドキュメント

実装解説ドキュメントを作成:

- `docs/07_実装解説/04_ワークフロー申請機能/04_Phase4_WorkflowUseCase.md`

## 設計判断

### 1. トレイトベース設計の採用

auth-service の `AuthUseCase` トレイトと同じパターンを採用。

**理由:**

- テスタビリティの向上（Mock 注入が容易）
- 依存性の逆転（ユースケース層がトレイトに依存）
- 将来の拡張性（別の実装への切り替えが容易）

### 2. 不変エンティティによる状態管理

状態遷移は新しいインスタンスを返すパターンを採用。

**理由:**

- 型による状態保証
- ビジネスルールの局所化
- 履歴が残る（状態変更が明示的）

### 3. MVP スコープの明確化

Phase 4 では固定の1段階承認のみを実装。

**理由:**

- MVP スコープを最小限に保つ
- 将来の拡張ポイントを明示（コメント + 未使用の _definition）
- 段階的な実装（Phase 5 で複数ステップ対応）

### 4. Mock リポジトリによるユニットテスト

メモリ内リポジトリでユースケースをテスト。

**理由:**

- 高速なテスト実行（DB 接続不要）
- 環境に依存しない
- ビジネスロジックのテストに集中

## 検証

```bash
just check-all
```

すべてのチェックがパス:

- フォーマット
- リント
- ユニットテスト (8 tests)
- 統合テスト
- フロントエンドテスト

## Issue 更新

Issue #35 の Phase 4 チェックボックスを更新:

```
- [x] Phase 4: ワークフロー作成ユースケースを実装
```

## 次のステップ

Phase 5: ワークフロー申請ユースケースの拡張

- 複数ステップの順次承認
- 差し戻し機能
- 承認/却下ユースケース

## 参照

- Issue: [#35 ワークフロー申請機能](https://github.com/ka2kama/ringiflow/issues/35)
- 実装解説: Phase 4: WorkflowUseCase（統合後のファイル: [02_ワークフロー申請_コード解説.md](../../../docs/07_実装解説/05_ワークフロー申請機能/02_ワークフロー申請_コード解説.md)）
- API設計: [docs/03_詳細設計書/03_API設計.md](../../../docs/03_詳細設計書/03_API設計.md)
