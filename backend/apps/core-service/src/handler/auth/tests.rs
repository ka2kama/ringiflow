use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Router,
    body::Body,
    http::{Method, Request},
    routing::get,
};
use ringiflow_domain::{
    clock::Clock,
    role::{Permission, Role, RoleId},
    tenant::{Tenant, TenantId, TenantName},
    user::{Email, User, UserId, UserStatus},
    value_objects::{DisplayIdEntityType, DisplayNumber, UserName},
};
use ringiflow_infra::{InfraError, repository::DisplayIdCounterRepository};
use tower::ServiceExt;

use super::*;

// テスト用のスタブ実装

struct StubUserRepository {
    user: Option<User>,
    user_with_roles: Option<(User, Vec<Role>)>,
}

impl StubUserRepository {
    fn with_user(user: User, roles: Vec<Role>) -> Self {
        Self {
            user: Some(user.clone()),
            user_with_roles: Some((user, roles)),
        }
    }

    fn empty() -> Self {
        Self {
            user: None,
            user_with_roles: None,
        }
    }
}

#[async_trait]
impl UserRepository for StubUserRepository {
    async fn find_by_email(
        &self,
        _tenant_id: &TenantId,
        _email: &Email,
    ) -> Result<Option<User>, InfraError> {
        Ok(self.user.clone())
    }

    async fn find_by_id(&self, _id: &UserId) -> Result<Option<User>, InfraError> {
        Ok(self.user.clone())
    }

    async fn find_with_roles(&self, _id: &UserId) -> Result<Option<(User, Vec<Role>)>, InfraError> {
        Ok(self.user_with_roles.clone())
    }

    async fn find_by_ids(&self, _ids: &[UserId]) -> Result<Vec<User>, InfraError> {
        Ok(self.user.clone().into_iter().collect())
    }

    async fn find_all_active_by_tenant(
        &self,
        _tenant_id: &TenantId,
    ) -> Result<Vec<User>, InfraError> {
        Ok(self.user.clone().into_iter().collect())
    }

    async fn update_last_login(&self, _id: &UserId) -> Result<(), InfraError> {
        Ok(())
    }

    async fn insert(&self, _user: &User) -> Result<(), InfraError> {
        todo!()
    }

    async fn update(&self, _user: &User) -> Result<(), InfraError> {
        todo!()
    }

    async fn update_status(&self, _user: &User) -> Result<(), InfraError> {
        todo!()
    }

    async fn find_by_display_number(
        &self,
        _tenant_id: &TenantId,
        _display_number: DisplayNumber,
    ) -> Result<Option<User>, InfraError> {
        todo!()
    }

    async fn find_all_by_tenant(
        &self,
        _tenant_id: &TenantId,
        _status_filter: Option<UserStatus>,
    ) -> Result<Vec<User>, InfraError> {
        todo!()
    }

    async fn insert_user_role(
        &self,
        _user_id: &UserId,
        _role_id: &ringiflow_domain::role::RoleId,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        todo!()
    }

    async fn replace_user_roles(
        &self,
        _user_id: &UserId,
        _role_id: &ringiflow_domain::role::RoleId,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        todo!()
    }

    async fn find_role_by_name(
        &self,
        _name: &str,
    ) -> Result<Option<ringiflow_domain::role::Role>, InfraError> {
        todo!()
    }

    async fn count_active_users_with_role(
        &self,
        _tenant_id: &TenantId,
        _role_name: &str,
        _excluding_user_id: Option<&UserId>,
    ) -> Result<i64, InfraError> {
        todo!()
    }

    async fn find_roles_for_users(
        &self,
        _user_ids: &[UserId],
        _tenant_id: &TenantId,
    ) -> Result<std::collections::HashMap<UserId, Vec<String>>, InfraError> {
        todo!()
    }
}

struct StubTenantRepository {
    tenant: Option<Tenant>,
}

impl StubTenantRepository {
    fn with_tenant(tenant: Tenant) -> Self {
        Self {
            tenant: Some(tenant),
        }
    }

    fn empty() -> Self {
        Self { tenant: None }
    }
}

#[async_trait]
impl TenantRepository for StubTenantRepository {
    async fn find_by_id(&self, _id: &TenantId) -> Result<Option<Tenant>, InfraError> {
        Ok(self.tenant.clone())
    }
}

struct StubDisplayIdCounterRepository;

#[async_trait]
impl DisplayIdCounterRepository for StubDisplayIdCounterRepository {
    async fn next_display_number(
        &self,
        _tenant_id: &TenantId,
        _entity_type: DisplayIdEntityType,
    ) -> Result<DisplayNumber, InfraError> {
        todo!()
    }
}

struct StubClock;

impl Clock for StubClock {
    fn now(&self) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now()
    }
}

// テストデータ生成

fn create_active_user(tenant_id: &TenantId) -> User {
    User::from_db(
        UserId::new(),
        tenant_id.clone(),
        DisplayNumber::new(1).unwrap(),
        Email::new("user@example.com").unwrap(),
        UserName::new("Test User").unwrap(),
        UserStatus::Active,
        None,
        chrono::Utc::now(),
        chrono::Utc::now(),
    )
}

fn create_user_role() -> Role {
    Role::new_system(
        RoleId::new(),
        "user".to_string(),
        Some("一般ユーザー".to_string()),
        vec![
            Permission::new("workflow:read"),
            Permission::new("task:read"),
        ],
        chrono::Utc::now(),
    )
}

fn create_tenant(tenant_id: &TenantId) -> Tenant {
    Tenant::from_db(tenant_id.clone(), TenantName::new("Test Tenant").unwrap())
}

fn create_test_app(user_repo: StubUserRepository, tenant_repo: StubTenantRepository) -> Router {
    let user_repo_arc = Arc::new(user_repo) as Arc<dyn UserRepository>;
    let usecase = crate::usecase::UserUseCaseImpl::new(
        user_repo_arc.clone(),
        Arc::new(StubDisplayIdCounterRepository) as Arc<dyn DisplayIdCounterRepository>,
        Arc::new(StubClock) as Arc<dyn Clock>,
    );
    let state = Arc::new(UserState {
        user_repository: user_repo_arc,
        tenant_repository: Arc::new(tenant_repo) as Arc<dyn TenantRepository>,
        usecase,
    });

    Router::new()
        .route("/internal/users/by-email", get(get_user_by_email))
        .route("/internal/users/{user_id}", get(get_user))
        .with_state(state)
}

// テストケース

#[tokio::test]
async fn test_get_user_by_email_ユーザーが見つかる() {
    // Given
    let tenant_id = TenantId::new();
    let user = create_active_user(&tenant_id);
    let tenant = create_tenant(&tenant_id);
    let roles = vec![create_user_role()];
    let sut = create_test_app(
        StubUserRepository::with_user(user, roles),
        StubTenantRepository::with_tenant(tenant),
    );

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/users/by-email?email=user@example.com&tenant_id={}",
            tenant_id.as_uuid()
        ))
        .body(Body::empty())
        .unwrap();

    // When
    let response = sut.oneshot(request).await.unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["email"], "user@example.com");
    assert_eq!(json["data"]["status"], "active");
}

#[tokio::test]
async fn test_get_user_by_email_ユーザーが見つからない() {
    // Given
    let sut = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());
    let tenant_id = TenantId::new();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/users/by-email?email=notfound@example.com&tenant_id={}",
            tenant_id.as_uuid()
        ))
        .body(Body::empty())
        .unwrap();

    // When
    let response = sut.oneshot(request).await.unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_user_by_email_不正なメールアドレス() {
    // Given
    let sut = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());
    let tenant_id = TenantId::new();

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!(
            "/internal/users/by-email?email=invalid-email&tenant_id={}",
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
async fn test_get_user_ユーザー情報を取得できる() {
    // Given
    let tenant_id = TenantId::new();
    let user = create_active_user(&tenant_id);
    let user_id = *user.id().as_uuid();
    let tenant = create_tenant(&tenant_id);
    let roles = vec![create_user_role()];
    let sut = create_test_app(
        StubUserRepository::with_user(user, roles),
        StubTenantRepository::with_tenant(tenant),
    );

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/internal/users/{}", user_id))
        .body(Body::empty())
        .unwrap();

    // When
    let response = sut.oneshot(request).await.unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["data"]["tenant_name"], "Test Tenant");
}

#[tokio::test]
async fn test_get_user_存在しないユーザーで404() {
    // Given
    let sut = create_test_app(StubUserRepository::empty(), StubTenantRepository::empty());

    let user_id = Uuid::now_v7();
    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/internal/users/{}", user_id))
        .body(Body::empty())
        .unwrap();

    // When
    let response = sut.oneshot(request).await.unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_user_テナントが見つからない場合に500() {
    // Given
    let tenant_id = TenantId::new();
    let user = create_active_user(&tenant_id);
    let user_id = *user.id().as_uuid();
    let roles = vec![create_user_role()];
    // テナントリポジトリは空（テナントが見つからないケース）
    let sut = create_test_app(
        StubUserRepository::with_user(user, roles),
        StubTenantRepository::empty(),
    );

    let request = Request::builder()
        .method(Method::GET)
        .uri(format!("/internal/users/{}", user_id))
        .body(Body::empty())
        .unwrap();

    // When
    let response = sut.oneshot(request).await.unwrap();

    // Then
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// --- build_user_with_permissions テスト ---

#[test]
fn build_user_with_permissions_ロールありで権限が集約される() {
    let tenant_id = TenantId::new();
    let user = create_active_user(&tenant_id);
    let roles = vec![create_user_role()];

    let result = build_user_with_permissions(&user, &roles, "Test Tenant".to_string());

    assert_eq!(result.tenant_name, "Test Tenant");
    assert_eq!(result.roles, vec!["user"]);
    assert_eq!(result.permissions.len(), 2);
    assert!(result.permissions.contains(&"workflow:read".to_string()));
    assert!(result.permissions.contains(&"task:read".to_string()));
}

#[test]
fn build_user_with_permissions_ロール空で空リスト() {
    let tenant_id = TenantId::new();
    let user = create_active_user(&tenant_id);

    let result = build_user_with_permissions(&user, &[], "Test Tenant".to_string());

    assert!(result.roles.is_empty());
    assert!(result.permissions.is_empty());
}
