# #723 ワークフロー定義の CRUD API 実装計画

## Context

ワークフローデザイナー GUI の前提として、ワークフロー定義を API 経由で CRUD 管理できるようにする。#722 で機能仕様書・詳細設計書・ADR-053 が整備済み。現状は GET（一覧・詳細）のみ実装されており、作成・更新・削除・公開・アーカイブ・バリデーションのエンドポイントを追加する。

→ 詳細設計: `docs/03_詳細設計書/15_ワークフローデザイナー設計.md`

## 対象

- ドメインモデル: `update()`, `can_delete()`, `can_archive()`, `can_publish()` 修正, `validate_definition()`
- リポジトリ: `insert()`, `update_with_version_check()`, `delete()`, `find_all_by_tenant()`
- ユースケース: 新規 `WorkflowDefinitionUseCaseImpl`（CRUD + publish/archive/validate）
- Core Service: 6 つの新規ハンドラ + ルート
- BFF: クライアント拡張 + 6 つの新規ハンドラ + ルート（`workflow_definition:manage` 権限）
- テスト: ユニット + ハンドラ + API

## 対象外

- フロントエンド（Elm）の変更
- デザイナー UI
- E2E テスト（UI がないため）

## 設計判断

### ユースケースの分離

`WorkflowUseCaseImpl`（インスタンス操作）とは別に `WorkflowDefinitionUseCaseImpl` を新設する。

理由: SRP。定義管理はインスタンス操作と責務が異なる。既存の `RoleUseCaseImpl` が独立しているパターンに倣う。

### リポジトリ操作のトランザクション

定義 CRUD は単一テーブルの単一行操作のため、`TxContext` なしでプールから直接実行する。ロールリポジトリ（`RoleRepository`）と同じパターン。

理由: `WorkflowInstanceRepository.insert()` が `TxContext` を使うのは、インスタンス + ステップの複数テーブル操作が必要なため。定義の単一行操作には不要。

### `can_publish()` のバグ修正

現在の `can_publish()` は「既に Published」のみチェックしており、Archived → Published 遷移を許可してしまう。詳細設計の通り Draft のみ許可に修正する（セッションログで既に記録済み）。

### `find_all_by_tenant()` の追加

既存の `find_published_by_tenant()` は Published のみ返す（申請時の定義選択用）。管理者向けのデザイナー一覧では Draft/Published/Archived すべて返す必要があるため、`find_all_by_tenant()` を追加する。

### 楽観的ロック

既存の `Version` 値オブジェクトと `rows_affected() == 0 → InfraError::Conflict` パターンに従う。publish/archive でもバージョンチェックを行う。

### DELETE エンドポイント

Issue の完了基準に明示されていないが、詳細設計に含まれており CRUD の "D" として自然。Draft のみ削除可能。

---

## Phase 1: ドメインモデルの拡張

`backend/crates/domain/src/workflow/definition.rs`

### 変更内容

1. `can_publish()` の修正: Draft のみ許可（Archived を拒否）
2. `can_archive()` の追加: Published のみアーカイブ可能
3. `can_delete()` の追加: Draft のみ削除可能
4. `update()` の追加: Draft のみ更新可能、version を next() で更新
5. `archived()` の修正: `can_archive()` チェックを追加し、`Result` を返すように変更

```rust
pub fn can_publish(&self) -> Result<(), DomainError> {
    if self.status != WorkflowDefinitionStatus::Draft {
        return Err(DomainError::Validation("下書き状態の定義のみ公開できます".to_string()));
    }
    Ok(())
}

pub fn can_delete(&self) -> Result<(), DomainError> {
    if self.status != WorkflowDefinitionStatus::Draft {
        return Err(DomainError::Validation("下書き状態の定義のみ削除できます".to_string()));
    }
    Ok(())
}

pub fn can_archive(&self) -> Result<(), DomainError> {
    if self.status != WorkflowDefinitionStatus::Published {
        return Err(DomainError::Validation("公開済みの定義のみアーカイブできます".to_string()));
    }
    Ok(())
}

pub fn update(
    self,
    name: WorkflowName,
    description: Option<String>,
    definition: JsonValue,
    now: DateTime<Utc>,
) -> Result<Self, DomainError> {
    if self.status != WorkflowDefinitionStatus::Draft {
        return Err(DomainError::Validation("下書き状態の定義のみ更新できます".to_string()));
    }
    Ok(Self {
        name,
        description,
        definition,
        version: self.version.next(),
        updated_at: now,
        ..self
    })
}

pub fn archived(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
    self.can_archive()?;
    Ok(Self {
        status: WorkflowDefinitionStatus::Archived,
        version: self.version.next(),
        updated_at: now,
        ..self
    })
}
```

注意: `archived()` の戻り型が `Self` → `Result<Self, DomainError>` に変わるため、既存の呼び出し箇所を修正する必要がある。また、`published()` と `archived()` で version を next() に変更する（詳細設計の楽観的ロック仕様）。

### 確認事項

- [x] 型: `Version::next()` → `value_objects.rs` L64, `self.0 + 1` を返す
- [x] パターン: 既存の `can_publish()` / `published()` → `definition.rs` L173-188
- [x] 影響: `archived()` の戻り型変更 → Grep で既存呼び出しを探す

### テストリスト

ユニットテスト:
- [ ] Draft 定義を更新できる
- [ ] Published 定義の更新はエラー
- [ ] Archived 定義の更新はエラー
- [ ] Draft 定義を公開できる（`can_publish()` 修正後）
- [ ] Published 定義の再公開はエラー
- [ ] Archived 定義の公開はエラー（修正: 以前は許可されていた）
- [ ] Published 定義をアーカイブできる
- [ ] Draft 定義のアーカイブはエラー
- [ ] Archived 定義の再アーカイブはエラー
- [ ] Draft 定義を削除可能チェックが成功
- [ ] Published 定義の削除チェックはエラー
- [ ] update() でバージョンがインクリメントされる

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 2: バリデーションロジック

`backend/crates/domain/src/workflow/definition_validator.rs`（新規ファイル）

### 変更内容

`validate_definition()` 関数と関連型を新規モジュールに実装する。10 のバリデーションルールを段階的に実装。

```rust
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_id: Option<String>,
}

pub fn validate_definition(definition: &JsonValue) -> ValidationResult { ... }
```

10 ルール:
1. `missing_start_step` — start ステップが正確に 1 つ
2. `multiple_start_steps` — start ステップが 2 つ以上
3. `missing_end_step` — end ステップが 1 つ以上
4. `missing_approval_step` — approval ステップが 1 つ以上
5. `orphaned_step` — 孤立ステップなし
6. `cycle_detected` — DAG（循環なし）
7. `missing_approval_transition` — approval から approve/reject 両方の遷移
8. `duplicate_step_id` — ステップ ID 重複なし
9. `invalid_transition_ref` — 遷移が有効なステップを参照
10. `invalid_form_field` — フォームフィールドが有効

### 確認事項

- [ ] 型: `serde_json::Value` の操作パターン → 既存の `extract_approval_steps()` を参照
- [ ] パターン: cycle detection — DAG でのトポロジカルソートまたは DFS で実装

### テストリスト

ユニットテスト:
- [ ] 有効な定義でバリデーション成功
- [ ] 各ルール × 正常系・異常系（10 ルール × 2 = 20 テスト程度）
- [ ] 複数エラーが同時に返される

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 3: リポジトリの拡張

`backend/crates/infra/src/repository/workflow_definition_repository.rs`

### 変更内容

トレイトに 4 メソッド追加:

```rust
#[async_trait]
pub trait WorkflowDefinitionRepository: Send + Sync {
    // 既存
    async fn find_published_by_tenant(...) -> ...;
    async fn find_by_id(...) -> ...;
    // 新規
    async fn find_all_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<WorkflowDefinition>, InfraError>;
    async fn insert(&self, definition: &WorkflowDefinition) -> Result<(), InfraError>;
    async fn update_with_version_check(&self, definition: &WorkflowDefinition, expected_version: Version) -> Result<(), InfraError>;
    async fn delete(&self, id: &WorkflowDefinitionId, tenant_id: &TenantId) -> Result<(), InfraError>;
}
```

PostgreSQL 実装: `RoleRepository` パターンに従い、プール直接操作（`TxContext` なし）。

`update_with_version_check` は `rows_affected() == 0` で `InfraError::Conflict` を返す。

モック更新: `backend/crates/infra/src/mock.rs` の `MockWorkflowDefinitionRepository` に 4 メソッドを追加。

### 確認事項

- [ ] パターン: `RoleRepository::insert()` の sqlx パターン → `role_repository.rs`
- [ ] パターン: `WorkflowInstanceRepository::update_with_version_check()` → `rows_affected()` チェック
- [ ] 型: `InfraError::Conflict { entity, id }` → `error.rs`

### テストリスト

ユニットテスト:
- [ ] トレイトが Send + Sync を実装（既存テスト拡張）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 4: ユースケースの実装

`backend/apps/core-service/src/usecase/workflow_definition.rs`（新規ファイル）

### 変更内容

`WorkflowDefinitionUseCaseImpl` を新設:

```rust
pub struct WorkflowDefinitionUseCaseImpl {
    definition_repo: Arc<dyn WorkflowDefinitionRepository>,
    clock: Arc<dyn Clock>,
}
```

メソッド:
- `list_definitions(tenant_id)` — `find_all_by_tenant()` で全ステータス返却
- `get_definition(id, tenant_id)` — `find_by_id()` + NotFound
- `create_definition(name, description, definition, tenant_id, user_id)` — `new()` + `insert()`
- `update_definition(id, name, description, definition, expected_version, tenant_id)` — `find_by_id()` + `update()` + `update_with_version_check()`
- `delete_definition(id, tenant_id)` — `find_by_id()` + `can_delete()` + `delete()`
- `publish_definition(id, expected_version, tenant_id)` — `find_by_id()` + `validate_definition()` + `published()` + `update_with_version_check()`
- `archive_definition(id, expected_version, tenant_id)` — `find_by_id()` + `archived()` + `update_with_version_check()`
- `validate_definition_json(definition)` — `validate_definition()` をラップ

エラーマッピング: `InfraError::Conflict` → `CoreError::Conflict`

### 確認事項

- [ ] パターン: `RoleUseCaseImpl` の構造 → `usecase/role.rs`
- [ ] パターン: `FindResultExt::or_not_found()` → `usecase/helpers.rs`
- [ ] パターン: エラーマッピング（InfraError → CoreError） → `approve.rs` L123-131

### テストリスト

ユニットテスト（モックリポジトリ使用）:
- [ ] 定義作成が成功し Draft 状態で保存される
- [ ] Draft 定義の更新が成功しバージョンがインクリメントされる
- [ ] Published 定義の更新がエラーを返す
- [ ] Draft 定義の削除が成功する
- [ ] Published 定義の削除がエラーを返す
- [ ] バリデーション成功後に公開が成功する
- [ ] バリデーション失敗で公開がエラーを返す
- [ ] Published 定義のアーカイブが成功する
- [ ] Draft 定義のアーカイブがエラーを返す
- [ ] 存在しない定義の操作が NotFound を返す

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 5: Core Service ハンドラ

`backend/apps/core-service/src/handler/workflow_definition/`（新規ディレクトリ）

### 変更内容

新規ハンドラモジュール:
- `mod.rs` — State, リクエスト/レスポンス型, DTO
- `command.rs` — POST/PUT/DELETE ハンドラ
- `query.rs` — GET ハンドラ（既存を移動 or 新規追加）

リクエスト型:

```rust
#[derive(Debug, Deserialize)]
pub struct CreateDefinitionRequest {
    pub name: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub tenant_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDefinitionRequest {
    pub name: String,
    pub description: Option<String>,
    pub definition: serde_json::Value,
    pub version: i32,
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct PublishArchiveRequest {
    pub version: i32,
    pub tenant_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ValidateDefinitionRequest {
    pub definition: serde_json::Value,
}
```

State:

```rust
pub struct WorkflowDefinitionState {
    pub usecase: WorkflowDefinitionUseCaseImpl,
}
```

ルート登録（`main.rs`）:

```rust
// ワークフロー定義管理 API
.route("/internal/workflow-definitions", get(list_definitions).post(create_definition))
.route(
    "/internal/workflow-definitions/{id}",
    get(get_definition).put(update_definition).delete(delete_definition),
)
.route("/internal/workflow-definitions/{id}/publish", post(publish_definition))
.route("/internal/workflow-definitions/{id}/archive", post(archive_definition))
.route("/internal/workflow-definitions/{id}/validate", post(validate_definition))
.with_state(definition_state)
```

注意: 既存の GET ハンドラ（`list_workflow_definitions`, `get_workflow_definition`）は `workflow_state` に紐づいている。新しい `definition_state` に移行するか、GET ハンドラを新旧で共存させるか検討が必要。

設計判断: 既存の GET ハンドラを `WorkflowState` から分離し、`WorkflowDefinitionState` に統合する。これにより definition 関連のルートが一箇所にまとまる。既存の `WorkflowUseCaseImpl` からは `definition_repo` を引き続き参照する（ワークフロー作成時に定義を参照するため）。

### 確認事項

- [ ] パターン: Core Service ハンドラの構造 → `handler/workflow/command.rs`
- [ ] パターン: ルート登録 → `main.rs` L270-275（既存の definition ルート）
- [ ] 影響: 既存 GET ハンドラの移動 → `handler/workflow/query.rs` の `list_workflow_definitions`, `get_workflow_definition`

### テストリスト

ユニットテスト（該当なし — ハンドラは薄いため）

ハンドラテスト（Stub リポジトリ + Router + oneshot）:
- [ ] POST /internal/workflow-definitions → 201 Created
- [ ] PUT /internal/workflow-definitions/{id} → 200 OK
- [ ] DELETE /internal/workflow-definitions/{id} → 204 No Content
- [ ] POST .../publish → 200 OK（バリデーション成功時）
- [ ] POST .../publish → 400（バリデーション失敗時）
- [ ] POST .../archive → 200 OK
- [ ] PUT（バージョン不一致） → 409 Conflict
- [ ] PUT（Published 定義） → 400 Bad Request
- [ ] DELETE（Published 定義） → 400 Bad Request

API テスト（該当なし — Phase 7 で実施）

E2E テスト（該当なし）

---

## Phase 6: BFF 層

### 変更内容

#### 6-1. BFF クライアント拡張

`backend/apps/bff/src/client/core_service/workflow_client.rs` にメソッド追加:

```rust
// CoreServiceWorkflowClient トレイトに追加
async fn create_workflow_definition(...) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError>;
async fn update_workflow_definition(...) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError>;
async fn delete_workflow_definition(...) -> Result<(), CoreServiceError>;
async fn publish_workflow_definition(...) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError>;
async fn archive_workflow_definition(...) -> Result<ApiResponse<WorkflowDefinitionDto>, CoreServiceError>;
async fn validate_workflow_definition(...) -> Result<ApiResponse<ValidationResultDto>, CoreServiceError>;
```

BFF types にリクエスト/レスポンス型を追加:
- `CreateDefinitionBffRequest`, `UpdateDefinitionBffRequest`, `PublishArchiveVersionRequest`
- `ValidationResultDto`, `ValidationErrorDto`

#### 6-2. BFF ハンドラ

`backend/apps/bff/src/handler/workflow_definition.rs`（新規ファイル）

ハンドラパターン: `authenticate()` → リクエスト変換 → クライアント呼び出し → レスポンス変換。ロールハンドラのパターンに従う。

#### 6-3. BFF ルート登録

`backend/apps/bff/src/main.rs` に権限付きルートを追加:

```rust
// ワークフロー定義管理 API（workflow_definition:manage 権限）
let definition_manage_authz = AuthzState {
    session_manager: session_manager.clone(),
    required_permission: "workflow_definition:manage".to_string(),
};

.merge(
    Router::new()
        .route("/api/v1/workflow-definitions", post(create_definition))
        .route(
            "/api/v1/workflow-definitions/{id}",
            put(update_definition).delete(delete_definition),
        )
        .route("/api/v1/workflow-definitions/{id}/publish", post(publish_definition))
        .route("/api/v1/workflow-definitions/{id}/archive", post(archive_definition))
        .route("/api/v1/workflow-definitions/{id}/validate", post(validate_definition))
        .layer(from_fn_with_state(definition_manage_authz, require_permission))
        .with_state(workflow_definition_state),
)
```

注意: 既存の GET（一覧・詳細）は認可不要（全ユーザーアクセス可能）なので、既存の `workflow_state` ルートグループに残す。

#### 6-4. 権限の DB シード

`workflow_definition:manage` 権限を管理者ロールに追加する必要がある。DB マイグレーションまたはシードデータの更新。

### 確認事項

- [ ] パターン: BFF ロールハンドラ → `handler/role.rs` の `create_role()`, `update_role()`, `delete_role()`
- [ ] パターン: BFF クライアント → `client/core_service/role_client.rs`
- [ ] パターン: AuthzState + require_permission → `main.rs` L328-373
- [ ] パターン: CoreServiceError バリアント → `client/core_service/error.rs`
- [ ] 型: `WorkflowDefinitionData` → 既存 `handler/workflow.rs` L224-251（拡張が必要か確認）

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし — BFF テストは統合テストで実施）

API テスト（Phase 7 で実施）

E2E テスト（該当なし）

---

## Phase 7: API テスト

`tests/api/` に統合テストを追加。

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（Core Service ハンドラ統合テスト + BFF 認可テスト）:
- [x] 定義の作成 → 取得で確認
- [x] 定義の更新 → 取得で確認
- [x] 定義の作成 → 公開 → 一覧で Published 確認
- [x] 公開 → アーカイブ → ステータス確認
- [x] Draft 以外の更新・削除が拒否される
- [x] バージョン競合で 409 が返る
- [x] 権限のないユーザーで 403 が返る（BFF authz テスト）

E2E テスト（該当なし — UI なし）

---

## 主要ファイル一覧

| ファイル | 操作 |
|---------|------|
| `backend/crates/domain/src/workflow/definition.rs` | 変更 |
| `backend/crates/domain/src/workflow/definition_validator.rs` | 新規 |
| `backend/crates/domain/src/workflow/mod.rs` | 変更（module 追加） |
| `backend/crates/infra/src/repository/workflow_definition_repository.rs` | 変更 |
| `backend/crates/infra/src/mock.rs` | 変更 |
| `backend/apps/core-service/src/usecase/workflow_definition.rs` | 新規 |
| `backend/apps/core-service/src/usecase/mod.rs` | 変更 |
| `backend/apps/core-service/src/handler/workflow_definition/mod.rs` | 新規 |
| `backend/apps/core-service/src/handler/workflow_definition/command.rs` | 新規 |
| `backend/apps/core-service/src/handler/workflow_definition/query.rs` | 新規 |
| `backend/apps/core-service/src/handler/mod.rs` | 変更 |
| `backend/apps/core-service/src/main.rs` | 変更 |
| `backend/apps/bff/src/client/core_service/workflow_client.rs` | 変更 |
| `backend/apps/bff/src/client/core_service/types.rs` | 変更 |
| `backend/apps/bff/src/handler/workflow_definition.rs` | 新規 |
| `backend/apps/bff/src/handler/mod.rs` | 変更 |
| `backend/apps/bff/src/main.rs` | 変更 |

## 検証方法

1. 各 Phase でユニットテスト: `cd backend && cargo test`
2. Phase 5 でハンドラテスト: `cd backend && cargo test --package ringiflow-core-service`
3. Phase 7 で API テスト: `just test-rust-integration`
4. 全体: `just check-all`

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `archived()` の戻り型変更が既存コードに影響する | 不完全なパス | Phase 1 に `archived()` の呼び出し箇所修正を含める。確認事項に Grep を追加 |
| 2回目 | `can_publish()` が Archived → Published を許可するバグ（セッションログ記録済み）| 欠陥の発見 | Phase 1 で修正。テストリストに「Archived 定義の公開はエラー」を追加 |
| 3回目 | `published()` / `archived()` で version を next() すべきか | 曖昧 | 詳細設計の楽観的ロック仕様で「更新時 version + 1」と明記。状態遷移も version 更新を含める |
| 4回目 | 既存 GET ハンドラの `WorkflowState` 依存 | アーキテクチャ不整合 | Phase 5 で `WorkflowDefinitionState` に統合する設計判断を追加。既存 list/get ハンドラを移動 |
| 5回目 | `workflow_definition:manage` 権限の DB シード | 未定義 | Phase 6 に権限シード追加を明記 |
| 6回目 | validate エンドポイントのレスポンス型が他と異なる（200 OK + valid/errors） | 既存手段の見落とし | BFF types に `ValidationResultDto` を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全エンドポイント（8 個中 GET 2 個は既存、新規 6 個）が計画に含まれている | OK | 詳細設計の API 一覧と突合。DELETE, validate 含め全エンドポイントを網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更内容をコードレベルで記述。DTO 変換、エラーマッピングも明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | ユースケース分離、トランザクション、GET ハンドラ移動、権限シードを記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外: フロントエンド、デザイナー UI、E2E テスト |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | 楽観的ロック、`archived()` 戻り型変更の影響、権限シードを確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 詳細設計書の API 仕様、セッションログの `can_publish()` バグ記録と照合 |
