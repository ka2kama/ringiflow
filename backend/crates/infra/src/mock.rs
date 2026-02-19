//! # テスト用モックリポジトリ
//!
//! ユースケーステストで使用するインメモリモックリポジトリ。
//! `test-utils` feature を有効にすることで、他クレートからも利用可能。
//!
//! ```toml
//! [dev-dependencies]
//! ringiflow-infra = { workspace = true, features = ["test-utils"] }
//! ```

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use ringiflow_domain::{
    role::{Role, RoleId},
    tenant::TenantId,
    user::{Email, User, UserId, UserStatus},
    value_objects::{DisplayIdEntityType, DisplayNumber, Version},
    workflow::{
        WorkflowComment,
        WorkflowDefinition,
        WorkflowDefinitionId,
        WorkflowDefinitionStatus,
        WorkflowInstance,
        WorkflowInstanceId,
        WorkflowStep,
        WorkflowStepId,
    },
};

use crate::{
    db::{TransactionManager, TxContext},
    error::InfraError,
    repository::{
        DisplayIdCounterRepository,
        UserRepository,
        WorkflowCommentRepository,
        WorkflowDefinitionRepository,
        WorkflowInstanceRepository,
        WorkflowStepRepository,
    },
};

// ===== MockWorkflowDefinitionRepository =====

#[derive(Clone, Default)]
pub struct MockWorkflowDefinitionRepository {
    definitions: Arc<Mutex<Vec<WorkflowDefinition>>>,
}

impl MockWorkflowDefinitionRepository {
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_definition(&self, def: WorkflowDefinition) {
        self.definitions.lock().unwrap().push(def);
    }
}

#[async_trait]
impl WorkflowDefinitionRepository for MockWorkflowDefinitionRepository {
    async fn find_published_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowDefinition>, InfraError> {
        Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .filter(|d| {
                d.tenant_id() == tenant_id && d.status() == WorkflowDefinitionStatus::Published
            })
            .cloned()
            .collect())
    }

    async fn find_by_id(
        &self,
        id: &WorkflowDefinitionId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowDefinition>, InfraError> {
        Ok(self
            .definitions
            .lock()
            .unwrap()
            .iter()
            .find(|d| d.id() == id && d.tenant_id() == tenant_id)
            .cloned())
    }
}

// ===== MockWorkflowInstanceRepository =====

#[derive(Clone, Default)]
pub struct MockWorkflowInstanceRepository {
    instances: Arc<Mutex<Vec<WorkflowInstance>>>,
}

impl MockWorkflowInstanceRepository {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl WorkflowInstanceRepository for MockWorkflowInstanceRepository {
    async fn insert(&self, instance: &WorkflowInstance) -> Result<(), InfraError> {
        let mut instances = self.instances.lock().unwrap();
        instances.push(instance.clone());
        Ok(())
    }

    async fn update_with_version_check(
        &self,
        instance: &WorkflowInstance,
        expected_version: Version,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        let mut instances = self.instances.lock().unwrap();
        if let Some(pos) = instances.iter().position(|i| i.id() == instance.id()) {
            if instances[pos].version() != expected_version {
                return Err(InfraError::Conflict {
                    entity: "WorkflowInstance".to_string(),
                    id:     instance.id().as_uuid().to_string(),
                });
            }
            instances[pos] = instance.clone();
        }
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &WorkflowInstanceId,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowInstance>, InfraError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id() == id && i.tenant_id() == tenant_id)
            .cloned())
    }

    async fn find_by_tenant(
        &self,
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowInstance>, InfraError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.tenant_id() == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_initiated_by(
        &self,
        tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<WorkflowInstance>, InfraError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.tenant_id() == tenant_id && i.initiated_by() == user_id)
            .cloned()
            .collect())
    }

    async fn find_by_ids(
        &self,
        ids: &[WorkflowInstanceId],
        tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowInstance>, InfraError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .filter(|i| ids.contains(i.id()) && i.tenant_id() == tenant_id)
            .cloned()
            .collect())
    }

    async fn find_by_display_number(
        &self,
        display_number: DisplayNumber,
        tenant_id: &TenantId,
    ) -> Result<Option<WorkflowInstance>, InfraError> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.display_number() == display_number && i.tenant_id() == tenant_id)
            .cloned())
    }
}

// ===== MockWorkflowStepRepository =====

#[derive(Clone, Default)]
pub struct MockWorkflowStepRepository {
    steps: Arc<Mutex<Vec<WorkflowStep>>>,
}

impl MockWorkflowStepRepository {
    pub fn new() -> Self {
        Self {
            steps: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl WorkflowStepRepository for MockWorkflowStepRepository {
    async fn insert(&self, step: &WorkflowStep, _tenant_id: &TenantId) -> Result<(), InfraError> {
        let mut steps = self.steps.lock().unwrap();
        steps.push(step.clone());
        Ok(())
    }

    async fn update_with_version_check(
        &self,
        step: &WorkflowStep,
        expected_version: Version,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        let mut steps = self.steps.lock().unwrap();
        if let Some(pos) = steps.iter().position(|s| s.id() == step.id()) {
            if steps[pos].version() != expected_version {
                return Err(InfraError::Conflict {
                    entity: "WorkflowStep".to_string(),
                    id:     step.id().as_uuid().to_string(),
                });
            }
            steps[pos] = step.clone();
        }
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: &WorkflowStepId,
        _tenant_id: &TenantId,
    ) -> Result<Option<WorkflowStep>, InfraError> {
        Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.id() == id)
            .cloned())
    }

    async fn find_by_instance(
        &self,
        instance_id: &WorkflowInstanceId,
        _tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowStep>, InfraError> {
        Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.instance_id() == instance_id)
            .cloned()
            .collect())
    }

    async fn find_by_assigned_to(
        &self,
        _tenant_id: &TenantId,
        user_id: &UserId,
    ) -> Result<Vec<WorkflowStep>, InfraError> {
        Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.assigned_to() == Some(user_id))
            .cloned()
            .collect())
    }

    async fn find_by_display_number(
        &self,
        display_number: DisplayNumber,
        instance_id: &WorkflowInstanceId,
        _tenant_id: &TenantId,
    ) -> Result<Option<WorkflowStep>, InfraError> {
        Ok(self
            .steps
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.display_number() == display_number && s.instance_id() == instance_id)
            .cloned())
    }
}

// ===== MockUserRepository =====

/// テスト用のモック UserRepository
///
/// ユーザー名解決テストが必要な場合に使用する。
/// ワークフローユースケースのテストでは直接利用しないが、
/// 型パラメータを満たすために必要。
#[derive(Clone)]
pub struct MockUserRepository;

#[async_trait]
impl UserRepository for MockUserRepository {
    async fn find_by_email(
        &self,
        _tenant_id: &TenantId,
        _email: &Email,
    ) -> Result<Option<User>, InfraError> {
        Ok(None)
    }

    async fn find_by_id(&self, _id: &UserId) -> Result<Option<User>, InfraError> {
        Ok(None)
    }

    async fn find_with_roles(
        &self,
        _id: &UserId,
    ) -> Result<Option<(User, Vec<ringiflow_domain::role::Role>)>, InfraError> {
        Ok(None)
    }

    async fn find_by_ids(&self, _ids: &[UserId]) -> Result<Vec<User>, InfraError> {
        Ok(Vec::new())
    }

    async fn find_all_active_by_tenant(
        &self,
        _tenant_id: &TenantId,
    ) -> Result<Vec<User>, InfraError> {
        Ok(Vec::new())
    }

    async fn update_last_login(&self, _id: &UserId) -> Result<(), InfraError> {
        Ok(())
    }

    async fn insert(&self, _user: &User) -> Result<(), InfraError> {
        Ok(())
    }

    async fn update(&self, _user: &User) -> Result<(), InfraError> {
        Ok(())
    }

    async fn update_status(&self, _user: &User) -> Result<(), InfraError> {
        Ok(())
    }

    async fn find_by_display_number(
        &self,
        _tenant_id: &TenantId,
        _display_number: DisplayNumber,
    ) -> Result<Option<User>, InfraError> {
        Ok(None)
    }

    async fn find_all_by_tenant(
        &self,
        _tenant_id: &TenantId,
        _status_filter: Option<UserStatus>,
    ) -> Result<Vec<User>, InfraError> {
        Ok(Vec::new())
    }

    async fn insert_user_role(
        &self,
        _user_id: &UserId,
        _role_id: &RoleId,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        Ok(())
    }

    async fn replace_user_roles(
        &self,
        _user_id: &UserId,
        _role_id: &RoleId,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        Ok(())
    }

    async fn find_role_by_name(&self, _name: &str) -> Result<Option<Role>, InfraError> {
        Ok(None)
    }

    async fn count_active_users_with_role(
        &self,
        _tenant_id: &TenantId,
        _role_name: &str,
        _excluding_user_id: Option<&UserId>,
    ) -> Result<i64, InfraError> {
        Ok(0)
    }

    async fn find_roles_for_users(
        &self,
        _user_ids: &[UserId],
        _tenant_id: &TenantId,
    ) -> Result<HashMap<UserId, Vec<String>>, InfraError> {
        Ok(HashMap::new())
    }
}

// ===== MockDisplayIdCounterRepository =====

/// テスト用のモック DisplayIdCounterRepository
///
/// 呼び出しごとにカウンターをインクリメントして返す。
#[derive(Clone, Default)]
pub struct MockDisplayIdCounterRepository {
    counter: Arc<Mutex<i64>>,
}

impl MockDisplayIdCounterRepository {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }
}

#[async_trait]
impl DisplayIdCounterRepository for MockDisplayIdCounterRepository {
    async fn next_display_number(
        &self,
        _tenant_id: &TenantId,
        _entity_type: DisplayIdEntityType,
    ) -> Result<DisplayNumber, InfraError> {
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        Ok(DisplayNumber::new(*counter).unwrap())
    }
}

// ===== MockWorkflowCommentRepository =====

#[derive(Clone, Default)]
pub struct MockWorkflowCommentRepository {
    comments: Arc<Mutex<Vec<WorkflowComment>>>,
}

impl MockWorkflowCommentRepository {
    pub fn new() -> Self {
        Self {
            comments: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

#[async_trait]
impl WorkflowCommentRepository for MockWorkflowCommentRepository {
    async fn insert(
        &self,
        comment: &WorkflowComment,
        _tenant_id: &TenantId,
    ) -> Result<(), InfraError> {
        let mut comments = self.comments.lock().unwrap();
        comments.push(comment.clone());
        Ok(())
    }

    async fn find_by_instance(
        &self,
        instance_id: &WorkflowInstanceId,
        _tenant_id: &TenantId,
    ) -> Result<Vec<WorkflowComment>, InfraError> {
        let comments = self.comments.lock().unwrap();
        let mut result: Vec<_> = comments
            .iter()
            .filter(|c| c.instance_id() == instance_id)
            .cloned()
            .collect();
        result.sort_by_key(|c| c.created_at());
        Ok(result)
    }
}

// ===== MockTransactionManager =====

/// テスト用の MockTransactionManager
///
/// `begin()` は常に `TxContext::mock()` を返す。
/// Mock リポジトリはインメモリ実装のため、実際のトランザクションは不要。
pub struct MockTransactionManager;

#[async_trait]
impl TransactionManager for MockTransactionManager {
    async fn begin(&self) -> Result<TxContext, InfraError> {
        Ok(TxContext::mock())
    }
}
