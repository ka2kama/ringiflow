# PostgreSQL Row Level Security (RLS)

## 概要

Row Level Security（RLS）は PostgreSQL の機能で、テーブル内の行単位でアクセス制御を実現する。マルチテナントアプリケーションでは、テナント間のデータ分離を DB レベルで強制するために使用する。

## 主な機能

### RLS の有効化

```sql
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
```

- テーブル所有者（通常は superuser）は RLS をバイパスする（`BYPASSRLS` 権限）
- アプリケーション用の非 superuser ロールを作成し、RLS を適用する

### ポリシーの作成

```sql
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = current_setting('app.tenant_id')::UUID);
```

- `USING` 句: SELECT、UPDATE、DELETE に適用される行フィルタ
- `WITH CHECK` 句: INSERT、UPDATE に適用される新しい行の検証（省略時は `USING` と同じ）
- ポリシーがない場合、RLS が有効なテーブルへのアクセスはすべて拒否される

### GUC（Grand Unified Configuration）によるテナントコンテキスト

PostgreSQL の GUC 変数を使ってコネクションにテナント ID を設定する:

```sql
-- テナントコンテキストの設定（パラメータ化クエリ対応）
SELECT set_config('app.tenant_id', $1, false);

-- テナントコンテキストの取得
SELECT current_setting('app.tenant_id', true);

-- リセット
SELECT set_config('app.tenant_id', '', false);
```

注意:
- `SET app.tenant_id = ...` はパラメータ化クエリ（`$1`）に対応していないため、`set_config()` を使用する
- `current_setting('app.tenant_id', true)` の第2引数 `true` は `missing_ok`。未設定時にエラーではなく空文字列を返す

## 安全なポリシーパターン: NULLIF

```sql
CREATE POLICY tenant_isolation ON users
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);
```

このパターンが必要な理由:

1. `current_setting('app.tenant_id', true)` は未設定時に空文字列 `''` を返す
2. `''::UUID` は PostgreSQL でキャストエラーになる
3. `NULLIF('', '')` は `NULL` を返す
4. `tenant_id = NULL` は常に `false`（SQL の三値論理）

結果: `app.tenant_id` が未設定の場合、どの行もマッチしない（安全側に倒れる）。

## テスト戦略

### superuser と RLS

`#[sqlx::test]` など、superuser で接続するテストフレームワークでは RLS は自動的にバイパスされる（`BYPASSRLS` 権限）。既存テストはそのまま動作する。

### RLS の動作検証

テスト内で `SET ROLE` を使って非 superuser に切り替え、RLS の動作を検証する:

```sql
-- テスト内で非 superuser に切り替え
SET ROLE ringiflow_app;

-- テナントコンテキスト設定
SELECT set_config('app.tenant_id', 'テナントA_UUID', false);

-- クエリ実行（RLS が適用される）
SELECT * FROM users;

-- superuser に戻す
RESET ROLE;
```

## プロジェクトでの使用箇所

- マイグレーション: `backend/migrations/20260210000003_enable_rls_policies.sql`
- 対象テーブル: 全 9 テナントスコープテーブル（tenants, users, roles, user_roles, workflow_definitions, workflow_instances, workflow_steps, display_id_counters, auth.credentials）
- アプリケーションロール: `ringiflow_app`
- 設計判断: [計画ファイル](../../prompts/plans/snoopy-prancing-abelson.md)（リネーム予定）

## 関連リソース

- [PostgreSQL 公式ドキュメント: Row Security Policies](https://www.postgresql.org/docs/current/ddl-rowsecurity.html)
- [PostgreSQL 公式ドキュメント: SET ROLE](https://www.postgresql.org/docs/current/sql-set-role.html)
- [PostgreSQL 公式ドキュメント: set_config](https://www.postgresql.org/docs/current/functions-admin.html#FUNCTIONS-ADMIN-SET)
