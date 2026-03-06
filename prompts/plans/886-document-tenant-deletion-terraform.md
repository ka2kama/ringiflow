# #886 ドキュメント管理のテナント退会削除対応と Terraform

## コンテキスト

### 目的
- Issue: #886
- Want: ドキュメント管理で追加したデータストアがテナント退会時に確実に削除される状態を保証し、S3 バケットの IaC 管理を実現する
- 完了基準:
  - テナント退会時に documents, folders テーブルが CASCADE で削除される
  - テナント退会時に S3 の `{tenant_id}/` プレフィックスが削除される
  - 削除レジストリのテストが通過する
  - S3 バケットが Terraform で定義されている

### ブランチ / PR
- ブランチ: `feature/886-document-tenant-deletion-terraform`
- PR: #1068（Draft）

### As-Is（探索結果の要約）

**既に実装済み（origin/main）:**
- `PostgresDocumentDeleter`（`backend/crates/infra/src/deletion/postgres_simple.rs`）
- `PostgresFoldersDeleter`（`backend/crates/infra/src/deletion/postgres_folders.rs`）
- `S3DocumentDeleter`（`backend/crates/infra/src/deletion/s3_document.rs`）
- `DeletionRegistry::with_all_deleters()` に全3つ登録済み
- `expected_deleter_names()` に `postgres:documents`, `postgres:folders`, `s3:documents` 含む
- `deletion_registry_test.rs` のレジストリ構造テスト通過
- 設計書（`docs/40_詳細設計書/06_テナント退会時データ削除設計.md`）に documents, folders 記載済み

**未実装:**
- `postgres_deleter_test.rs` の `delete_all` 統合テストに NotificationLog/Document/Folder Deleter が未追加
- 個別 Deleter の統合テスト（notification_logs, documents, folders）なし
- S3 バケットの Terraform 定義なし（S3 モジュール自体が存在しない）

**テーブル構造（FK 関係）:**
- `notification_logs.workflow_instance_id → workflow_instances(id) ON DELETE CASCADE`
- `notification_logs.tenant_id → tenants(id) ON DELETE CASCADE`
- `documents.workflow_instance_id → workflow_instances(id) ON DELETE CASCADE`
- `documents.folder_id → folders(id) ON DELETE RESTRICT`
- `documents.tenant_id → tenants(id) ON DELETE CASCADE`
- `folders.parent_id → folders(id) ON DELETE RESTRICT`（自己参照）
- `folders.tenant_id → tenants(id) ON DELETE CASCADE`

**Terraform 既存パターン:**
- `infra/terraform/modules/ses/` が唯一のモジュール（SES ドメイン検証）
- `infra/terraform/environments/dev/main.tf` からモジュールを呼び出す
- AWS provider `~> 5.0`、Terraform `>= 1.0, < 2.0`

### 進捗
- [ ] Phase 1: 統合テスト追加（notification_logs, documents, folders の個別テスト + delete_all 拡張）
- [ ] Phase 2: S3 バケット Terraform モジュール

## 仕様整理

### スコープ
- 対象:
  - `postgres_deleter_test.rs` への新 Deleter 統合テスト追加
  - `delete_all` 統合テストへの NotificationLog/Document/Folder Deleter 追加
  - S3 バケット `ringiflow-{env}-documents` の Terraform モジュール定義
- 対象外:
  - Deleter 実装の変更（既に完了）
  - 設計書の更新（既に完了）
  - S3 バケットポリシー（IAM ロール等は ECS タスク定義時に実装）
  - MinIO の Docker Compose 設定（別 Issue）

### 操作パス

該当なし（テスト追加と IaC 定義のため、ユーザー操作パスは存在しない）

## 設計

### 設計判断

| # | 判断 | 選択肢 | 選定理由 | 状態 |
|---|------|--------|---------|------|
| 1 | 統合テストでの documents 挿入方法 | A: folder_id コンテキストで挿入 / B: workflow_instance_id コンテキストで挿入 | B: workflow_instance_id コンテキスト。delete_all テストは既にワークフローデータを作成しており、documents の XOR 制約（folder_id XOR workflow_instance_id）に適合する。folder は別途個別テストで検証 | 確定 |
| 2 | S3 Terraform のライフサイクルルール | A: なし / B: バージョニング + ライフサイクル | A: 初期実装はシンプルに。バージョニングやライフサイクルは運用フェーズで追加（YAGNI）| 確定 |
| 3 | S3 バケットの暗号化 | A: SSE-S3 / B: SSE-KMS | A: SSE-S3（AES-256）。初期段階では AWS 管理キーで十分。テナント単位のキー管理は将来検討 | 確定 |

### Phase 1: 統合テスト追加

#### 確認事項
- 型: `PostgresNotificationLogDeleter`, `PostgresDocumentDeleter`, `PostgresFoldersDeleter` → `backend/crates/infra/src/deletion/mod.rs` の re-export
- パターン: `postgres_deleter_test.rs` の既存テストパターン（`assert_count_delete_count` ヘルパー）
- パターン: `common/mod.rs` のヘルパー関数（`setup_test_data` が返す `(TenantId, UserId)`）

#### テストリスト

ユニットテスト（該当なし — 既存の単体テストで網羅済み）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

統合テスト（`postgres_deleter_test.rs`）:
- [ ] `PostgresNotificationLogDeleter`: count と delete が正しく動作する
- [ ] `PostgresDocumentDeleter`: count と delete が正しく動作する（workflow_instance_id コンテキスト）
- [ ] `PostgresFoldersDeleter`: count と delete が正しく動作する（depth 降順削除の検証含む）
- [ ] `delete_all` 統合テスト拡張: notification_logs, documents, folders を含めて FK 制約に違反せず完了する

### Phase 2: S3 バケット Terraform モジュール

#### 確認事項
- パターン: `infra/terraform/modules/ses/` の構造（main.tf, variables.tf, outputs.tf）
- パターン: `infra/terraform/environments/dev/main.tf` のモジュール呼び出し

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証:
- [ ] `terraform fmt -check` が通過する
- [ ] `terraform validate` が通過する（可能であれば）

## ブラッシュアップ

### ギャップ発見の観点 進行状態

| 観点 | 状態 | メモ |
|------|------|------|
| 未定義 | 完了 | 全参照を確認、未定義なし |
| 曖昧 | 完了 | documents の挿入コンテキストを明確化（設計判断 #1） |
| 競合・エッジケース | 完了 | documents の XOR 制約、folders の自己参照 FK を考慮 |
| 不完全なパス | 完了 | 操作パス該当なし（テスト + IaC） |
| アーキテクチャ不整合 | 完了 | 既存の Terraform モジュールパターンに準拠 |
| 責務の蓄積 | 完了 | 既存テストファイルへの追加のみ、新モジュールは Terraform のみ |
| 既存手段の見落とし | 完了 | `assert_count_delete_count` ヘルパーを再利用 |
| テスト層網羅漏れ | 完了 | 統合テスト層のみ。ユニットテストは既存で網羅済み |
| 操作パス網羅漏れ | 完了 | 該当なし |
| テスト責任の断絶 | 完了 | 本 Story で統合テストの責任を完結 |
| セキュリティ境界の欠落 | 完了 | S3 バケットの暗号化を SSE-S3 で設定、パブリックアクセスブロック有効化 |

### ループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | documents の挿入時に XOR 制約を考慮する必要がある | 競合・エッジケース | 設計判断 #1 で workflow_instance_id コンテキストを選択 |
| 1回目 | S3 バケットのセキュリティ設定が未記載 | セキュリティ境界の欠落 | SSE-S3 暗号化 + パブリックアクセスブロックを追加 |

### 未解決の問い
なし

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 統合テスト（4テスト）+ Terraform モジュール。Deleter 実装は As-Is で完了確認済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | documents 挿入コンテキスト、S3 暗号化方式を確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 3つの設計判断がすべて確定 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外に IAM ポリシー、MinIO Docker 設定を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | FK 制約の削除順序、XOR 制約、自己参照 FK を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 設計書（06, 17）と矛盾なし |
