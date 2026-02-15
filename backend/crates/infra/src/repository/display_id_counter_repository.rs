//! # DisplayIdCounterRepository
//!
//! 表示用 ID の採番カウンターを管理するリポジトリ。
//!
//! ## 設計方針
//!
//! - **悲観的ロック**: `SELECT FOR UPDATE`
//!   でカウンター行を排他ロックし、並行採番の整合性を保証
//! - **トランザクション自己完結**: 採番操作は内部でトランザクションを管理し、
//!   呼び出し側にトランザクション管理を要求しない
//! - **テナント分離**: テナント × エンティティ種別ごとに独立した連番を管理
//!
//! 詳細: [表示用 ID 設計](../../../../docs/03_詳細設計書/12_表示用ID設計.md)

use async_trait::async_trait;
use ringiflow_domain::{
    tenant::TenantId,
    value_objects::{DisplayIdEntityType, DisplayNumber},
};
use sqlx::PgPool;

use crate::error::InfraError;

/// 表示用 ID カウンターリポジトリトレイト
///
/// テナント × エンティティ種別ごとの採番を管理する。
#[async_trait]
pub trait DisplayIdCounterRepository: Send + Sync {
    /// 次の表示用連番を取得する
    ///
    /// カウンターをインクリメントし、新しい番号を返す。
    /// `SELECT FOR UPDATE` による悲観的ロックで並行性を保証する。
    ///
    /// # 引数
    ///
    /// - `tenant_id`: テナント ID
    /// - `entity_type`: 対象エンティティ種別
    ///
    /// # 戻り値
    ///
    /// - `Ok(DisplayNumber)`: 採番された連番
    /// - `Err(InfraError)`: カウンター行が存在しない場合やデータベースエラー
    async fn next_display_number(
        &self,
        tenant_id: &TenantId,
        entity_type: DisplayIdEntityType,
    ) -> Result<DisplayNumber, InfraError>;
}

/// PostgreSQL 実装の表示用 ID カウンターリポジトリ
///
/// `display_id_counters` テーブルを使用して採番を行う。
/// 各呼び出しで内部的にトランザクションを開始・コミットする。
pub struct PostgresDisplayIdCounterRepository {
    pool: PgPool,
}

impl PostgresDisplayIdCounterRepository {
    /// 新しいリポジトリを作成する
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DisplayIdCounterRepository for PostgresDisplayIdCounterRepository {
    async fn next_display_number(
        &self,
        tenant_id: &TenantId,
        entity_type: DisplayIdEntityType,
    ) -> Result<DisplayNumber, InfraError> {
        let mut tx = self.pool.begin().await?;
        let entity_type_str: &str = entity_type.into();

        // 悲観的ロック付きでカウンターを取得
        let row = sqlx::query!(
            r#"
         SELECT last_number
         FROM display_id_counters
         WHERE tenant_id = $1 AND entity_type = $2
         FOR UPDATE
         "#,
            tenant_id.as_uuid(),
            entity_type_str,
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => InfraError::Unexpected(format!(
                "カウンター行が見つかりません: tenant_id={}, entity_type={}",
                tenant_id.as_uuid(),
                entity_type_str
            )),
            other => InfraError::Database(other),
        })?;

        let next = row.last_number + 1;

        // カウンターを更新
        sqlx::query!(
            r#"
         UPDATE display_id_counters
         SET last_number = $3
         WHERE tenant_id = $1 AND entity_type = $2
         "#,
            tenant_id.as_uuid(),
            entity_type_str,
            next,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        DisplayNumber::new(next).map_err(|e| InfraError::Unexpected(e.to_string()))
    }
}
