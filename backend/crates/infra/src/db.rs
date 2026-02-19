//! # PostgreSQL データベース接続管理
//!
//! データベース接続プールの作成と管理を行う。
//!
//! ## 設計方針
//!
//! - **接続プール**: 毎回接続を張り直すオーバーヘッドを避け、接続を再利用
//! - **sqlx 採用**: コンパイル時 SQL 検証、非同期サポート、型安全なクエリ
//! - **PostgreSQL 専用**: Aurora PostgreSQL との互換性を考慮
//!
//! ## 接続プールとは
//!
//! データベース接続は TCP 接続の確立、認証、SSL ハンドシェイクなど
//! コストの高い処理を伴う。接続プールは以下のように動作する:
//!
//! 1. 起動時に複数の接続を事前に確立
//! 2. クエリ実行時にプールから接続を借りる
//! 3. クエリ完了後、接続をプールに返却（切断しない）
//! 4. 次のクエリで同じ接続を再利用
//!
//! ## 本番環境での推奨設定
//!
//! ```rust,ignore
//! PgPoolOptions::new()
//!     .max_connections(20)           // 最大接続数（CPU コア数 × 2〜4）
//!     .min_connections(5)            // 最小接続数（起動時に確保）
//!     .acquire_timeout(Duration::from_secs(5))  // 接続取得タイムアウト
//!     .idle_timeout(Duration::from_secs(600))   // アイドル接続のタイムアウト
//!     .max_lifetime(Duration::from_secs(1800))  // 接続の最大寿命
//! ```
//!
//! ## 使用例
//!
//! ```rust,ignore
//! use ringiflow_infra::db;
//!
//! async fn example() -> Result<(), sqlx::Error> {
//!     let pool = db::create_pool("postgres://user:pass@localhost/ringiflow").await?;
//!
//!     // クエリ実行（接続はプールから自動取得・返却）
//!     let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
//!         .fetch_one(&pool)
//!         .await?;
//!
//!     Ok(())
//! }
//! ```

use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

use async_trait::async_trait;
use ringiflow_domain::tenant::TenantId;
use sqlx::{
    PgConnection,
    PgPool,
    Postgres,
    Transaction,
    pool::PoolConnection,
    postgres::PgPoolOptions,
};

use crate::error::InfraError;

/// RLS 用の `after_release` フックを含む `PgPoolOptions` を返す
///
/// コネクションがプールに返却される際、`app.tenant_id` セッション変数を
/// 空文字列にリセットする。これにより、別テナントのリクエストで
/// 前のテナントの ID が残留することを防ぐ。
///
/// テストでは `max_connections(1)` と組み合わせて使用する。
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

/// データベースマイグレーションを実行する
///
/// `sqlx::migrate!()` マクロで埋め込まれたマイグレーションファイルを
/// 順番に適用する。適用済みのマイグレーションはスキップされる。
///
/// sqlx が PostgreSQL の advisory lock を使用するため、
/// 複数プロセスから同時に呼び出しても安全。
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}

/// PostgreSQL 接続プールを作成する
///
/// アプリケーション起動時に一度だけ呼び出し、作成したプールを
/// アプリケーション全体で共有する。
///
/// # 引数
///
/// * `database_url` - PostgreSQL 接続 URL
///   - 形式: `postgres://user:password@host:port/database`
///   - SSL: `?sslmode=require` を付与して SSL を強制可能
///
/// # 戻り値
///
/// 成功時は `PgPool`（接続プール）を返す。
/// 失敗時は `sqlx::Error` を返す（接続失敗、認証エラーなど）。
///
/// # 設定値
///
/// - `max_connections(10)`: 最大接続数。本番環境では負荷に応じて調整
/// - `acquire_timeout(5秒)`: 接続取得のタイムアウト。超過時はエラー
///
/// # 例
///
/// ```rust,ignore
/// use ringiflow_infra::db;
///
/// let pool = db::create_pool("postgres://localhost/ringiflow").await?;
/// ```
///
/// # パニック
///
/// この関数はパニックしない。すべてのエラーは `Result` で返される。
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    pool_options()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

// =============================================================================
// TenantConnection
// =============================================================================

/// テナントスコープ付き DB コネクション
///
/// コネクション取得時に `app.tenant_id` PostgreSQL セッション変数を設定する。
/// RLS ポリシーがこの変数を参照してテナント分離を実現する。
///
/// ドロップ時（プールへの返却時）に [`pool_options`] の `after_release`
/// フックが `app.tenant_id` をリセットする。
pub struct TenantConnection {
    conn:      PoolConnection<Postgres>,
    tenant_id: TenantId,
}

impl TenantConnection {
    /// テナントスコープ付きコネクションを取得する
    ///
    /// プールからコネクションを取得し、`app.tenant_id` セッション変数に
    /// テナント ID を設定してから返す。
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

// Deref/DerefMut で PgConnection として使用可能にする。
// PoolConnection<Postgres> が Deref<Target = PgConnection> を実装しているため、
// TenantConnection も同じターゲットに deref する。
impl Deref for TenantConnection {
    type Target = PgConnection;

    fn deref(&self) -> &Self::Target {
        &self.conn
    }
}

impl DerefMut for TenantConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.conn
    }
}

// =============================================================================
// TxContext
// =============================================================================

/// トランザクションコンテキスト
///
/// 書き込みリポジトリメソッドの必須引数。
/// トランザクションなしの書き込みをコンパイルエラーにする（構造的強制）。
///
/// # 構造的強制とは
///
/// 従来は「書き込みにはトランザクションを使うべき」というルールだったが、
/// ルールの存在だけでは守られなかった。`TxContext` を必須引数にすることで、
/// トランザクションなしの書き込みはコンパイルエラーになる。
///
/// # ライフサイクル
///
/// 1. `TransactionManager::begin()` で作成
/// 2. 書き込みメソッドに `&mut TxContext` として渡す
/// 3. `commit()` でコミット、またはドロップでロールバック
pub struct TxContext(TxContextInner);

enum TxContextInner {
    Pg(Transaction<'static, Postgres>),
    #[cfg(any(test, feature = "test-utils"))]
    Mock,
}

impl TxContext {
    /// Postgres トランザクションを開始する
    ///
    /// `PgTransactionManager` のみが使用する。
    /// ユースケース層は `TransactionManager` trait 経由で TxContext を取得する。
    pub(crate) async fn begin_pg(pool: &PgPool) -> Result<Self, InfraError> {
        Ok(Self(TxContextInner::Pg(pool.begin().await?)))
    }

    /// テスト用のモック TxContext を作成する
    ///
    /// Mock リポジトリはインメモリ実装のため、実際のトランザクションは不要。
    /// `conn()` を呼ぶと panic するが、Mock リポジトリは `conn()` を使用しない。
    #[cfg(any(test, feature = "test-utils"))]
    pub fn mock() -> Self {
        Self(TxContextInner::Mock)
    }

    /// トランザクションをコミットする
    ///
    /// 呼ばずにドロップすると、sqlx が自動的にロールバックする。
    pub async fn commit(self) -> Result<(), InfraError> {
        match self.0 {
            TxContextInner::Pg(tx) => {
                tx.commit().await?;
                Ok(())
            }
            #[cfg(any(test, feature = "test-utils"))]
            TxContextInner::Mock => Ok(()),
        }
    }

    /// トランザクション内の DB コネクションを取得する
    ///
    /// Postgres リポジトリ実装が `sqlx::query!().execute(tx.conn())` として使用する。
    /// Phase 3（#687）で書き込みメソッドが TxContext を要求するようになった時に使用される。
    #[allow(dead_code)]
    pub(crate) fn conn(&mut self) -> &mut PgConnection {
        match &mut self.0 {
            TxContextInner::Pg(tx) => tx,
            #[cfg(any(test, feature = "test-utils"))]
            TxContextInner::Mock => {
                panic!("BUG: conn() called on Mock TxContext. Mock repos should not call conn().")
            }
        }
    }
}

// =============================================================================
// TransactionManager
// =============================================================================

/// トランザクション管理 trait
///
/// ユースケース層が TxContext を作成するための抽象化。
/// ユースケース層は PgPool に直接依存せず、この trait 経由で
/// トランザクションを開始する。
#[async_trait]
pub trait TransactionManager: Send + Sync {
    /// トランザクションを開始し、TxContext を返す
    async fn begin(&self) -> Result<TxContext, InfraError>;
}

/// Postgres 用 TransactionManager 実装
pub struct PgTransactionManager {
    pool: PgPool,
}

impl PgTransactionManager {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TransactionManager for PgTransactionManager {
    async fn begin(&self) -> Result<TxContext, InfraError> {
        TxContext::begin_pg(&self.pool).await
    }
}

// Send + Sync 検証
#[cfg(test)]
mod tx_context_tests {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn test_tx_contextはsendを実装している() {
        assert_send::<TxContext>();
    }

    #[test]
    fn test_pg_transaction_managerはsendとsyncを実装している() {
        assert_send_sync::<PgTransactionManager>();
    }

    #[test]
    fn test_transaction_manager_traitはsendとsyncを実装している() {
        assert_send_sync::<Box<dyn TransactionManager>>();
    }
}
