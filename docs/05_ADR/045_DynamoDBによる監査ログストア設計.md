# ADR-045: DynamoDB による監査ログストア設計

## ステータス

承認済み

## コンテキスト

Phase 2-2（#403）でユーザー管理・ロール管理機能を実装するにあたり、操作履歴の監査ログ基盤が必要になった。監査ログはコンプライアンス要件（RPT-004）として、テナント管理者が操作履歴を閲覧・検索できる必要がある。

監査ログの特性:
- 追記専用（Append Only）: 一度記録されたログは変更されない
- 時系列データ: 新しい順での閲覧が主なアクセスパターン
- テナント単位の分離: テナント間でデータが漏洩してはならない
- 大量蓄積: テナントあたり月数千〜数万件が想定される
- 自動削除: 一定期間（1 年）経過後に自動削除

## 検討した選択肢

### 選択肢 A: PostgreSQL（Aurora）に格納

監査ログ用テーブルを Aurora PostgreSQL に追加する。

評価:
- 利点: 既存インフラのみで完結。複雑なクエリ（JOIN、集計）が可能。RLS によるテナント分離が既に確立
- 欠点: 追記専用の時系列データに RDB は過剰。大量蓄積でパフォーマンス劣化の可能性。TTL による自動削除が組み込みでない

### 選択肢 B: DynamoDB に格納（採用）

DynamoDB テーブルに監査ログを格納する。

評価:
- 利点: 追記専用・時系列データに最適化された設計が可能。PK（tenant_id）による自然なテナント分離。TTL 属性による自動削除。スケーラビリティに優れる
- 欠点: 複雑なクエリが困難。新しいインフラコンポーネントの追加（運用コスト増）。AWS SDK の依存追加

### 選択肢 C: S3 + Athena

S3 に JSON/Parquet で格納し、Athena でクエリする。

評価:
- 利点: 大量データの低コスト保存。Athena で柔軟なクエリが可能
- 欠点: リアルタイム性が低い（バッチ投入が前提）。クエリ課金モデル。実装の複雑さ

## 決定

**選択肢 B: DynamoDB** を採用する。

理由:
1. 監査ログのアクセスパターン（テナント単位の時系列読み取り）が DynamoDB の PK + SK 設計に完全に適合する
2. TTL による自動削除は監査ログの保持期限管理に最適
3. 基本設計書で DynamoDB は Data 層に含まれており、アーキテクチャとして想定済み
4. 学習効果の観点からも、DynamoDB の実運用パターンを習得できる

## テーブル設計

### スキーマ

| 属性 | キー | 型 | 説明 |
|------|------|-----|------|
| `tenant_id` | PK | String (UUID) | テナント ID |
| `sort_key` | SK | String | `{ISO8601 timestamp}#{UUID}` |
| `id` | - | String (UUID) | 監査ログ ID |
| `actor_id` | - | String (UUID) | 操作者 ID |
| `actor_name` | - | String | 操作者名 |
| `action` | - | String | 操作種別（例: `user.create`） |
| `result` | - | String | 結果（`success` / `failure`） |
| `resource_type` | - | String | 対象リソース種別 |
| `resource_id` | - | String | 対象リソース ID |
| `detail` | - | Map | 操作の詳細情報 |
| `source_ip` | - | String | 操作元 IP |
| `ttl` | - | Number | TTL（Unix タイムスタンプ、作成から 1 年後） |
| `created_at` | - | String | 作成日時（ISO 8601） |

### アクセスパターン

| パターン | 操作 | キー条件 |
|---------|------|---------|
| テナント内の最新ログ | Query | PK = tenant_id, SK desc |
| 時間範囲フィルタ | Query | PK = tenant_id, SK BETWEEN from AND to |
| カーソルページネーション | Query | PK = tenant_id, ExclusiveStartKey = cursor |

### ページネーション

カーソルベースページネーションを採用。DynamoDB の `LastEvaluatedKey` を Base64 エンコードしてクライアントに返却する。

理由: DynamoDB はオフセットベースのページネーションをサポートしない。`LastEvaluatedKey` を使ったカーソル方式が自然かつ効率的。

## テナント分離

PK が `tenant_id` であるため、DynamoDB レベルでテナント間のデータアクセスが物理的に分離される。追加の RLS やアプリケーション層のフィルタは不要（PK の指定がクエリの必須条件であるため）。

## 実装上の判断

### BFF 直接アクセス

監査ログの記録・読み取りは BFF が DynamoDB に直接アクセスする。Core Service を経由しない。

理由:
- 監査ログの記録は BFF ハンドラの操作結果に基づいて行われる
- Core Service を経由すると、監査ログ記録のためだけに内部 API が必要になり複雑化する
- BFF は既にセッション情報（actor_id, tenant_id）を保持しているため、監査ログに必要な情報がすべて揃っている

### DynamoDB Local（開発環境）

開発・テスト環境では DynamoDB Local を Docker Compose で起動し、実際の AWS 環境と同じ SDK で操作する。テーブル作成は `ensure_audit_log_table` で冪等に行う。

## 影響

- `backend/crates/infra/` に `dynamodb.rs`（接続管理）と `repository/audit_log_repository.rs`（リポジトリ実装）を追加
- `infra` クレートに `aws-sdk-dynamodb` 依存を追加
- 開発環境の Docker Compose に DynamoDB Local を追加
- CI に DynamoDB Local サービスを追加

## 参照

- 要件: RPT-004（監査ログ）
- 機能仕様書: `docs/01_要件定義書/機能仕様書/03_監査ログ.md`
- 基本設計書: `docs/02_基本設計書/00_アーキテクチャ概要.md`（監査ログセクション）
- 削除設計: `docs/03_詳細設計書/06_テナント退会時データ削除設計.md`
- 削除レジストリ実装: #449
