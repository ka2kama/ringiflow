# Phase A-1: DB スキーマ変更

## 概要

表示用 ID の基盤となる DB スキーマ変更を実装した。
採番カウンターテーブルの作成、`workflow_instances` への `display_number` カラム追加、`DisplayNumber` 値オブジェクトの追加、既存コードの対応を含む。

### 対応 Issue

[#205 表示用 ID: DB スキーマ変更（Phase A-1）](https://github.com/ka2kama/ringiflow/issues/205)

---

## 設計書との対応

| 設計書セクション | 対応内容 |
|----------------|---------|
| [表示用 ID 設計 > DB スキーマ設計](../../03_詳細設計書/12_表示用ID設計.md#db-スキーマ設計) | テーブル定義、インデックス |
| [テナント退会時データ削除設計](../../03_詳細設計書/06_テナント退会時データ削除設計.md) | `display_id_counters` の削除方針（CASCADE） |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/migrations/20260202000001_create_display_id_counters.sql`](../../../backend/migrations/20260202000001_create_display_id_counters.sql) | 採番カウンターテーブル作成 |
| [`backend/migrations/20260202000002_add_display_number_to_workflow_instances.sql`](../../../backend/migrations/20260202000002_add_display_number_to_workflow_instances.sql) | カラム追加 + データ移行 + カウンター初期化 |
| [`backend/migrations/20260202000003_set_display_number_not_null.sql`](../../../backend/migrations/20260202000003_set_display_number_not_null.sql) | NOT NULL 制約適用 |
| [`backend/crates/domain/src/value_objects.rs`](../../../backend/crates/domain/src/value_objects.rs) | `DisplayNumber` 値オブジェクト |
| [`backend/crates/domain/src/workflow.rs`](../../../backend/crates/domain/src/workflow.rs) | `WorkflowInstance` に `display_number` フィールド追加 |
| [`backend/crates/infra/src/repository/workflow_instance_repository.rs`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs) | INSERT/SELECT クエリに `display_number` 追加 |

---

## 実装内容

### マイグレーション

3段階に分割して実行する。

**Migration 1: カウンターテーブル作成**

```sql
CREATE TABLE display_id_counters (
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    entity_type VARCHAR(50) NOT NULL,
    last_number BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (tenant_id, entity_type),
    CONSTRAINT chk_last_number_non_negative CHECK (last_number >= 0)
);
```

- 複合主キー `(tenant_id, entity_type)` でテナント × エンティティ種別の組み合わせを一意に管理
- `ON DELETE CASCADE` によりテナント削除時に自動削除

**Migration 2: カラム追加 + データ移行**

```sql
-- 1. カラム追加（NULLABLE）
ALTER TABLE workflow_instances ADD COLUMN display_number BIGINT;

-- 2. 部分ユニークインデックス
CREATE UNIQUE INDEX idx_workflow_instances_display_number
    ON workflow_instances (tenant_id, display_number)
    WHERE display_number IS NOT NULL;

-- 3. 既存データに display_number を割り当て
UPDATE workflow_instances SET display_number = sub.rn
FROM (
    SELECT id, ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY created_at, id) AS rn
    FROM workflow_instances
) sub
WHERE workflow_instances.id = sub.id;

-- 4. カウンターテーブル初期化
INSERT INTO display_id_counters (tenant_id, entity_type, last_number)
SELECT tenant_id, 'workflow_instance', COUNT(*)
FROM workflow_instances
GROUP BY tenant_id
ON CONFLICT (tenant_id, entity_type) DO UPDATE SET last_number = EXCLUDED.last_number;
```

**Migration 3: NOT NULL 制約**

```sql
ALTER TABLE workflow_instances ALTER COLUMN display_number SET NOT NULL;
```

### DisplayNumber 値オブジェクト（[`value_objects.rs`](../../../backend/crates/domain/src/value_objects.rs)）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DisplayNumber(i64);

impl DisplayNumber {
    pub fn new(value: i64) -> Result<Self, DomainError> { ... }  // 1 以上
    pub fn as_i64(&self) -> i64 { ... }                          // DB 互換
}

impl TryFrom<i64> for DisplayNumber { ... }  // DB 復元用
impl Display for DisplayNumber { ... }       // 数値のみ（プレフィックスなし）
```

TDD で 7 テスト:
- `new(0)` → Err、`new(1)` → Ok、`new(-1)` → Err、`new(i64::MAX)` → Ok
- `TryFrom(0)` → Err、`TryFrom(42)` → Ok
- `Display` → `"42"`

### WorkflowInstance モデル修正（[`workflow.rs`](../../../backend/crates/domain/src/workflow.rs)）

`display_number: DisplayNumber` フィールドを追加。`new()` と `from_db()` の引数に追加。getter メソッド `display_number()` を追加。

### リポジトリ修正（[`workflow_instance_repository.rs`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs)）

- INSERT クエリ: `display_number` カラムを追加（1箇所）
- SELECT クエリ: `display_number` カラムを追加（4箇所: `find_by_id`, `find_by_tenant`, `find_by_initiated_by`, `find_by_ids`）
- `from_db()` 呼び出し: `DisplayNumber::try_from(row.display_number)` を追加（4箇所）
- UPDATE クエリ: **変更なし**（`display_number` は作成後に変更されないため）

---

## テスト

### DisplayNumber 単体テスト

```bash
cd backend && cargo test -p ringiflow-domain test_表示用連番
```

7 テスト: 正常系 2 + 異常系 3 + 変換 1 + 表示形式 1

### 既存テストの修正

`WorkflowInstance::new()` の全呼び出し箇所に `DisplayNumber` 引数を追加:

| テストファイル | 修正箇所数 |
|--------------|-----------|
| `workflow_instance_repository_test.rs` | 11 |
| `workflow_step_repository_test.rs` | 7 |
| `workflow.rs`（usecase テスト） | 6 |
| `dashboard.rs`（usecase テスト） | 6 |
| `task.rs`（usecase テスト） | 5 |

テスト用値: `DisplayNumber::new(100).unwrap()`（同一テスト内で複数の場合は 100, 101, ...）

---

## 関連ドキュメント

- [表示用 ID 設計](../../03_詳細設計書/12_表示用ID設計.md)
- [テナント退会時データ削除設計](../../03_詳細設計書/06_テナント退会時データ削除設計.md)
- [セッションログ](../../../prompts/runs/2026-02/2026-02-02_1636_表示用ID_PhaseA1_DBスキーマ変更.md)
- [改善記録: ローカルと CI のテストスコープ差異](../../../prompts/improvements/2026-02/2026-02-02_1636_ローカルとCIのテストスコープ差異による検証漏れ.md)

---

## 設計解説

### 1. 3段階マイグレーションパターン

**場所**: `backend/migrations/20260202000001_*.sql` 〜 `20260202000003_*.sql`

**なぜこの設計か**: PostgreSQL では NOT NULL カラムの追加には既存行がすべて非 NULL であることが必要。1 ファイルに DDL + データ移行 + DDL を詰め込むと、ステップ間の暗黙の依存関係が生じる。3 ファイルに分離することで各ステップの意図が明確になり、トラブルシュート時にどのステップで失敗したかが分かる。

**代替案**: 1 ファイルに `DO $$` ブロックでまとめる方法もあるが、sqlx のマイグレーションは 1 ファイル = 1 トランザクションであり、分割しても実行コストは変わらない。可読性と保守性を優先した。

### 2. 部分ユニークインデックス

**場所**: Migration 2 の `CREATE UNIQUE INDEX ... WHERE display_number IS NOT NULL`

```sql
CREATE UNIQUE INDEX idx_workflow_instances_display_number
    ON workflow_instances (tenant_id, display_number)
    WHERE display_number IS NOT NULL;
```

**なぜこの設計か**: Migration 2 でカラムを追加した直後は NULL 行が存在する（データ移行前）。`WHERE display_number IS NOT NULL` とすることで、NULL 行はインデックス対象外となり、データ移行前にインデックスを作成できる。Migration 3 で NOT NULL 化した後は事実上フルインデックスとして機能する。

**代替案**: Migration 3 で通常の UNIQUE インデックスに作り直す方法もあるが、部分インデックスのまま残しても NOT NULL 制約があれば全行がインデックス対象になるため機能的に同等。不要なインデックス再構築を避けた。

### 3. ROW_NUMBER + ON CONFLICT によるデータ移行

**場所**: Migration 2 のデータ移行部分

```sql
-- display_number 割り当て
UPDATE workflow_instances SET display_number = sub.rn
FROM (SELECT id, ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY created_at, id) AS rn
      FROM workflow_instances) sub
WHERE workflow_instances.id = sub.id;

-- カウンター初期化
INSERT INTO display_id_counters (tenant_id, entity_type, last_number)
SELECT tenant_id, 'workflow_instance', COUNT(*) FROM workflow_instances GROUP BY tenant_id
ON CONFLICT (tenant_id, entity_type) DO UPDATE SET last_number = EXCLUDED.last_number;
```

**なぜこの設計か**: `ROW_NUMBER() OVER (PARTITION BY tenant_id ORDER BY created_at, id)` でテナントごとに `created_at` 順の連番を割り当てる。`id` をセカンダリソートキーにすることで、同一時刻のインスタンスも決定的な順序になる。`ON CONFLICT DO UPDATE` でカウンターの冪等な初期化を実現し、マイグレーションの再実行にも対応する。

### 4. DisplayNumber に `initial()` を設けない判断

**場所**: `backend/crates/domain/src/value_objects.rs`

**なぜこの設計か**: `Version` には `initial()` メソッド（常に 1 を返す）があるが、`DisplayNumber` には設けなかった。Version は「エンティティは常にバージョン 1 から始まる」という普遍的な意味があるが、DisplayNumber は「カウンターから取得した値」であり「初期値」の概念が異なる。`DisplayNumber::initial()` を作ると、採番サービスを経由せずに値を生成できてしまい、設計意図に反する。

**代替案**: テスト用に `DisplayNumber::for_test(value: i64)` のようなメソッドを追加する案もあるが、`#[cfg(test)]` の有無にかかわらず `new()` で十分。テスト用の特別メソッドは不要な複雑さを追加する。
