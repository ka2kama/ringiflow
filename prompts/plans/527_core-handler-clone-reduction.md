# Issue #527: Core Service ハンドラ層のクローン削減・ファイルサイズ削減

## コンテキスト

Epic #467（jscpd コードクローン警告ゼロ化）の Story 4。Core Service ハンドラ層に約35のコードクローンが存在し、3つのファイルが500行閾値を超過している。クローン削減と責務分離により保守性を向上させる。

## スコープ

**対象:**
- Core Service handler のクローン削減（workflow/command.rs, query.rs, auth.rs, role.rs, task.rs）
- ファイルサイズ削減: command.rs (603行), Core auth.rs (965行), BFF auth.rs (1075行)

**対象外:**
- Core Service handler types ↔ BFF client types の重複（Story 7 #530 で対応）
  - task.rs L46-56 ↔ bff/client/core_service/types.rs L200-210
  - workflow.rs 型定義 ↔ bff/client/core_service/types.rs

## 設計判断

### 1. ヘルパー関数の配置先: `workflow.rs` に追加

`to_user_ref()` が既に `workflow.rs` に配置されている前例に従う。追加するのは変換ヘルパー数個のみであり、新ファイル作成は不要。

### 2. RoleDetailDto: `From<&Role>` trait impl

`From<WorkflowDefinition> for WorkflowDefinitionDto`（workflow.rs L160）の前例に従い、慣用的な `From` trait で統一する。

### 3. レスポンス構築ヘルパー: ジェネリック関数化しない

`ApiResponse::new(dto)` + `(StatusCode::XX, Json(response)).into_response()` は各2行で十分短い。StatusCode が CREATED/OK で分岐するケースがあり、抽象化すると引数が増えてかえって読みにくくなる。過度な DRY を避ける原則に沿う。

### 4. BFF auth.rs 分割: login+logout / me+csrf

login と logout は認証状態の変更（Cookie 設定/クリア）、me と csrf はセッション参照（read-only）。CQRS 的な責務分離と一致。テストスタブは `mod.rs` の `#[cfg(test)]` で共有する。

### 5. Core auth.rs: テストモジュール分離 + ヘルパー抽出

Core auth.rs（965行 = ハンドラ ~556行 + テスト ~409行）は、テストモジュールを `auth/tests.rs` に分離しハンドラコードを `mod.rs` に残す。`build_user_with_permissions` ヘルパー抽出でクローンも削減する。

---

## Phase 1: Workflow VO parse ヘルパー + convert_approvers 抽出

command.rs のボイラープレートを集約する基盤ヘルパーを `workflow.rs` に追加。

### 変更ファイル
- `backend/apps/core-service/src/handler/workflow.rs` — ヘルパー関数3つ追加
- `backend/apps/core-service/src/handler/workflow/command.rs` — ヘルパー呼び出しに変更
- `backend/apps/core-service/src/handler/workflow/query.rs` — parse_display_number 呼び出しに変更

### ヘルパー関数

```rust
// workflow.rs に追加

/// i64 を DisplayNumber に変換する。
/// 不正な値の場合は CoreError::BadRequest を返す。
pub(crate) fn parse_display_number(value: i64, field: &str) -> Result<DisplayNumber, CoreError> {
    DisplayNumber::try_from(value)
        .map_err(|e| CoreError::BadRequest(format!("不正な {field}: {e}")))
}

/// i32 を Version に変換する。
pub(crate) fn parse_version(value: i32) -> Result<Version, CoreError> {
    Version::try_from(value)
        .map_err(|e| CoreError::BadRequest(format!("不正なバージョン: {e}")))
}

/// StepApproverRequest のリストを StepApprover のリストに変換する。
pub(crate) fn convert_approvers(approvers: Vec<StepApproverRequest>) -> Vec<StepApprover> {
    approvers
        .into_iter()
        .map(|a| StepApprover {
            step_id:     a.step_id,
            assigned_to: UserId::from_uuid(a.assigned_to),
        })
        .collect()
}
```

### Before/After 例

```rust
// Before (command.rs L239-240)
let display_number = DisplayNumber::try_from(display_number)
    .map_err(|e| CoreError::BadRequest(format!("不正な display_number: {}", e)))?;

// After
let display_number = parse_display_number(display_number, "display_number")?;
```

```rust
// Before (command.rs L284-287 — 2つの DisplayNumber 変換)
let workflow_display_number = DisplayNumber::try_from(params.display_number)
    .map_err(|e| CoreError::BadRequest(format!("不正な display_number: {}", e)))?;
let step_display_number = DisplayNumber::try_from(params.step_display_number)
    .map_err(|e| CoreError::BadRequest(format!("不正な step_display_number: {}", e)))?;

// After
let workflow_display_number = parse_display_number(params.display_number, "display_number")?;
let step_display_number = parse_display_number(params.step_display_number, "step_display_number")?;
```

```rust
// Before (command.rs L104-112)
let input = SubmitWorkflowInput {
    approvers: req
        .approvers
        .into_iter()
        .map(|a| StepApprover {
            step_id:     a.step_id,
            assigned_to: UserId::from_uuid(a.assigned_to),
        })
        .collect(),
};

// After
let input = SubmitWorkflowInput {
    approvers: convert_approvers(req.approvers),
};
```

### 削減箇所

| ヘルパー | command.rs | query.rs | 計 |
|---------|-----------|---------|---|
| `parse_display_number` | 9箇所 | 2箇所 | 11 |
| `parse_version` | 8箇所 | 0 | 8 |
| `convert_approvers` | 4箇所 | 0 | 4 |

### 確認事項
- [x] 型: `DisplayNumber::try_from(i64)` のエラー型 → `DomainError::Validation` (value_objects.rs L191-201)、0以下でエラー
- [x] 型: `Version::try_from(i32)` のエラー型 → `DomainError::Validation` (value_objects.rs L108-123)、0以下でエラー
- [x] 型: `StepApprover` のフィールド → `step_id: String, assigned_to: UserId` (usecase/workflow.rs L53-58)
- [x] パターン: `StepApproverRequest` のフィールド → `step_id: String, assigned_to: Uuid` (workflow.rs L42-48)
- [x] ライブラリ: `DisplayNumber::try_from` の既存使用 → command.rs 9箇所、query.rs 2箇所で確認済み

### テストリスト

ユニットテスト:
- [x] `parse_display_number` 正常系: 正の整数で `Ok(DisplayNumber)` を返す
- [x] `parse_display_number` 異常系: 0以下で `CoreError::BadRequest` を返す（エラーメッセージにフィールド名を含む）
- [x] `parse_version` 正常系: 正の整数で `Ok(Version)` を返す
- [x] `parse_version` 異常系: 0以下で `CoreError::BadRequest` を返す
- [x] `convert_approvers` 正常系: 空リスト → 空リスト
- [x] `convert_approvers` 正常系: 複数要素 → 正しく変換される

ハンドラテスト（該当なし — 既存テストのパスで確認 ✅）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 2: Workflow ユーザー名解決メソッド抽出

「collect_user_ids → resolve_user_names → build DTO」の3段階フローを `WorkflowInstanceDto` の async メソッドに集約する。

### 変更ファイル
- `backend/apps/core-service/src/handler/workflow.rs` — resolve メソッド2つ追加
- `backend/apps/core-service/src/handler/workflow/command.rs` — resolve メソッド呼び出しに変更
- `backend/apps/core-service/src/handler/workflow/query.rs` — resolve メソッド呼び出しに変更

### メソッドシグネチャ

```rust
// workflow.rs の impl WorkflowInstanceDto に追加

/// WorkflowInstance からユーザー名を解決し、DTO を構築する（ステップなし）
fn resolve_from_instance(
    instance: &WorkflowInstance,
    usecase: &WorkflowUseCaseImpl,
) -> Result<Self, CoreError> { ... }

/// WorkflowWithSteps からユーザー名を解決し、DTO を構築する
pub(crate) async fn resolve_from_workflow_with_steps(
    data: &WorkflowWithSteps,
    usecase: &WorkflowUseCaseImpl,
) -> Result<Self, CoreError> { ... }
```

注: `resolve_from_instance` は `async` で、内部で `usecase.resolve_user_names()` を呼ぶ。可視性は既存の `from_instance`（private = workflow モジュール + 子モジュール）に合わせる。`resolve_from_workflow_with_steps` は task.rs からも使われる可能性があるため `pub(crate)`。

### Before/After 例

```rust
// Before (command.rs L161-173, approve_step 末尾)
let user_ids = crate::usecase::workflow::collect_user_ids_from_workflow(
    &workflow_with_steps.instance,
    &workflow_with_steps.steps,
);
let user_names = state.usecase.resolve_user_names(&user_ids).await?;
let response = ApiResponse::new(WorkflowInstanceDto::from_workflow_with_steps(
    &workflow_with_steps,
    &user_names,
));

// After
let dto = WorkflowInstanceDto::resolve_from_workflow_with_steps(
    &workflow_with_steps,
    &state.usecase,
).await?;
let response = ApiResponse::new(dto);
```

### 削減箇所

| メソッド | command.rs | query.rs | 計 |
|---------|-----------|---------|---|
| `resolve_from_instance` | 3箇所 (create, submit, submit_by_dn) | 0 | 3 |
| `resolve_from_workflow_with_steps` | 8箇所 (approve/reject/request_changes/resubmit × UUID/DN) | 2箇所 (get, get_by_dn) | 10 |

注: 以下は異なるパターンのため対象外:
- `list_my_workflows` (query.rs L94-120): 複数インスタンスの `initiated_by` を HashSet で収集する一覧用パターン
- `list_comments` (query.rs L208-239): `WorkflowCommentDto` を使用するコメント一覧用パターン
- `post_comment` (command.rs L579-603): 単一コメント投稿者のみ

### 確認事項
- [x] 型: `WorkflowUseCaseImpl::resolve_user_names(&self, &[UserId]) -> Result<HashMap<UserId, String>, CoreError>` (usecase/workflow.rs L144-148)
- [x] 型: `collect_user_ids_from_workflow(instance: &WorkflowInstance, steps: &[WorkflowStep]) -> Vec<UserId>` (usecase/workflow.rs L91-98)
- [x] パターン: `from_instance` は private（fn、pub(crate) でない）、子モジュールからアクセス可能 (workflow.rs L243)

### テストリスト

ユニットテスト（該当なし — resolve メソッドは既存の from_instance / from_workflow_with_steps + usecase.resolve_user_names の薄い合成。各部品は既にテスト済み。既存ハンドラテストのパスで振る舞いを確認）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 3: Role/Auth クローン削減

### 3a: RoleDetailDto に `From<&Role>` を実装

role.rs の get_role, create_role, update_role で繰り返される7フィールドの DTO 構築を統一する。

### 変更ファイル
- `backend/apps/core-service/src/handler/role.rs` — `From<&Role>` impl 追加、3箇所を置換

```rust
impl From<&Role> for RoleDetailDto {
    fn from(role: &Role) -> Self {
        Self {
            id:          *role.id().as_uuid(),
            name:        role.name().to_string(),
            description: role.description().map(|s| s.to_string()),
            permissions: role.permissions().iter().map(|p| p.to_string()).collect(),
            is_system:   role.is_system(),
            created_at:  role.created_at().to_rfc3339(),
            updated_at:  role.updated_at().to_rfc3339(),
        }
    }
}
```

```rust
// Before (role.rs L143-151)
let response = ApiResponse::new(RoleDetailDto {
    id:          *role.id().as_uuid(),
    name:        role.name().to_string(),
    // ... 5 more fields
});

// After
let response = ApiResponse::new(RoleDetailDto::from(&role));
```

### 3b: Core auth.rs に `build_user_with_permissions` ヘルパー

get_user (L354-366) と get_user_by_display_number (L459-470) の権限集約+DTO構築を統一する。

### 変更ファイル
- `backend/apps/core-service/src/handler/auth.rs` — ヘルパー関数追加、2箇所を置換

```rust
/// User と Role リストから UserWithPermissionsData を構築する
fn build_user_with_permissions(
    user: &User,
    roles: &[Role],
    tenant_name: String,
) -> UserWithPermissionsData {
    let permissions: Vec<String> = roles
        .iter()
        .flat_map(|r| r.permissions().iter().map(|p| p.to_string()))
        .collect();

    UserWithPermissionsData {
        user: UserResponse::from(user),
        tenant_name,
        roles: roles.iter().map(|r| r.name().to_string()).collect(),
        permissions,
    }
}
```

### 確認事項
- [x] 型: `Role` のメソッド群 → role.rs L59-67 の RoleDetailDto フィールドと一致（id, name, description, permissions, is_system, created_at, updated_at）
- [x] パターン: `From<WorkflowDefinition> for WorkflowDefinitionDto` → workflow.rs L160-174 で確認済み
- [x] 型: `UserWithPermissionsData` → auth.rs L76-81: user, tenant_name, roles, permissions
- [x] 型: `UserResponse::from(&User)` → auth.rs L62-72 で確認済み

### テストリスト

ユニットテスト:
- [x] `build_user_with_permissions` 正常系: ロールありで権限が正しく集約される
- [x] `build_user_with_permissions` 正常系: ロール空で空の権限・ロールリスト

ハンドラテスト（該当なし — 既存テストのパスで確認 ✅）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 4: Core auth.rs サブモジュール分割

Core auth.rs（965行 = ハンドラ ~556行 + テスト ~409行）をディレクトリ化し、テストモジュールを分離する。

### 変更ファイル
- `backend/apps/core-service/src/handler/auth.rs` → `backend/apps/core-service/src/handler/auth/mod.rs` に移動
- `backend/apps/core-service/src/handler/auth/tests.rs` — テストモジュールを分離

### 分割構造

```
handler/auth/
├── mod.rs      # 型定義 + ハンドラ + ヘルパー (~556行)
└── tests.rs    # テストコード (~409行)
```

mod.rs の末尾:
```rust
#[cfg(test)]
mod tests;
```

tests.rs の先頭:
```rust
use super::*;
// 既存のテストコードをそのまま移動
```

### 確認事項
- [x] パターン: `#[cfg(test)] mod tests;` + `tests.rs` で `super::*` — Rust の標準パターン確認済み
- [x] handler.rs の re-export: `pub mod auth;` (handler.rs L15) — ディレクトリ化しても変更不要

### テストリスト

ユニットテスト（該当なし — 既存テストの移動のみ、ロジック変更なし）

ハンドラテスト（該当なし — 既存テストのパスで確認）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 5: BFF auth.rs サブモジュール分割

BFF auth.rs（1075行）を責務別にサブモジュール化し、ファイルサイズを削減する。

### 変更ファイル
- `backend/apps/bff/src/handler/auth.rs` → `backend/apps/bff/src/handler/auth/mod.rs` に移動
- `backend/apps/bff/src/handler/auth/login.rs` — login + logout ハンドラ + テスト
- `backend/apps/bff/src/handler/auth/session.rs` — me + csrf ハンドラ + テスト

### 分割構造

```
handler/auth/
├── mod.rs       # 共有型 + State + Cookie ヘルパー + テストスタブ (~200行)
├── login.rs     # login + logout + テスト (~450行)
└── session.rs   # me + csrf + テスト (~400行)
```

### mod.rs の内容

```rust
mod login;
mod session;

pub use login::{login, logout};
pub use session::{csrf, me};

// 共有型: AuthState, LoginRequest, LoginResponseData, LoginUserResponse,
//         MeResponseData, CsrfResponseData
// 共有定数: SESSION_COOKIE_NAME, SESSION_MAX_AGE
// Cookie ヘルパー: build_session_cookie, build_clear_cookie
// テストスタブ: #[cfg(test)] で StubCoreServiceClient, StubAuthServiceClient, StubSessionManager
```

### login.rs の内容

```rust
use super::*;  // mod.rs の型・定数をインポート

pub async fn login(...) -> impl IntoResponse { ... }
pub async fn logout(...) -> impl IntoResponse { ... }

#[cfg(test)]
mod tests { ... }  // login, logout のテスト
```

### session.rs の内容

```rust
use super::*;

pub async fn me(...) -> impl IntoResponse { ... }
pub async fn csrf(...) -> impl IntoResponse { ... }

#[cfg(test)]
mod tests { ... }  // me, csrf のテスト
```

### テストスタブ共有の設計

テストスタブ（StubCoreServiceClient, StubAuthServiceClient, StubSessionManager）は login.rs と session.rs の両方で使用される。mod.rs の `#[cfg(test)]` ブロックで定義し、`pub(super)` で子モジュールから参照可能にする。

```rust
// mod.rs
#[cfg(test)]
pub(super) mod test_utils {
    // StubCoreServiceClient, StubAuthServiceClient, StubSessionManager
}
```

### 確認事項
- [ ] パターン: workflow/ ディレクトリ化の前例 → `handler/workflow.rs` (mod.rs 相当) + command.rs + query.rs
- [ ] 型: `AuthState` が全ハンドラで共有 → mod.rs に残す
- [ ] パターン: テストスタブの共有方法 → `#[cfg(test)] pub(super) mod test_utils` で子モジュールに公開

### テストリスト

ユニットテスト（該当なし — 既存テストの移動のみ）

ハンドラテスト（該当なし — 既存テストのパスで確認）

API テスト（該当なし）

E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `parse_display_number` のエラーメッセージにフィールド名（display_number / step_display_number）の違いがある | 競合・エッジケース | `field: &str` パラメータを追加してメッセージを保持する設計に変更 |
| 2回目 | Phase 2 の resolve メソッドが list_my_workflows / list_comments / post_comment には適用不可（異なるパターン） | 不完全なパス | 対象外ケースを明示。list_my_workflows は複数インスタンスの HashSet パターン、list_comments は WorkflowCommentDto パターン |
| 3回目 | task.rs (get_task / get_task_by_display_numbers) でも `resolve_from_workflow_with_steps` が使えるが、WorkflowWithSteps を手動構築している | 既存手段の見落とし | task.rs への適用は Phase 2 の追加改善として記載。`pub(crate)` 可視性で task.rs からもアクセス可能 |
| 4回目 | BFF auth.rs の tenant ID 抽出クローン（4箇所）は `crate::error::extract_tenant_id` として既に関数化済み。match パターンの重複は残るが、BFF handler のクローンは Story 3 (#526) のスコープ | 既存手段の見落とし | BFF auth.rs のスコープをファイルサイズ削減（サブモジュール分割）に限定 |
| 5回目 | Core auth.rs が965行だが非テストコードは ~556行。テスト分離で対応可能 | シンプルさ | Phase 4 をテストモジュール分離のみに簡素化（ハンドラの機能分割は不要） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Issue の5グループ全て（workflow 17, auth+role 12, role 3, task 2, workflow 4）を Phase 1-5 でカバー。Story 7 スコープの types 重複は対象外として明示 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全 Phase で具体的なコードスニペット、ファイルパス、削減箇所数を記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 5つの設計判断に選択肢・理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象外: Story 7 の types 重複、BFF handler クローン（Story 3 #526） |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Rust のモジュール可視性（private = 子モジュールからアクセス可能）、`#[cfg(test)] mod tests;` パターン |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | workflow/ の CQRS 分割パターン、`From` trait の前例（WorkflowDefinitionDto）と整合 |

## 検証方法

```bash
# 各 Phase 完了後に実行
just check       # lint + test（軽量チェック）

# 全 Phase 完了後に実行
just check-all   # lint + test + API test + E2E test
```

既存テストがすべてパスすること = リファクタリングの正当性検証。新規ヘルパー関数のユニットテストで変換ロジックの正確性を検証。
