//! # フォルダハンドラ
//!
//! Core API のフォルダ管理内部 API を提供する。
//!
//! ## エンドポイント
//!
//! - `GET /internal/folders` - テナントのフォルダ一覧
//! - `POST /internal/folders` - フォルダ作成
//! - `PUT /internal/folders/{folder_id}` - フォルダ更新（名前変更・移動）
//! - `DELETE /internal/folders/{folder_id}` - フォルダ削除

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use ringiflow_domain::{folder::FolderId, tenant::TenantId};
use ringiflow_shared::ApiResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::CoreError,
    usecase::folder::{CreateFolderInput, FolderUseCaseImpl, UpdateFolderInput},
};

/// フォルダ API の共有状態
pub struct FolderState {
    pub usecase: FolderUseCaseImpl,
}

// --- リクエスト/レスポンス型 ---

/// テナント ID クエリパラメータ
#[derive(Debug, Deserialize)]
pub struct FolderTenantQuery {
    pub tenant_id: Uuid,
}

/// フォルダ作成リクエスト
#[derive(Debug, Deserialize)]
pub struct CreateFolderRequest {
    pub tenant_id:  Uuid,
    pub name:       String,
    pub parent_id:  Option<Uuid>,
    pub created_by: Uuid,
}

/// フォルダ更新リクエスト
#[derive(Debug, Deserialize)]
pub struct UpdateFolderRequest {
    pub tenant_id: Uuid,
    pub name:      Option<String>,
    pub parent_id: Option<Option<Uuid>>,
}

/// フォルダ DTO
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FolderDto {
    pub id:         Uuid,
    pub name:       String,
    pub parent_id:  Option<Uuid>,
    pub path:       String,
    pub depth:      i32,
    pub created_at: String,
    pub updated_at: String,
}

// --- ハンドラ ---

/// GET /internal/folders
///
/// テナントのフォルダ一覧を path 順で取得する。
#[tracing::instrument(skip_all)]
pub async fn list_folders(
    State(state): State<Arc<FolderState>>,
    Query(query): Query<FolderTenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    let folders = state.usecase.list_folders(&tenant_id).await?;

    let items: Vec<FolderDto> = folders
        .iter()
        .map(|f| FolderDto {
            id:         *f.id().as_uuid(),
            name:       f.name().as_str().to_string(),
            parent_id:  f.parent_id().map(|p| *p.as_uuid()),
            path:       f.path().to_string(),
            depth:      f.depth(),
            created_at: f.created_at().to_rfc3339(),
            updated_at: f.updated_at().to_rfc3339(),
        })
        .collect();

    let response = ApiResponse::new(items);
    Ok((StatusCode::OK, Json(response)))
}

/// POST /internal/folders
///
/// フォルダを作成する。
///
/// ## レスポンス
///
/// - `201 Created`: 作成されたフォルダ
/// - `400 Bad Request`: バリデーションエラー、階層上限超過
/// - `404 Not Found`: 親フォルダが存在しない
/// - `409 Conflict`: 同名フォルダ重複
#[tracing::instrument(skip_all)]
pub async fn create_folder(
    State(state): State<Arc<FolderState>>,
    Json(req): Json<CreateFolderRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let input = CreateFolderInput {
        tenant_id:  TenantId::from_uuid(req.tenant_id),
        name:       req.name,
        parent_id:  req.parent_id,
        created_by: req.created_by,
    };

    let folder = state.usecase.create_folder(input).await?;

    let dto = FolderDto {
        id:         *folder.id().as_uuid(),
        name:       folder.name().as_str().to_string(),
        parent_id:  folder.parent_id().map(|p| *p.as_uuid()),
        path:       folder.path().to_string(),
        depth:      folder.depth(),
        created_at: folder.created_at().to_rfc3339(),
        updated_at: folder.updated_at().to_rfc3339(),
    };

    let response = ApiResponse::new(dto);
    Ok((StatusCode::CREATED, Json(response)))
}

/// PUT /internal/folders/{folder_id}
///
/// フォルダを更新する（名前変更・移動）。
///
/// ## レスポンス
///
/// - `200 OK`: 更新後のフォルダ
/// - `400 Bad Request`: バリデーションエラー、循環移動
/// - `404 Not Found`: フォルダが見つからない
/// - `409 Conflict`: 同名フォルダ重複
#[tracing::instrument(skip_all, fields(%folder_id))]
pub async fn update_folder(
    State(state): State<Arc<FolderState>>,
    Path(folder_id): Path<Uuid>,
    Json(req): Json<UpdateFolderRequest>,
) -> Result<impl IntoResponse, CoreError> {
    let input = UpdateFolderInput {
        folder_id: FolderId::from_uuid(folder_id),
        tenant_id: TenantId::from_uuid(req.tenant_id),
        name:      req.name,
        parent_id: req.parent_id,
    };

    let folder = state.usecase.update_folder(input).await?;

    let dto = FolderDto {
        id:         *folder.id().as_uuid(),
        name:       folder.name().as_str().to_string(),
        parent_id:  folder.parent_id().map(|p| *p.as_uuid()),
        path:       folder.path().to_string(),
        depth:      folder.depth(),
        created_at: folder.created_at().to_rfc3339(),
        updated_at: folder.updated_at().to_rfc3339(),
    };

    let response = ApiResponse::new(dto);
    Ok((StatusCode::OK, Json(response)))
}

/// DELETE /internal/folders/{folder_id}
///
/// フォルダを削除する。
///
/// ## レスポンス
///
/// - `204 No Content`: 削除成功
/// - `400 Bad Request`: 子フォルダが存在する
/// - `404 Not Found`: フォルダが見つからない
#[tracing::instrument(skip_all, fields(%folder_id))]
pub async fn delete_folder(
    State(state): State<Arc<FolderState>>,
    Path(folder_id): Path<Uuid>,
    Query(query): Query<FolderTenantQuery>,
) -> Result<impl IntoResponse, CoreError> {
    let folder_id = FolderId::from_uuid(folder_id);
    let tenant_id = TenantId::from_uuid(query.tenant_id);

    state.usecase.delete_folder(&folder_id, &tenant_id).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use axum::{Router, body::Body, http::Request, routing::get};
    use chrono::{DateTime, Utc};
    use ringiflow_domain::{
        clock::Clock,
        folder::{Folder, FolderId, FolderName},
        tenant::TenantId,
        user::UserId,
    };
    use ringiflow_infra::{InfraError, repository::FolderRepository};
    use ringiflow_shared::ApiResponse;
    use tower::ServiceExt;

    use super::*;

    // --- スタブ ---

    struct StubFolderRepository {
        folders: Vec<Folder>,
    }

    impl StubFolderRepository {
        fn empty() -> Self {
            Self {
                folders: Vec::new(),
            }
        }

        fn with_folders(folders: Vec<Folder>) -> Self {
            Self { folders }
        }
    }

    #[async_trait]
    impl FolderRepository for StubFolderRepository {
        async fn find_all_by_tenant(
            &self,
            _tenant_id: &TenantId,
        ) -> Result<Vec<Folder>, InfraError> {
            Ok(self.folders.clone())
        }

        async fn find_by_id(
            &self,
            id: &FolderId,
            _tenant_id: &TenantId,
        ) -> Result<Option<Folder>, InfraError> {
            Ok(self.folders.iter().find(|f| f.id() == id).cloned())
        }

        async fn insert(&self, _folder: &Folder) -> Result<(), InfraError> {
            Ok(())
        }

        async fn update(
            &self,
            _tx: &mut ringiflow_infra::TxContext,
            _folder: &Folder,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn update_subtree_paths(
            &self,
            _tx: &mut ringiflow_infra::TxContext,
            _old_path: &str,
            _new_path: &str,
            _depth_delta: i32,
            _tenant_id: &TenantId,
        ) -> Result<(), InfraError> {
            Ok(())
        }

        async fn delete(&self, _id: &FolderId, _tenant_id: &TenantId) -> Result<(), InfraError> {
            Ok(())
        }

        async fn count_children(
            &self,
            parent_id: &FolderId,
            _tenant_id: &TenantId,
        ) -> Result<i64, InfraError> {
            let count = self
                .folders
                .iter()
                .filter(|f| f.parent_id() == Some(parent_id))
                .count() as i64;
            Ok(count)
        }
    }

    struct StubClock;

    impl Clock for StubClock {
        fn now(&self) -> DateTime<Utc> {
            DateTime::from_timestamp(1_700_000_000, 0).unwrap()
        }
    }

    // --- ヘルパー ---

    fn create_test_app(repo: StubFolderRepository) -> Router {
        let repo_arc = Arc::new(repo) as Arc<dyn FolderRepository>;
        let usecase = FolderUseCaseImpl::new(
            repo_arc,
            Arc::new(StubClock) as Arc<dyn Clock>,
            Arc::new(ringiflow_infra::mock::MockTransactionManager),
        );
        let state = Arc::new(FolderState { usecase });

        Router::new()
            .route("/internal/folders", get(list_folders).post(create_folder))
            .route(
                "/internal/folders/{folder_id}",
                axum::routing::put(update_folder).delete(delete_folder),
            )
            .with_state(state)
    }

    fn fixed_now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn create_root_folder(tenant_id: &TenantId, name: &str) -> Folder {
        Folder::new(
            FolderId::new(),
            tenant_id.clone(),
            FolderName::new(name).unwrap(),
            None,
            None,
            None,
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap()
    }

    fn create_child_folder(tenant_id: &TenantId, name: &str, parent: &Folder) -> Folder {
        Folder::new(
            FolderId::new(),
            tenant_id.clone(),
            FolderName::new(name).unwrap(),
            Some(parent.id().clone()),
            Some(parent.path()),
            Some(parent.depth()),
            Some(UserId::new()),
            fixed_now(),
        )
        .unwrap()
    }

    async fn response_body<T: serde::de::DeserializeOwned>(
        response: axum::http::Response<Body>,
    ) -> T {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // --- テストケース ---

    #[tokio::test]
    async fn test_post_ルート直下にフォルダを作成すると201が返る() {
        // Given
        let sut = create_test_app(StubFolderRepository::empty());
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/folders")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "name": "2026年度",
                    "parent_id": null,
                    "created_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: ApiResponse<FolderDto> = response_body(response).await;
        assert_eq!(body.data.name, "2026年度");
        assert_eq!(body.data.path, "/2026年度/");
        assert_eq!(body.data.depth, 1);
        assert!(body.data.parent_id.is_none());
    }

    #[tokio::test]
    async fn test_post_親フォルダの下にサブフォルダを作成すると201が返る() {
        // Given
        let tenant_id = TenantId::new();
        let parent = create_root_folder(&tenant_id, "2026年度");
        let parent_id = *parent.id().as_uuid();
        let sut = create_test_app(StubFolderRepository::with_folders(vec![parent]));

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/folders")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "name": "経費精算",
                    "parent_id": parent_id,
                    "created_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::CREATED);
        let body: ApiResponse<FolderDto> = response_body(response).await;
        assert_eq!(body.data.name, "経費精算");
        assert_eq!(body.data.path, "/2026年度/経費精算/");
        assert_eq!(body.data.depth, 2);
        assert_eq!(body.data.parent_id, Some(parent_id));
    }

    #[tokio::test]
    async fn test_post_フォルダ名が空のとき400が返る() {
        // Given
        let sut = create_test_app(StubFolderRepository::empty());
        let tenant_id = TenantId::new();

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/folders")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "name": "",
                    "parent_id": null,
                    "created_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_5階層を超えると400が返る() {
        // Given: depth = 5 のフォルダを親として指定
        let tenant_id = TenantId::new();
        let l1 = create_root_folder(&tenant_id, "l1");
        let l2 = create_child_folder(&tenant_id, "l2", &l1);
        let l3 = create_child_folder(&tenant_id, "l3", &l2);
        let l4 = create_child_folder(&tenant_id, "l4", &l3);
        let l5 = create_child_folder(&tenant_id, "l5", &l4);
        let l5_id = *l5.id().as_uuid();

        let sut = create_test_app(StubFolderRepository::with_folders(vec![l1, l2, l3, l4, l5]));

        let request = Request::builder()
            .method(axum::http::Method::POST)
            .uri("/internal/folders")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "name": "too-deep",
                    "parent_id": l5_id,
                    "created_by": Uuid::new_v4()
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_put_フォルダ名を変更すると200が返る() {
        // Given
        let tenant_id = TenantId::new();
        let folder = create_root_folder(&tenant_id, "old-name");
        let folder_id = *folder.id().as_uuid();
        let sut = create_test_app(StubFolderRepository::with_folders(vec![folder]));

        let request = Request::builder()
            .method(axum::http::Method::PUT)
            .uri(format!("/internal/folders/{}", folder_id))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "tenant_id": tenant_id.as_uuid(),
                    "name": "new-name"
                }))
                .unwrap(),
            ))
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<FolderDto> = response_body(response).await;
        assert_eq!(body.data.name, "new-name");
        assert_eq!(body.data.path, "/new-name/");
    }

    #[tokio::test]
    async fn test_delete_空フォルダを削除すると204が返る() {
        // Given
        let tenant_id = TenantId::new();
        let folder = create_root_folder(&tenant_id, "empty-folder");
        let folder_id = *folder.id().as_uuid();
        let sut = create_test_app(StubFolderRepository::with_folders(vec![folder]));

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/folders/{}?tenant_id={}",
                folder_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    #[tokio::test]
    async fn test_delete_子フォルダがあると400が返る() {
        // Given
        let tenant_id = TenantId::new();
        let parent = create_root_folder(&tenant_id, "parent");
        let child = create_child_folder(&tenant_id, "child", &parent);
        let parent_id = *parent.id().as_uuid();
        let sut = create_test_app(StubFolderRepository::with_folders(vec![parent, child]));

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/folders/{}?tenant_id={}",
                parent_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_フォルダ一覧がpath順で返る() {
        // Given
        let tenant_id = TenantId::new();
        let a = create_root_folder(&tenant_id, "a");
        let b = create_root_folder(&tenant_id, "b");
        let ab = create_child_folder(&tenant_id, "ab", &a);
        // path 順: /a/, /a/ab/, /b/
        let sut = create_test_app(StubFolderRepository::with_folders(vec![
            a.clone(),
            ab.clone(),
            b.clone(),
        ]));

        let request = Request::builder()
            .method(axum::http::Method::GET)
            .uri(format!(
                "/internal/folders?tenant_id={}",
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::OK);
        let body: ApiResponse<Vec<FolderDto>> = response_body(response).await;
        assert_eq!(body.data.len(), 3);
        assert_eq!(body.data[0].name, "a");
        assert_eq!(body.data[1].name, "ab");
        assert_eq!(body.data[2].name, "b");
    }

    #[tokio::test]
    async fn test_存在しないフォルダ_idで404が返る() {
        // Given
        let sut = create_test_app(StubFolderRepository::empty());
        let tenant_id = TenantId::new();
        let nonexistent_id = Uuid::new_v4();

        let request = Request::builder()
            .method(axum::http::Method::DELETE)
            .uri(format!(
                "/internal/folders/{}?tenant_id={}",
                nonexistent_id,
                tenant_id.as_uuid()
            ))
            .body(Body::empty())
            .unwrap();

        // When
        let response = sut.oneshot(request).await.unwrap();

        // Then
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
