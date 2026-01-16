---
paths:
  - "apps/core-api/migrations/**/*.sql"
  - "infra/dynamodb/**/*.tf"
  - "infra/s3/**/*.tf"
  - "**/redis/**/*.rs"
  - "**/repository/**/*.rs"
  - "**/deletion/**/*.rs"
---

# データストア変更時のルール

このルールは以下のファイルを編集する際に適用される:
- `apps/core-api/migrations/**` - PostgreSQL マイグレーション
- `infra/dynamodb/**` - DynamoDB テーブル定義
- `infra/s3/**` - S3 バケット定義
- `**/redis/**` - Redis 関連コード

## 必須チェックリスト

新しいデータストア（テーブル、バケット、キーパターン等）を追加する場合、以下を必ず確認・実施すること。

### 1. tenant_id によるデータ分離

| データストア | 要件 |
|-------------|------|
| PostgreSQL | `tenant_id` カラム必須、外部キーに `ON DELETE CASCADE` または `ON DELETE SET NULL` |
| DynamoDB | パーティションキーに `tenant_id` を含める、または GSI で `tenant_id` 検索可能に |
| S3 | パス先頭に `{tenant_id}/` を含める |
| Redis | キーパターンに `{tenant_id}` を含める |

### 2. 削除レジストリへの登録

新しいデータストアを追加したら、以下のファイルを更新:

1. **削除ハンドラの実装**
   - `apps/core-api/src/domain/tenant/deletion/` に新しい Deleter を追加
   - `TenantDeleter` トレイトを実装

2. **レジストリへの登録**
   - `apps/core-api/src/domain/tenant/deletion/registry.rs` の `DeletionRegistry::new()` に追加

3. **テストの更新**
   - `apps/core-api/tests/tenant_deletion_test.rs` の期待リストに追加
   - 統合テストで削除→検証のフローを確認

### 3. 設計書の更新

`docs/02_設計書/07_テナント退会時データ削除設計.md` の「削除対象データ一覧」セクションに追記。

## AI エージェントへの指示

新しいデータストアを追加するコードを書く場合:

1. 上記チェックリストの各項目を確認したか、ユーザーに報告すること
2. 削除ハンドラの実装も同時に行うこと（後回しにしない）
3. 不明点があればユーザーに確認すること

**禁止事項:**
- tenant_id なしでアクセスできるテーブル/バケット/キーの作成
- 削除ハンドラなしでのデータストア追加

## 参照

- 設計書: [07_テナント退会時データ削除設計.md](../../docs/02_設計書/07_テナント退会時データ削除設計.md)
- ADR: [007_テナント退会時のデータ削除方針.md](../../docs/04_ADR/007_テナント退会時のデータ削除方針.md)
