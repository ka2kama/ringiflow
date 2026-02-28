# PostgreSQL マイグレーション構文

## 概要

このプロジェクトのマイグレーションファイルで使用している PostgreSQL 特有の構文を解説する。
標準 SQL にはない PostgreSQL 独自の機能が多く含まれている。

## 構文一覧

### 1. ドルクォートと PL/pgSQL 関数

```sql
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

| 要素 | 説明 |
|------|------|
| `CREATE OR REPLACE FUNCTION` | 関数を作成。既存なら置き換え |
| `$$` ... `$$` | ドルクォート。関数本体を囲む区切り文字 |
| `LANGUAGE plpgsql` | 関数の言語。PL/pgSQL は PostgreSQL の手続き型言語 |

**ドルクォートの利点:**
- シングルクォート `'...'` の代わりに使える
- 内部でシングルクォートをエスケープする必要がない
- `$tag$...$tag$` のようにタグ付きも可能（ネスト時に便利）

**PL/pgSQL（ピーエル・ピージーエスキューエル）とは:**

PostgreSQL の手続き型言語（Procedural Language）。純粋な SQL では書けない処理を可能にする。

| 処理 | SQL | PL/pgSQL |
|------|-----|----------|
| 変数 | ❌ | ✅ `DECLARE x INTEGER;` |
| 条件分岐 | △ CASE式のみ | ✅ `IF/ELSIF/ELSE` |
| ループ | ❌ | ✅ `FOR/WHILE/LOOP` |
| 例外処理 | ❌ | ✅ `EXCEPTION WHEN` |
| トリガー関数 | ❌ | ✅ `RETURNS TRIGGER` |

このプロジェクトでは `updated_at` 自動更新のトリガー関数で使用。純粋な SQL では `NEW` への代入ができないため PL/pgSQL が必要。

### 2. RETURNS TRIGGER

```sql
RETURNS TRIGGER AS $$
```

トリガーから呼ばれる関数専用の戻り値型。

- 通常の関数: `RETURNS INTEGER`, `RETURNS TEXT` など
- トリガー関数: `RETURNS TRIGGER` を指定
- 実際に返すのは `NEW`、`OLD`、または `NULL`

### 3. NEW / OLD（トリガー特殊変数）

```sql
NEW.updated_at = NOW();
RETURN NEW;
```

トリガー関数内で使える特殊変数。

| 変数 | 説明 | 使用可能な操作 |
|------|------|---------------|
| `NEW` | INSERT/UPDATE 後の行データ | INSERT, UPDATE |
| `OLD` | UPDATE/DELETE 前の行データ | UPDATE, DELETE |

```
UPDATE users SET name = 'Bob' WHERE id = 1;
         ↓
トリガー発火
         ↓
OLD.name = 'Alice'（変更前）
NEW.name = 'Bob'  （変更後）
```

`RETURN NEW` で変更後の行を返すと、その値が実際にテーブルに書き込まれる。
`RETURN NULL` を返すと、その行の操作がキャンセルされる（BEFORE トリガーの場合）。

### 4. gen_random_uuid()

```sql
id UUID PRIMARY KEY DEFAULT gen_random_uuid()
```

UUID v4（ランダム）を生成する PostgreSQL 組み込み関数。

- PostgreSQL 13 以降で標準搭載（拡張不要）
- PostgreSQL 12 以前は `uuid-ossp` 拡張が必要だった
- 生成例: `550e8400-e29b-41d4-a716-446655440000`

**UUID v4 の特徴:**
- 122 ビットのランダム値
- 衝突確率は極めて低い（実質ゼロ）
- 順序性がないため、B-tree インデックスの効率は低め

### 5. TIMESTAMPTZ

```sql
created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
```

タイムゾーン付きタイムスタンプ型。`TIMESTAMP WITH TIME ZONE` の略。

| 型 | 内部保存 | 入出力時の動作 |
|---|---------|---------------|
| `TIMESTAMP` | そのまま | タイムゾーン情報なし |
| `TIMESTAMPTZ` | UTC に変換 | セッションの TZ で表示 |

```sql
-- セッションが Asia/Tokyo の場合
SET timezone = 'Asia/Tokyo';
INSERT INTO t (ts) VALUES ('2026-01-17 10:00:00');
SELECT ts FROM t;  -- 2026-01-17 10:00:00+09

-- セッションを変更
SET timezone = 'UTC';
SELECT ts FROM t;  -- 2026-01-17 01:00:00+00
```

**ベストプラクティス:** 日時は常に `TIMESTAMPTZ` を使う。`TIMESTAMP` はタイムゾーンの曖昧さを生む原因になる。

### 6. JSONB

```sql
settings JSONB NOT NULL DEFAULT '{}'
```

バイナリ形式で保存される JSON 型。

| 型 | 保存形式 | 特徴 |
|---|---------|------|
| `JSON` | テキスト | 挿入が速い、キーの順序保持 |
| `JSONB` | バイナリ | 検索・更新が速い、インデックス対応 |

**JSONB の演算子:**

| 演算子 | 説明 | 例 |
|--------|------|-----|
| `->` | JSON 値を取得 | `settings->'theme'` → `"dark"` |
| `->>` | テキストとして取得 | `settings->>'theme'` → `dark` |
| `@>` | 包含検索 | `settings @> '{"a": 1}'` |
| `?` | キー存在確認 | `settings ? 'theme'` |

```sql
-- 検索例
SELECT * FROM tenants WHERE settings->>'theme' = 'dark';
SELECT * FROM tenants WHERE settings @> '{"notifications": true}';

-- 更新例
UPDATE tenants SET settings = settings || '{"new_key": "value"}';
```

**推奨:** ほぼ常に `JSONB` を使う。`JSON` を選ぶ理由はほとんどない。

### 7. UNIQUE 制約とインデックスの関係

```sql
ALTER TABLE users ADD CONSTRAINT users_tenant_email_key UNIQUE (tenant_id, email);
```

**UNIQUE 制約を定義すると、PostgreSQL は自動的にユニークインデックスを作成する。**

これは UNIQUE 制約を強制するための内部実装であり、明示的に `CREATE INDEX` する必要はない。

```sql
-- この UNIQUE 制約を定義すると...
UNIQUE (tenant_id, email)

-- PostgreSQL が自動的にこれと同等のインデックスを作成する
CREATE UNIQUE INDEX users_tenant_email_key ON users (tenant_id, email);
```

**PRIMARY KEY も同様:**

```sql
PRIMARY KEY (id)  -- 自動的に id にユニークインデックスが作成される
```

**確認方法:**

```sql
-- psql でテーブルのインデックス一覧を表示
\d users

-- SQL で確認
SELECT indexname, indexdef FROM pg_indexes WHERE tablename = 'users';
```

**注意点:**

| 観点 | 説明 |
|------|------|
| 制約削除時 | 制約を削除するとインデックスも一緒に削除される |
| 命名 | 制約名がそのままインデックス名になる |
| 機能 | 明示的に作成したインデックスと機能的には同等 |

**設計書との関係:**

設計書で「UNIQUE 制約」と記載されている箇所は、自動的にインデックスも存在することを意味する。
そのため、同じカラムに対して別途インデックスを作成する必要はない。

### 8. CONSTRAINT ... CHECK

```sql
CONSTRAINT tenants_plan_check CHECK (plan IN ('free', 'standard', 'professional', 'enterprise'))
```

値の制約を定義する。違反するとエラー。

```sql
-- 成功
INSERT INTO tenants (name, subdomain, plan) VALUES ('A社', 'a-corp', 'free');

-- エラー
INSERT INTO tenants (name, subdomain, plan) VALUES ('B社', 'b-corp', 'invalid');
-- ERROR: new row violates check constraint "tenants_plan_check"
```

**CHECK 制約の用途:**
- 許可値のリスト: `CHECK (status IN ('active', 'inactive'))`
- 範囲制限: `CHECK (age >= 0 AND age <= 150)`
- 正規表現: `CHECK (email ~ '^[^@]+@[^@]+$')`
- 複数カラムの関係: `CHECK (start_date < end_date)`

**利点:** アプリではなく DB レベルで不正データを防ぐ。データ整合性の最後の砦。

### 9. CREATE TRIGGER

**トリガーとは:** テーブルへの操作（INSERT/UPDATE/DELETE）をきっかけに自動実行される処理。
アプリ側で毎回書く必要がなくなり、漏れを防げる。

```sql
CREATE TRIGGER tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at();
```

テーブルへの操作時に自動で関数を実行する。

| 要素 | 説明 |
|------|------|
| `BEFORE` / `AFTER` | 操作の前か後か |
| `INSERT` / `UPDATE` / `DELETE` | 対象の操作 |
| `ON tenants` | 対象テーブル |
| `FOR EACH ROW` | 行ごとに実行 |
| `FOR EACH STATEMENT` | クエリ単位で1回実行 |
| `EXECUTE FUNCTION` | 呼び出す関数 |

```
UPDATE tenants SET name = 'New' WHERE id = 1;
         ↓
1. BEFORE UPDATE トリガー発火
2. update_updated_at() 実行 → NEW.updated_at = NOW()
3. 実際の UPDATE 実行（updated_at も更新される）
4. AFTER UPDATE トリガー発火（あれば）
```

**主な用途:**
- `updated_at` の自動更新
- 監査ログの記録
- データ検証
- 派生値の計算

**なぜトリガーを使うか:**

| アプローチ | 問題点 |
|-----------|--------|
| アプリで毎回 `updated_at = NOW()` を書く | 書き忘れる可能性がある |
| **トリガーで自動化** | 漏れがない、DB レベルで保証される |

### 10. REFERENCES ... ON DELETE CASCADE

```sql
tenant_id UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE
```

外部キー制約と、親削除時の動作を指定。

| 句 | 説明 |
|---|------|
| `REFERENCES tenants(id)` | `tenants.id` への外部キー制約 |
| `ON DELETE CASCADE` | 親が削除されたら子も削除 |

```sql
-- tenants に id=1 のレコードがある
INSERT INTO users (tenant_id, ...) VALUES (1, ...);  -- OK
INSERT INTO users (tenant_id, ...) VALUES (999, ...);  -- ERROR（存在しない）

-- テナントを削除すると...
DELETE FROM tenants WHERE id = 1;
-- → そのテナントの users も自動削除される（CASCADE）
```

**ON DELETE オプション一覧:**

| オプション | 動作 |
|-----------|------|
| `CASCADE` | 親削除 → 子も削除 |
| `SET NULL` | 親削除 → 子の FK を NULL に |
| `SET DEFAULT` | 親削除 → 子の FK をデフォルト値に |
| `RESTRICT` | 子がいる限り親を削除不可（デフォルト） |
| `NO ACTION` | RESTRICT と同じ（制約チェックのタイミングが違う） |

**ON UPDATE も同様に指定可能:**

```sql
REFERENCES tenants(id) ON DELETE CASCADE ON UPDATE CASCADE
```

## プロジェクトでの使用箇所

| ファイル | 使用している構文 |
|---------|-----------------|
| `20260115000001_create_tenants.sql` | トリガー関数、CHECK、JSONB、TIMESTAMPTZ |
| `20260115000002_create_users.sql` | ON DELETE CASCADE、複合ユニーク制約 |
| `20260115000003_create_roles.sql` | JSONB（permissions）、NULL 許容 FK |
| `20260115000004_create_user_roles.sql` | 複合ユニーク制約、CASCADE |

## 関連リソース

- [PostgreSQL 公式ドキュメント: CREATE FUNCTION](https://www.postgresql.org/docs/current/sql-createfunction.html)
- [PostgreSQL 公式ドキュメント: CREATE TRIGGER](https://www.postgresql.org/docs/current/sql-createtrigger.html)
- [PostgreSQL 公式ドキュメント: JSON Types](https://www.postgresql.org/docs/current/datatype-json.html)
- [PostgreSQL 公式ドキュメント: Constraints](https://www.postgresql.org/docs/current/ddl-constraints.html)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-17 | 初版作成 |
| 2026-01-17 | UNIQUE 制約とインデックスの関係を追加 |
