# #408 RLS コネクション管理: after_release フックと TenantConnection

## Context

Phase 2-1（マルチテナント RLS）の一環。DB コネクション返却時のテナントコンテキストリセット機構と、テナントスコープ付きコネクション型を導入する。

現状: `create_pool` は単純な接続プール作成のみ（`backend/crates/infra/src/db.rs`）。RLS 用のセッション変数管理は未実装。Phase 1 ではアプリケーション層（WHERE 句）でのみテナント分離を実現しており、基本設計書 7.1.3 節で定義された二重防御の DB 層が未実装。

## スコープ

**対象:**

- `create_pool` に `after_release` フックを追加（`set_config` でリセット）
- `pool_options()` ヘルパー関数を抽出（テストでの再利用のため）
- `TenantConnection` 型を新設（`set_config` でテナント ID を設定済みのコネクションを返す）
- 統合テスト

**対象外:**

- RLS ポリシーの作成（#407）
- 既存リポジトリの RLS 対応（#410）
- RLS 統合テスト — クロステナントアクセス防止検証（#409）

## 設計

### 設計判断 1: `pool_options()` の抽出

`after_release` フックを含む `PgPoolOptions` を返す関数を抽出する。

理由:

- `create_pool` は `pool_options()` を使って本番設定（max_connections, timeout）を付加する
- テストでは `pool_options()` を直接使い、`max_connections(1)` で同一物理接続の再取得を保証できる
- フック定義の重複を避ける

```rust
/// RLS 用の after_release フックを含む PgPoolOptions を返す
///
/// コネクションがプールに返却される際、`app.tenant_id` セッション変数を
/// 空文字列にリセットする。これにより、別テナントのリクエストで
/// 前のテナントの ID が残留することを防ぐ。
pub fn pool_options() -> PgPoolOptions {
    PgPoolOptions::new().after_release(|conn, _meta| {
        Box::pin(async move {
            sqlx::query("SELECT set_config('app.tenant_id', '', false)")
                .execute(&mut *conn)
                .await?;
            Ok(true)
        })
    })
}

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    pool_options()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}
```

`set_config` の第3引数 `false`（`is_local`）: セッション全体に適用。`true` にするとトランザクション内のみ有効になるが、RLS ポリシーはトランザクション外のクエリにも適用する必要があるため `false`。

### 設計判断 2: `TenantConnection` 型

テナントスコープ付きコネクション。`acquire` で `app.tenant_id` を設定し、ドロップ時に `after_release` フックでリセットされる。

```rust
/// テナントスコープ付き DB コネクション
///
/// コネクション取得時に `app.tenant_id` PostgreSQL セッション変数を設定する。
/// RLS ポリシーがこの変数を参照してテナント分離を実現する。
///
/// ドロップ時（プールへの返却時）に `after_release` フックが
/// `app.tenant_id` をリセットする。
pub struct TenantConnection {
    conn: PoolConnection<Postgres>,
    tenant_id: TenantId,
}

impl TenantConnection {
    /// テナントスコープ付きコネクションを取得する
    pub async fn acquire(pool: &PgPool, tenant_id: &TenantId) -> Result<Self, sqlx::Error> {
        let mut conn = pool.acquire().await?;
        sqlx::query("SELECT set_config('app.tenant_id', $1, false)")
            .bind(tenant_id.to_string())
            .execute(&mut *conn)
            .await?;
        Ok(Self {
            conn,
            tenant_id: tenant_id.clone(),
        })
    }

    /// 設定されているテナント ID を取得する
    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }
}

// Deref/DerefMut で PgConnection として使用可能にする
// PoolConnection<Postgres> が Deref<Target = PgConnection> を実装しているため、
// TenantConnection も同じターゲットに deref する
impl Deref for TenantConnection {
    type Target = PgConnection;
    fn deref(&self) -> &Self::Target {
        &self.conn // PoolConnection の Deref を経由して PgConnection への参照を返す
    }
}

impl DerefMut for TenantConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}
```

代替案:

| 案 | メリット | デメリット | 判断 |
|----|---------|-----------|------|
| Deref/DerefMut で PgConnection を公開 | sqlx のエグゼキュータとして直接使える | Deref 乱用の懸念 | **採用**: スマートポインタパターンに該当（PoolConnection と同等） |
| `as_conn()` メソッドで明示的に取得 | 明示的 | sqlx の使い勝手が悪化 | 見送り |
| Executor トレイト実装 | 最も型安全 | 実装が複雑、sqlx の内部トレイト | 見送り |

### 設計判断 3: テスト方針

`set_config` / `current_setting` は PostgreSQL のビルトイン関数でテーブル不要。`#[tokio::test]` + `DATABASE_URL` を使用する（`session_test.rs` と同じパターン）。

理由:

- `#[sqlx::test]` はテスト用 DB を作成しマイグレーション実行するが、本テストではテーブル不要
- `Pool::connect_options()` は `any` feature が必要で、現プロジェクトでは無効
- `session_test.rs` で `#[tokio::test]` + 環境変数パターンの前例あり

`max_connections(1)` を使用して、同一物理接続の再取得を保証する。

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: after_release フック + pool_options

#### 確認事項

- ライブラリ: `PgPoolOptions::after_release` のシグネチャ → docs.rs で確認済み。`Fn(&mut PgConnection, PoolConnectionMetadata) -> Pin<Box<dyn Future<Output = Result<bool, Error>> + Send>>`
- ライブラリ: `PoolConnectionMetadata` のインポートパス → Grep `PoolConnectionMetadata` in project
- パターン: 既存の `create_pool` → `backend/crates/infra/src/db.rs:85-91`
- パターン: session_test.rs のテスト構造 → `backend/crates/infra/tests/session_test.rs`

#### テストリスト

- [ ] after_release でコネクション返却時に tenant_id がリセットされる

#### 変更内容

1. `db.rs`: `pool_options()` 関数を抽出し、`after_release` フックを追加
2. `db.rs`: `create_pool()` を `pool_options()` を使うように変更
3. `db_test.rs`: テスト作成

### Phase 2: TenantConnection

#### 確認事項

- 型: `TenantId` の定義 → `backend/crates/domain/src/tenant.rs:73-75`（Newtype(Uuid), Display, Clone）
- 型: `PoolConnection<Postgres>` → `sqlx::pool::PoolConnection` + `sqlx::Postgres`
- パターン: `Deref`/`DerefMut` 実装 → `PoolConnection` のソースを参照

#### テストリスト

- [ ] acquire でテナント ID がセッション変数に設定される
- [ ] drop 後に接続がプールに返却され tenant_id がリセットされる（E2E）
- [ ] 異なるテナントで連続して acquire できる
- [ ] TenantConnection は Send を実装している（コンパイル時検証）

#### 変更内容

1. `db.rs`: `TenantConnection` 構造体、`acquire` メソッド、`Deref`/`DerefMut` 実装
2. `db_test.rs`: テスト追加

## 検証方法

1. `just check-all` がパスすること
2. `cd backend && cargo test -p ringiflow-infra --test db_test` で個別テスト実行
3. `just sqlx-prepare` でキャッシュ更新（`db.rs` 変更のため）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `Pool::connect_options()` が `any` feature 必須で使用不可。テストでプールの接続情報を再利用できない | 技術的前提 | `#[sqlx::test]` ではなく `#[tokio::test]` + `DATABASE_URL` に変更（session_test.rs と同パターン） |
| 2回目 | `TenantConnection` が `tenant_id` を保持しない設計だと、デバッグや後続 Issue（#410 リポジトリ統合）で不便 | 品質の向上: 型の活用 | `tenant_id` フィールドと `tenant_id()` アクセサを追加 |
| 3回目 | `set_config` の `is_local` パラメータの意味が曖昧 | 曖昧さ排除 | `false`（セッション全体に適用）の理由を設計判断に明記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue #408 の完了基準 6 項目すべてを Phase 1-2 のテストリストと検証方法でカバー |
| 2 | 曖昧さ排除 | OK | コードスニペットで関数シグネチャ・型定義を一意に確定。`is_local` パラメータの意味も明記 |
| 3 | 設計判断の完結性 | OK | pool_options 抽出、TenantConnection の Deref 設計、テスト方針の 3 判断に代替案と理由を記載 |
| 4 | スコープ境界 | OK | 対象（フック + TenantConnection + テスト）と対象外（#407, #409, #410）を明記 |
| 5 | 技術的前提 | OK | sqlx 0.8 `after_release` API を docs.rs で確認。`connect_options()` の `any` feature 制約も確認済み |
| 6 | 既存ドキュメント整合 | OK | 基本設計書 7.1.3 の二重防御設計（SET app.tenant_id + RLS policy）に準拠 |
