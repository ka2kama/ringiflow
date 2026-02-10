# 計画: #409 RLS 統合テスト

## Context

RLS（Row Level Security）ポリシーの動作を直接検証する統合テストが存在しない。既存テストはリポジトリの WHERE 句（アプリ層）を検証しているが、DB 層の RLS ポリシーが実際にクロステナントアクセスを防止することは未検証。#407（RLS スキーマ準備）と #408（TenantConnection 実装）が完了したため、DB 層の防御を統合テストで検証する。

## スコープ

**対象:**
- 新規ファイル `backend/crates/infra/tests/rls_test.rs` の作成
- 全 9 テナントスコープテーブルの RLS ポリシー動作検証
- `app.tenant_id` 未設定時の安全動作検証
- system roles の全テナント参照可能性検証
- TenantConnection と RLS の統合検証

**対象外:**
- 既存テストファイルの変更
- RLS ポリシー・マイグレーションの変更
- プロダクションコードの変更

## 設計判断

### テストファイル構成

`rls_test.rs` を新規作成。`db_test.rs`（接続管理）と `rls_test.rs`（データ分離）で関心事を分離する。

### テスト方式

`#[sqlx::test(migrations = "../../migrations")]` + `SET ROLE ringiflow_app` を採用。

- `#[sqlx::test]` がテスト用 DB を作成し、マイグレーションで `ringiflow_app` ロールとポリシーを適用
- superuser でテストデータを INSERT
- `SET ROLE ringiflow_app` で非 superuser に切り替えて RLS の動作を検証
- superuser は BYPASSRLS 権限を持つため、SET ROLE なしでは RLS がバイパスされる

### ヘルパー配置

`rls_test.rs` 内に定義。`common/mod.rs` はリポジトリテスト用のヘルパーであり、RLS テスト固有のセットアップ（2 テナント + 全テーブルデータ）を追加すると責務が混在する。

### TenantConnection テスト方式

`TenantConnection::acquire` は superuser プールでは RLS がバイパスされるため、SET ROLE と set_config を組み合わせた「本番相当操作のシミュレーション」として検証する。

## 実装計画

### 対象ファイル

| ファイル | 操作 |
|---------|------|
| `backend/crates/infra/tests/rls_test.rs` | 新規作成 |

### Phase 1: テストヘルパーと基本テーブル RLS テスト

#### 確認事項
- パターン: `#[sqlx::test]` 内での SET ROLE + set_config の動作 → 最初のテストで実証
- パターン: 既存テストの INSERT パターン → `backend/crates/infra/tests/user_repository_test.rs`
- 型: `Uuid::now_v7()` の使用パターン → 既存テストで確認済み

#### テストリスト

ヘルパー:
- [ ] `TwoTenantFixture` 構造体と `setup_two_tenants()` ヘルパー関数
- [ ] `set_tenant_context()` / `reset_role()` ヘルパー関数

tenants テーブル:
- [ ] テナント A のコンテキストで自テナントのみ取得できる
- [ ] `app.tenant_id` 未設定時にデータが返らない

users テーブル:
- [ ] テナント A のコンテキストで自テナントのユーザーのみ取得できる

roles テーブル:
- [ ] テナント A のコンテキストで自テナントロールと system roles が取得できる
- [ ] テナント固有ロールはクロステナントアクセスできない

### Phase 2: 残りのテーブルの RLS テスト

#### 確認事項
- 確認事項: なし（Phase 1 のパターン踏襲）

#### テストリスト

user_roles テーブル:
- [ ] テナント A のコンテキストで自テナントの user_roles のみ取得できる

workflow_definitions テーブル:
- [ ] テナント A のコンテキストで自テナントの定義のみ取得できる

workflow_instances テーブル:
- [ ] テナント A のコンテキストで自テナントのインスタンスのみ取得できる

workflow_steps テーブル:
- [ ] テナント A のコンテキストで自テナントのステップのみ取得できる

display_id_counters テーブル:
- [ ] テナント A のコンテキストで自テナントのカウンターのみ取得できる

auth.credentials テーブル:
- [ ] テナント A のコンテキストで自テナントの credentials のみ取得できる

### Phase 3: WITH CHECK テスト + TenantConnection 統合テスト

#### 確認事項
- パターン: RLS の WITH CHECK 違反時の PostgreSQL エラー → `new row violates row-level security policy`
- 型: `TenantConnection` の acquire/deref パターン → `backend/crates/infra/src/db.rs`

#### テストリスト

WITH CHECK（INSERT 制約）:
- [ ] テナント A のコンテキストでテナント B の tenant_id を持つ行を INSERT できない

TenantConnection 統合:
- [ ] SET ROLE + set_config でテナント分離が機能する（TenantConnection が本番で行う操作と同等）

## テストデータ構造

### TwoTenantFixture

```rust
struct TwoTenantFixture {
    tenant_a: Uuid,
    tenant_b: Uuid,
    user_a: Uuid,
    user_b: Uuid,
    definition_a: Uuid,
    definition_b: Uuid,
    instance_a: Uuid,
    instance_b: Uuid,
}
```

### INSERT 順序（FK 依存）

1. `tenants` (A, B)
2. `users` (A: user_a, B: user_b) — display_number 付き
3. `roles` (A: tenant_role_a, B: tenant_role_b) — system roles はシードで既存
4. `user_roles` (A: user_a + system 'user' role; B: user_b + system 'user' role) — tenant_id 付き
5. `workflow_definitions` (A: def_a, B: def_b)
6. `workflow_instances` (A: inst_a, B: inst_b) — display_number 付き
7. `workflow_steps` (A: step_a, B: step_b) — display_number + tenant_id 付き
8. `display_id_counters` (A, B) — (tenant_id, entity_type, last_number)
9. `auth.credentials` (A: cred_a, B: cred_b) — (user_id, tenant_id, credential_type, credential_data)

### テストの SQL パターン

```rust
#[sqlx::test(migrations = "../../migrations")]
async fn test_テーブル名_テナント分離(pool: PgPool) {
    // Arrange: superuser でデータ投入
    let fixture = setup_two_tenants(&pool).await;

    // Act: ringiflow_app ロールでテナント A としてクエリ
    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, &fixture.tenant_a).await;

    let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT id FROM テーブル名")
        .fetch_all(&mut *conn)
        .await
        .unwrap();

    // Assert: テナント A のデータのみ
    assert_eq!(rows.len(), 1);

    // Cleanup
    reset_role(&mut conn).await;
}
```

## テーブルスキーマ参照

| テーブル | PK | tenant 識別子 | 備考 |
|---------|-----|-------------|------|
| tenants | id (UUID) | `id` | ポリシーは `id = ...` |
| users | id (UUID) | tenant_id | display_number NOT NULL |
| roles | id (UUID) | tenant_id (nullable) | NULL = system role |
| user_roles | id (UUID) | tenant_id | user_id, role_id も必要 |
| workflow_definitions | id (UUID) | tenant_id | name, version, definition(JSON), status, created_by |
| workflow_instances | id (UUID) | tenant_id | definition_id, display_number, title, form_data(JSON), initiated_by |
| workflow_steps | id (UUID) | tenant_id | instance_id, display_number, step_id, step_name, step_type |
| display_id_counters | (tenant_id, entity_type) | tenant_id | last_number |
| auth.credentials | id (UUID) | tenant_id | user_id, credential_type, credential_data |

## 検証方法

```bash
# RLS テスト単体実行
cd backend && cargo test -p ringiflow-infra --test rls_test

# 全テスト
just check-all
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `#[sqlx::test]` 内で SET ROLE が動作するか未検証 | 技術的前提 | Phase 1 最初のテストで実証する方針。動作しない場合は `#[tokio::test]` + 手動 DB 管理に切り替え |
| 2回目 | TenantConnection::acquire は superuser プールでは RLS バイパス | アーキテクチャ不整合 | SET ROLE + set_config の組み合わせで本番相当操作をシミュレーションする方式に決定 |
| 3回目 | user_roles の tenant_id カラムが後追加（20260210000002） | 未定義 | マイグレーションを確認し、INSERT 時に tenant_id を含める |
| 4回目 | auth.credentials に user_id FK 制約なし | 既存手段の見落とし | FK 制約なし（サービス境界の独立性）を確認。テストでは任意の UUID を使用可能 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全 9 テーブルがテストに含まれている | OK | tenants, users, roles, user_roles, workflow_definitions, workflow_instances, workflow_steps, display_id_counters, auth.credentials — 全てテストリストに記載 |
| 2 | 曖昧さ排除 | テスト方式、データ構造が確定 | OK | SET ROLE パターン、TwoTenantFixture、INSERT 順序が明示。SET ROLE 動作未検証リスクは Phase 1 で対処 |
| 3 | 設計判断の完結性 | 全判断に理由が記載 | OK | ファイル構成、テスト方式、ヘルパー配置、TenantConnection テスト方式の 4 判断を記載 |
| 4 | スコープ境界 | 対象・対象外が明記 | OK | 冒頭のスコープセクションに記載 |
| 5 | 技術的前提 | SET ROLE + `#[sqlx::test]` の動作が考慮 | OK | Phase 1 で検証し、不可の場合のフォールバック（`#[tokio::test]`）を記載 |
| 6 | 既存ドキュメント整合 | マイグレーション・ナレッジベースと矛盾なし | OK | RLS ナレッジベースの SET ROLE パターン、マイグレーション 20260210000003 のポリシー定義と照合 |
