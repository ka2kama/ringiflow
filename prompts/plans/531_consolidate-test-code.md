# テストコード共通化 計画（Issue #531）

## Context

Epic #467 のサブ Issue。テストコード間の重複（26クローン）を共通化して保守性を向上させる。テストの振る舞い（テスト関数の数・検証内容）は変えない。純粋なリファクタリング。

## 対象

| ファイル | 行数 | クローン数 |
|---------|------|-----------|
| `backend/apps/bff/tests/auth_integration_test.rs` | 934 | 10 |
| `backend/crates/infra/tests/user_repository_test.rs` | 698 | 6 |
| `backend/crates/infra/tests/rls_test.rs` | 614 | 3 |
| `backend/crates/infra/tests/workflow_step_repository_test.rs` | 350 | 3 |
| `backend/crates/infra/tests/postgres_deleter_test.rs` | 395 | 2 |
| `backend/crates/infra/tests/display_id_counter_repository_test.rs` | 123 | 1 |
| `backend/crates/infra/tests/db_test.rs` | 152 | 1 |

対象外:
- `session_test.rs` の redis_url() 重複（後述の判断1で除外）
- `display_id_counter_repository_test.rs` の1クローン（後述の判断6で除外）
- `db_test.rs` の1クローン（後述の判断6で除外）

## 設計判断

### 判断1: redis_url() の cross-crate 重複 → 共通化しない

`auth_integration_test.rs`（bff）と `session_test.rs`（infra）で同一の6行関数。cross-crate 共有は `shared` クレートへの `test-utils` feature 追加が必要で、6行の関数に対してビルド構成の複雑さが見合わない。「3回繰り返すまでは重複を許容」の原則にも合致。

### 判断2: auth_integration_test.rs のログインパターン → ヘルパー関数

8テストで繰り返される「ログイン → Set-Cookie → セッションID抽出」を `login_and_get_session()` に集約。ログイン自体を検証するテスト（`test_ログインからログアウトまでの一連フロー`、`test_csrfトークン_ログイン成功時に生成される`）には適用しない。

### 判断3: auth_integration_test.rs のクリーンアップ → ヘルパー関数

7テストで繰り返される「セッション + CSRF トークンの Redis クリーンアップ」を `cleanup_auth_session()` に集約。

### 判断4: 別テナント作成パターン → common/mod.rs に追加

`user_repository_test.rs` の3箇所と `postgres_deleter_test.rs` で繰り返される「テナント INSERT」を `create_other_tenant()` として共通化。

### 判断5: 追加ユーザー INSERT パターン → common/mod.rs に追加

`user_repository_test.rs` の4箇所で繰り返される手書き SQL INSERT を `insert_user_raw()` として共通化。パラメータ（email, name, status, display_number）を引数で受ける。

### 判断6: display_id_counter / db_test の1クローンずつ → 対応しない

各1クローンの共通化はヘルパー導入のオーバーヘッドが削減効果を上回る。KISS 原則に合致。

## Phase 構成

### Phase 1: common/mod.rs の拡充

`backend/crates/infra/tests/common/mod.rs` にヘルパーを追加。

#### 確認事項
- [x] 型: `TenantId::from_uuid(Uuid::now_v7())` のパターン → `common/mod.rs` L39-41、確認済み
- [x] パターン: `setup_test_data()` の SQL → `common/mod.rs` L112-142、確認済み
- [x] パターン: user_repository_test.rs の手書き INSERT → L79-88, L198-209, L274-285, L304-315、確認済み

#### 変更内容

**`create_other_tenant()`**: 別テナント作成ヘルパー

```rust
pub async fn create_other_tenant(pool: &PgPool) -> TenantId {
    let tenant_id = TenantId::from_uuid(Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO tenants (id, name, subdomain, plan, status)
        VALUES ($1, 'Other Tenant', 'other', 'free', 'active')
        "#,
        tenant_id.as_uuid()
    )
    .execute(pool)
    .await
    .expect("別テナント作成に失敗");
    tenant_id
}
```

**`insert_user_raw()`**: SQL 直接挿入ヘルパー（リポジトリを経由しない）

```rust
pub async fn insert_user_raw(
    pool: &PgPool,
    tenant_id: &TenantId,
    display_number: i64,
    email: &str,
    name: &str,
    status: &str,
) -> UserId {
    let user_id = UserId::from_uuid(Uuid::now_v7());
    sqlx::query!(
        r#"
        INSERT INTO users (id, tenant_id, display_number, email, name, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
        user_id.as_uuid(),
        tenant_id.as_uuid(),
        display_number,
        email,
        name,
        status,
    )
    .execute(pool)
    .await
    .expect("ユーザー挿入に失敗");
    user_id
}
```

#### テストリスト

ユニットテスト（該当なし — ヘルパー追加のみ）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `just check-all` で既存テストの回帰がないこと。

---

### Phase 2: auth_integration_test.rs の共通化（10クローン）

#### 確認事項
- [x] 型: `AuthState` の構造 → `auth_integration_test.rs` create_test_app の戻り値、`Arc<AuthState>` 確認済み
- [x] パターン: `SessionManager` の API → `get_csrf_token`, `delete`, `delete_csrf_token`、確認済み
- [x] パターン: `extract_session_id` の戻り値 → `Option<String>`、確認済み

#### 変更内容

**`login_and_get_session()`**: ログイン + セッションID取得ヘルパー

```rust
async fn login_and_get_session(sut: &Router, state: &AuthState) -> String {
    let login_response = sut
        .clone()
        .oneshot(login_request("user@example.com", "password123"))
        .await
        .unwrap();
    assert_eq!(login_response.status(), StatusCode::OK);
    let set_cookie = login_response
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap();
    extract_session_id(set_cookie).expect("セッション ID が設定されていない")
}
```

注: session_id のみ返す。CSRF トークンが必要なテストは `get_csrf_token_via_api()` または `state.session_manager.get_csrf_token()` で別途取得。

**`get_csrf_token_via_api()`**: GET /api/v1/auth/csrf 経由でトークン取得

```rust
async fn get_csrf_token_via_api(sut: &Router, session_id: &str) -> String {
    let csrf_response = sut
        .clone()
        .oneshot(csrf_request(session_id))
        .await
        .unwrap();
    assert_eq!(csrf_response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(csrf_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["data"]["token"].as_str().unwrap().to_string()
}
```

**`cleanup_auth_session()`**: セッション + CSRF トークンの Redis クリーンアップ

```rust
async fn cleanup_auth_session(state: &AuthState, session_id: &str) {
    let tenant_id = ringiflow_domain::tenant::TenantId::from_uuid(test_tenant_id());
    let _ = state.session_manager.delete(&tenant_id, session_id).await;
    let _ = state
        .session_manager
        .delete_csrf_token(&tenant_id, session_id)
        .await;
}
```

**適用先**:

| テスト関数 | login_and_get_session | get_csrf_token_via_api | cleanup_auth_session |
|-----------|:---:|:---:|:---:|
| test_ログインからログアウトまでの一連フロー | - | - | - |
| test_ログアウト後にauthmeで401 | ✓ | - | ✓ |
| test_csrfトークン_ログイン成功時に生成される | - | - | ✓ |
| test_csrfトークン_get_auth_csrfで取得できる | ✓ | - | ✓ |
| test_csrfトークン_正しいトークンでpostリクエストが成功する | ✓ | ✓ | ✓ |
| test_csrfトークン_トークンなしでpostリクエストが403になる | ✓ | - | ✓ |
| test_csrfトークン_不正なトークンでpostリクエストが403になる | ✓ | - | ✓ |
| test_csrfトークン_ログアウト時に削除される | ✓ | ✓ | - |

- `test_ログインからログアウトまでの一連フロー`: ログイン成功の検証がテスト目的の一部。ヘルパー不使用。
- `test_csrfトークン_ログイン成功時に生成される`: ログイン後のCSRFトークン存在確認がテスト目的。ログイン部分は直接記述。クリーンアップのみヘルパー使用。
- `test_csrfトークン_ログアウト時に削除される`: ログアウトでCSRFが削除されることの検証がテスト目的。クリーンアップ不要。

#### テストリスト

ユニットテスト（該当なし — リファクタリングのため新規テスト不要）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `cargo test -p ringiflow-bff --test auth_integration_test` で全12テストがパスすること。

---

### Phase 3: user_repository_test.rs の共通化（6クローン）

#### 確認事項
- [x] パターン: `create_other_tenant()` → Phase 1 で common/mod.rs に追加済み、確認済み
- [x] パターン: `insert_user_raw()` → Phase 1 で common/mod.rs に追加済み、確認済み
- [x] パターン: 手書き INSERT のパラメータ差異 → 各テストの email, name, status, display_number を確認済み

#### 変更内容

**別テナント作成の共通化（3箇所）**:
- L79-88 (`test_別テナントのユーザーは取得できない`) → `create_other_tenant(&pool)`
- L141-151 (`test_find_with_roles_別テナントのロール割り当ては含まれない`) → `create_other_tenant(&pool)`
- L333-343 (`test_他テナントのユーザーは含まれない`) → `create_other_tenant(&pool)`

**追加ユーザー INSERT の共通化（4箇所）**:
- L198-209 (`test_複数idでユーザーを一括取得できる`) → `insert_user_raw(&pool, &tenant_id, 2, "user2@example.com", "User Two", "active")`
- L274-285 (`test_テナント内のアクティブユーザー一覧を取得できる`) → 同上
- L304-315 (`test_非アクティブユーザーは除外される`) → `insert_user_raw(&pool, &tenant_id, 2, "inactive@example.com", "Inactive User", "inactive")`
- L346-357 (`test_他テナントのユーザーは含まれない`) → `insert_user_raw(&pool, &other_tenant_id, 1, "other@example.com", "Other User", "active")`

同様パターンの追加適用（Issue クローン対象外だが同一パターン）:
- L438-449 (`test_find_all_by_tenantでステータスフィルタが機能する`) → `insert_user_raw`
- L479-489 (`test_find_all_by_tenantでdeletedユーザーは除外される`) → `insert_user_raw`
- L651-662 (`test_find_roles_for_usersで複数ユーザーのロールを一括取得できる`) → `insert_user_raw`

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `cargo test -p ringiflow-infra --test user_repository_test` で全テストがパスすること。

---

### Phase 4: workflow_step_repository_test.rs の共通化（3クローン）

#### 確認事項
- [x] パターン: リポジトリ初期化 + インスタンス INSERT → L29-35, L46-51 で繰り返し確認済み
- [x] 型: `PostgresWorkflowInstanceRepository::new()`, `PostgresWorkflowStepRepository::new()` のシグネチャ、`PgPool` 引数確認済み

#### 変更内容

**`setup_repos_with_instance()`**: リポジトリ初期化 + インスタンス INSERT ヘルパー

```rust
struct StepTestContext {
    sut: PostgresWorkflowStepRepository,
    instance: WorkflowInstance,
    tenant_id: TenantId,
}

async fn setup_repos_with_instance(pool: PgPool, display_number: i64) -> StepTestContext {
    let instance_repo = PostgresWorkflowInstanceRepository::new(pool.clone());
    let sut = PostgresWorkflowStepRepository::new(pool);
    let tenant_id = seed_tenant_id();
    let instance = create_test_instance(display_number);
    instance_repo.insert(&instance).await.unwrap();
    StepTestContext { sut, instance, tenant_id }
}
```

適用先: `test_insert_`, `test_find_by_id_`, `test_find_by_instance_`, `test_find_by_assigned_to_`, `test_update_with_version_check_*`, `test_find_by_display_number_*`, `test_ステップを完了して*` の各テスト（テスト目的に応じて使い分け）。

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `cargo test -p ringiflow-infra --test workflow_step_repository_test` で全テストがパスすること。

---

### Phase 5: rls_test.rs の共通化（3クローン）

#### 確認事項
- [x] パターン: `set_tenant_context` + `reset_role` のライフサイクル → `rls_test.rs` L230-253、確認済み
- [x] パターン: `SELECT id FROM {table}` の結果取得 → `Vec<(Uuid,)>` 確認済み、クエリ文字列引数に変更

#### 変更内容

**`query_ids_as_tenant()`**: テナントコンテキストで `SELECT id FROM {table}` を実行するヘルパー

```rust
async fn query_ids_as_tenant(pool: &PgPool, tenant_id: &Uuid, table: &str) -> Vec<Uuid> {
    let mut conn = pool.acquire().await.unwrap();
    set_tenant_context(&mut conn, tenant_id).await;
    let query = format!("SELECT id FROM {table}");
    let rows: Vec<(Uuid,)> = sqlx::query_as(&query)
        .fetch_all(&mut *conn)
        .await
        .unwrap();
    reset_role(&mut conn).await;
    rows.into_iter().map(|r| r.0).collect()
}
```

適用先: `tenants`, `users`, `user_roles`, `workflow_definitions`, `workflow_instances`, `workflow_steps`, `display_id_counters` の各テスト（`SELECT id FROM` パターン）。`roles` テストは3カラム返却のため適用外。`auth.credentials` テストも同様。

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `cargo test -p ringiflow-infra --test rls_test` で全テストがパスすること。

---

### Phase 6: postgres_deleter_test.rs の共通化（2クローン）

#### 確認事項
- [x] 型: `TenantDeleter` trait → `deletion/mod.rs` L45-54, `count() -> Result<u64>`, `delete() -> Result<DeletionResult>`、確認済み
- [x] パターン: `setup_two_tenants()` → L26-51、Phase 1 ヘルパーで簡潔化確認済み

#### 変更内容

**`setup_two_tenants()` の簡潔化**: Phase 1 の `create_other_tenant()` + `insert_user_raw()` で置換

```rust
async fn setup_two_tenants(pool: &PgPool) -> (TenantId, TenantId) {
    let (tenant_a, _) = setup_test_data(pool).await;
    let tenant_b = create_other_tenant(pool).await;
    insert_user_raw(pool, &tenant_b, 1, "b@example.com", "User B", "active").await;
    (tenant_a, tenant_b)
}
```

**`assert_count_delete_count()`**: count → delete → count=0 パターンのヘルパー

```rust
async fn assert_count_delete_count<T: TenantDeleter>(
    sut: &T,
    tenant_id: &TenantId,
    expected_count: u64,
    expected_deleted: u64,
) {
    let count = sut.count(tenant_id).await.unwrap();
    assert_eq!(count, expected_count);
    let result = sut.delete(tenant_id).await.unwrap();
    assert_eq!(result.deleted_count, expected_deleted);
    let count_after = sut.count(tenant_id).await.unwrap();
    assert_eq!(count_after, 0);
}
```

適用先: `test_role_deleter_*`, `test_display_id_counter_deleter_*`, `test_auth_credentials_deleter_*` の count/delete テスト。`test_workflow_deleter_*` は delete が steps + instances + definitions の合計を返す特殊ケースだが、expected_deleted=3 で対応可能。

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

検証: `cargo test -p ringiflow-infra --test postgres_deleter_test` で全テストがパスすること。

---

## 対応しないクローン（3件）

| クローン | ファイル | 理由 |
|---------|---------|------|
| redis_url() cross-crate | auth_integration_test ↔ session_test | 6行×2箇所。cross-crate 共有のコストが見合わない |
| insert_counter パターン | display_id_counter_repository_test | 1クローン。テスト固有のセットアップ |
| create_test_pool パターン | db_test | 1クローン。テスト固有のセットアップ |

対応クローン数: 26 - 3 = **23クローン**

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `login_and_get_session` がすべてのテストに適用できるわけではない | 不完全なパス | ログイン検証がテスト目的のテスト（2件）を除外し、適用テーブルを明記 |
| 2回目 | TenantDeleter の async fn in trait と object safety | 技術的前提 | `&dyn TenantDeleter` ではなくジェネリクス `<T: TenantDeleter>` を使用 |
| 3回目 | `TenantDeleter::count()` の戻り値が `u64` なのにPlan agentが `i64` と記載 | 事実的妥当性 | `deletion/mod.rs` L53 を確認し `u64` に修正 |
| 4回目 | Phase 4 の `setup_instance_with_step` は insert テスト自体がステップ INSERT を検証するため使い分けが必要 | 不完全なパス | `setup_repos_with_instance` でインスタンスのみ INSERT し、ステップ INSERT はテスト側に委ねる設計に変更 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 26クローン全てに対応方針が決まっている | OK | 23クローンを共通化、3クローンは理由付きで除外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase のヘルパー関数シグネチャと適用テーブルを明記 |
| 3 | 設計判断の完結性 | 全クローンに判断が記載されている | OK | 6つの設計判断で全パターンをカバー |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | 対象7ファイル26クローン、対象外3件を理由付きで明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Rust integration test の独立クレート制約、`async fn in trait` の object safety、`TenantDeleter::count()` の戻り値型 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | CLAUDE.md「3回繰り返すまでは重複を許容」原則に合致 |

## 検証方法

1. 各 Phase で対象テストファイルの `cargo test` を実行し全テストがパスすることを確認
2. 全 Phase 完了後に `just check-all` で全体の回帰テストを実施
3. jscpd でテストファイルのクローン数が削減されていることを確認
