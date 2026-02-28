---
paths:
  - "backend/migrations/**/*.sql"
  - "infra/dynamodb/**/*.tf"
  - "infra/s3/**/*.tf"
  - "**/redis/**/*.rs"
  - "**/repository/**/*.rs"
  - "**/deletion/**/*.rs"
  - "infra/**/docker-compose*.yaml"
---

# データストア変更時のルール

このルールは以下のファイルを編集する際に適用される:
- `backend/migrations/**` - PostgreSQL マイグレーション
- `infra/dynamodb/**` - DynamoDB テーブル定義
- `infra/s3/**` - S3 バケット定義
- `**/redis/**` - Redis 関連コード
- `infra/**/docker-compose*.yaml` - Docker Compose 構成

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
   - `crates/infra/src/deletion/` に新しい Deleter を追加
   - `TenantDeleter` トレイトを実装

2. **レジストリへの登録**
   - `crates/infra/src/deletion/registry.rs` の `DeletionRegistry::with_all_deleters()` に追加
   - `DeletionRegistry::expected_deleter_names()` に名前を追加

3. **テストの更新**
   - `crates/infra/tests/deletion_registry_test.rs` の期待リストに追加
   - 統合テストで削除→検証のフローを確認

### 3. デプロイ環境の docker-compose 同期

新しいデータストアをローカル開発環境（`infra/docker/docker-compose.yaml`）に追加した場合、全デプロイ環境の docker-compose にも同じサービスを追加する。

対象ファイル:

| 環境 | ファイル |
|------|---------|
| ローカル開発 | `infra/docker/docker-compose.yaml` |
| API テスト | `infra/docker/docker-compose.api-test.yaml` |
| Lightsail デモ | `infra/lightsail/docker-compose.yaml` |

確認事項:
- サービス定義（イメージ、ポート、環境変数）が各環境で適切に設定されているか
- 依存サービスの環境変数（エンドポイント URL 等）がアプリケーション設定に追加されているか

### 4. 設計書の更新

`docs/40_詳細設計書/06_テナント退会時データ削除設計.md` の「削除対象データ一覧」セクションに追記。

## AI エージェントへの指示

新しいデータストアを追加するコードを書く場合:

1. 上記チェックリストの各項目を確認したか、ユーザーに報告すること
2. 削除ハンドラの実装も同時に行うこと（後回しにしない）
3. 不明点があればユーザーに確認すること

マイグレーションファイルを追加・変更した場合:

1. `just setup-db` でマイグレーションを適用すること
2. `sqlx::query!` を使用するクエリを追加・変更した場合は `cd backend && cargo sqlx prepare --workspace -- --all-targets` でキャッシュを更新すること

**禁止事項:**
- tenant_id なしでアクセスできるテーブル/バケット/キーの作成
- 削除ハンドラなしでのデータストア追加
- マイグレーション追加後に適用せずに放置すること
- ローカル開発環境の docker-compose にデータストアを追加しながら、他のデプロイ環境の docker-compose を更新しないこと

## 参照

- 設計書: [06_テナント退会時データ削除設計.md](../../docs/40_詳細設計書/06_テナント退会時データ削除設計.md)
- ADR: [007_テナント退会時のデータ削除方針.md](../../docs/70_ADR/007_テナント退会時のデータ削除方針.md)
- 改善記録: [デプロイ環境のインフラ依存追加漏れ](../../process/improvements/2026-02/2026-02-13_2221_デプロイ環境のインフラ依存追加漏れ.md)
