# Story 1: スキーマ準備（tenant_id カラム追加 + RLS ポリシー設定）

## 概要

全テナントスコープテーブルに PostgreSQL RLS を導入し、DB レベルのテナント分離を確立する基盤を構築した。

### 対応 Issue

- #407（Story 1: スキーマ準備）
- Epic: #402（Phase 2-1: マルチテナント RLS）

### 設計書との対応

- [基本設計書: インフラと DB 設計 7.1.3 節](../../02_基本設計書/03_インフラとDB設計.md) — 二重防御（アプリ層 + DB 層）

## 実装したコンポーネント

### マイグレーション

| ファイル | 責務 |
|---------|------|
| [`20260210000001_add_tenant_id_to_workflow_steps.sql`](../../../backend/migrations/20260210000001_add_tenant_id_to_workflow_steps.sql) | workflow_steps に tenant_id カラム追加 |
| [`20260210000002_add_tenant_id_to_user_roles.sql`](../../../backend/migrations/20260210000002_add_tenant_id_to_user_roles.sql) | user_roles に tenant_id カラム追加 |
| [`20260210000003_enable_rls_policies.sql`](../../../backend/migrations/20260210000003_enable_rls_policies.sql) | RLS 有効化 + ポリシー作成 + アプリロール |

### 更新したリポジトリ

| ファイル | 変更内容 |
|---------|---------|
| [`workflow_step_repository.rs`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs) | `insert` に `tenant_id` パラメータ追加 |

## 実装内容

### 1. カラム追加マイグレーション

既存テーブルに NOT NULL カラムを追加する安全なパターン:

```sql
-- 1. カラムを NULL 許容で追加
ALTER TABLE workflow_steps ADD COLUMN tenant_id UUID;

-- 2. 既存データをバックフィル（親テーブルから）
UPDATE workflow_steps ws
SET tenant_id = wi.tenant_id
FROM workflow_instances wi
WHERE ws.instance_id = wi.id;

-- 3. NOT NULL 制約を設定
ALTER TABLE workflow_steps ALTER COLUMN tenant_id SET NOT NULL;

-- 4. FK + インデックス追加
ALTER TABLE workflow_steps
    ADD CONSTRAINT workflow_steps_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE;
CREATE INDEX workflow_steps_tenant_id_idx ON workflow_steps(tenant_id);
```

この順序により、既存データの整合性を保ちながら NOT NULL カラムを安全に追加できる。

### 2. RLS ポリシー

全 9 テナントスコープテーブルに統一パターンのポリシーを設定:

```sql
CREATE POLICY tenant_isolation ON <table>
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);
```

特殊ケース:
- `tenants` テーブル: `id` で比較（`tenant_id` カラムではなく主キー）
- `roles` テーブル: `OR tenant_id IS NULL`（システムロールは全テナントから参照可能）

### 3. アプリケーションロール

```sql
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'ringiflow_app') THEN
        CREATE ROLE ringiflow_app LOGIN;
    END IF;
END
$$;
```

superuser は `BYPASSRLS` 権限を持つため RLS をバイパスする。アプリケーションからの接続には非 superuser の `ringiflow_app` ロールを使用することで RLS が適用される。

## テスト

既存の統合テスト全 48 テストがパス:

```bash
cd backend && cargo test --all-features -p ringiflow-infra \
    --test workflow_step_repository_test \
    --test user_repository_test \
    --test workflow_instance_repository_test \
    --test workflow_definition_repository_test \
    --test tenant_repository_test \
    --test display_id_counter_repository_test
```

`#[sqlx::test]` は superuser で接続するため RLS をバイパスし、既存テストはそのまま動作する。RLS 動作の検証は Story 3（#409）のスコープ。

## 設計解説

### 1. 非正規化（tenant_id カラム追加）の選択

**場所**: マイグレーション `20260210000001`, `20260210000002`

**なぜこの設計か**:

RLS ポリシーで行フィルタを行う際、JOIN ベースのポリシーは PostgreSQL が全行に対して JOIN を評価するため、パフォーマンスが大幅に劣化する。PostgreSQL 公式ドキュメントでも JOIN ベースのポリシーは非推奨とされている。

**代替案**:

| 方式 | メリット | デメリット |
|------|---------|----------|
| **カラム追加（採用）** | 単純なポリシー、高パフォーマンス | データの冗長性（親子間で tenant_id が重複） |
| JOIN ベースポリシー | 正規化を維持 | 全行評価でパフォーマンス劣化、PostgreSQL 公式非推奨 |

冗長性のデメリットは、FK 制約と CASCADE 削除で整合性を担保できるため、パフォーマンスの利点が上回る。

### 2. NULLIF パターンによる安全設計

**場所**: マイグレーション `20260210000003`

```sql
NULLIF(current_setting('app.tenant_id', true), '')::UUID
```

**なぜこの設計か**:

`current_setting('app.tenant_id', true)` は `app.tenant_id` が未設定時に空文字列を返す。`''::UUID` は PostgreSQL でキャストエラーになるため、`NULLIF` で空文字列を `NULL` に変換する。`tenant_id = NULL` は SQL の三値論理で常に `false` となり、未設定時はどの行もマッチしない。

この「フェイルセーフ」設計により、アプリケーション側でテナントコンテキストの設定を忘れた場合でもデータ漏洩が発生しない。

## 関連ドキュメント

- [計画ファイル](../../../prompts/plans/snoopy-prancing-abelson.md)（リネーム予定）
- [ナレッジベース: PostgreSQL RLS](../../06_ナレッジベース/infra/PostgreSQL_RLS.md)
- [基本設計書: インフラと DB 設計](../../02_基本設計書/03_インフラとDB設計.md)
