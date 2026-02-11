# Story #427: 認可ミドルウェア（RBAC Permission チェック基盤）

## Context

Phase 2-2（ユーザー管理 + 監査ログ）の最初の Story。
現状、BFF には認証（ログイン済みかどうか）はあるが、認可（権限チェック）がない。
管理者向けエンドポイント（ユーザー CRUD、ロール管理）を保護するため、RBAC ベースの認可ミドルウェアを構築する。

### 現状（As-Is）

- `SessionData` には `roles: Vec<String>` があるが `permissions` フィールドがない
- ログイン時に Core Service から `permissions` を取得しているが、セッションに保存していない
- `Permission` 型には `new()` と `as_str()` のみ。ワイルドカードマッチングロジックがない
- CSRF ミドルウェアは `from_fn_with_state` パターンで実装済み（参考になる）
- `forbidden_response()` ヘルパーは既に存在する

### 理想（To-Be）

- `Permission::satisfies()` でワイルドカードマッチングが可能
- セッションにパーミッション情報が含まれる
- axum ミドルウェアでルート単位の認可チェックが可能
- 管理者エンドポイントに認可が適用される

## スコープ

### 対象

- `Permission::satisfies()` ワイルドカードマッチング
- `SessionData` への `permissions` フィールド追加
- ログイン時のパーミッション保存
- 認可ミドルウェア（`require_permission`）
- 管理者ルートへの適用

### 対象外

- 既存ハンドラのリファクタリング（セッション取得パターンの共通化など）
- フロントエンドでの権限チェック
- ユーザー CRUD API（Story #428）

## Phase 1: Permission ワイルドカードマッチング

ドメイン層に `Permission::satisfies()` メソッドを追加する。

### 確認事項

- 型: `Permission` の定義 → `backend/crates/domain/src/role.rs:88`（確認済み）
- パターン: 既存テストパターン → `backend/crates/domain/src/role.rs:297`（rstest + fixture）

### 設計

```rust
impl Permission {
    /// この権限が、要求された権限を満たすか判定する
    ///
    /// ## マッチングルール
    ///
    /// | 保持権限 | 要求権限 | 結果 |
    /// |---------|---------|------|
    /// | `*` | 任意 | true（全権限） |
    /// | `user:*` | `user:read` | true（リソース内の全アクション） |
    /// | `user:read` | `user:read` | true（完全一致） |
    /// | `user:read` | `user:write` | false |
    /// | `user:*` | `task:read` | false（リソース不一致） |
    pub fn satisfies(&self, required: &Permission) -> bool {
        let held = self.as_str();
        let req = required.as_str();

        if held == "*" {
            return true;
        }

        if let Some(resource) = held.strip_suffix(":*") {
            return req.starts_with(&format!("{resource}:"));
        }

        held == req
    }
}
```

メソッド名は `satisfies` を採用。`matches` は `str::matches` と紛らわしいため。

### テストリスト

- [ ] `*` は任意の権限を満たす
- [ ] `resource:*` は同一リソースの任意のアクションを満たす
- [ ] `resource:action` は完全一致のみ満たす
- [ ] `resource:*` は異なるリソースを満たさない
- [ ] `resource:action` は異なるアクションを満たさない
- [ ] `resource:action` は `*` を満たさない（一般ユーザーが全権限を満たさない）

### 変更ファイル

- `backend/crates/domain/src/role.rs` — `satisfies()` メソッドとテスト追加

## Phase 2: SessionData への permissions 追加

### 確認事項

- 型: `SessionData` の定義 → `backend/crates/infra/src/session.rs:39`（確認済み）
- パターン: ログインフロー → `backend/apps/bff/src/handler/auth.rs:184`（確認済み）
- パターン: DevAuth → `backend/apps/bff/src/dev_auth.rs:66`（確認済み）

### 設計

`SessionData` に `permissions: Vec<String>` を追加する。

```rust
pub struct SessionData {
    user_id: UserId,
    tenant_id: TenantId,
    email: String,
    name: String,
    roles: Vec<String>,
    permissions: Vec<String>,  // 追加
    created_at: DateTime<Utc>,
    last_accessed_at: DateTime<Utc>,
}
```

`SessionData::new()` のシグネチャに `permissions: Vec<String>` を追加:

```rust
pub fn new(
    user_id: UserId,
    tenant_id: TenantId,
    email: String,
    name: String,
    roles: Vec<String>,
    permissions: Vec<String>,  // 追加
) -> Self
```

getter を追加:

```rust
pub fn permissions(&self) -> &[String] {
    &self.permissions
}
```

呼び出し元の更新:

1. **ログインハンドラ** (`auth.rs:184`): `user_with_roles.data.permissions.clone()` を追加
2. **DevAuth** (`dev_auth.rs:66`): `tenant_admin` 相当の権限を追加
   ```rust
   pub const DEV_USER_PERMISSIONS: &[&str] = &["tenant:*", "user:*", "workflow:*", "task:*"];
   ```
3. **テストスタブ** (`auth.rs` テスト内 `StubSessionManager`): permissions パラメータ追加

### 後方互換性

`SessionData` は Redis に JSON 形式で保存される。既存セッション（`permissions` フィールドなし）からのデシリアライズを考慮し、`#[serde(default)]` を `permissions` フィールドに付与する:

```rust
#[serde(default)]
permissions: Vec<String>,
```

これにより、既存セッションは `permissions: []` として読み込まれる（ログインし直すと正しい権限が保存される）。

### テストリスト

- [ ] `SessionData::new()` で permissions が保存される
- [ ] `SessionData::permissions()` で保存した permissions が取得できる
- [ ] permissions フィールドなしの JSON からデシリアライズすると空 Vec になる（後方互換性）

### 変更ファイル

- `backend/crates/infra/src/session.rs` — permissions フィールド、getter、serde(default) 追加
- `backend/apps/bff/src/handler/auth.rs` — ログイン時に permissions を保存、テストスタブ更新
- `backend/apps/bff/src/dev_auth.rs` — DEV_USER_PERMISSIONS 追加

## Phase 3: 認可ミドルウェア

### 確認事項

- パターン: CSRF ミドルウェア → `backend/apps/bff/src/middleware/csrf.rs`（確認済み、`from_fn_with_state` パターン）
- 型: `forbidden_response()` → `backend/apps/bff/src/error.rs:147`（確認済み）
- 型: `get_session()` → `backend/apps/bff/src/error.rs:59`（確認済み）
- ライブラリ: `from_fn_with_state` → `main.rs:60`、`csrf.rs:67` で使用確認済み

### 設計

新ファイル `backend/apps/bff/src/middleware/authz.rs` を作成する。

```rust
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::CookieJar;
use ringiflow_domain::role::Permission;
use ringiflow_infra::SessionManager;

use crate::error::{forbidden_response, get_session, extract_tenant_id};

/// 認可ミドルウェアの状態
#[derive(Clone)]
pub struct AuthzState {
    pub session_manager: Arc<dyn SessionManager>,
    pub required_permission: String,
}

/// 認可ミドルウェア
///
/// セッションから権限を取得し、要求された権限を満たすか検証する。
pub async fn require_permission(
    State(state): State<AuthzState>,
    jar: CookieJar,
    request: Request<Body>,
    next: Next,
) -> Response {
    // テナント ID を取得
    let tenant_id = match extract_tenant_id(request.headers()) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };

    // セッションを取得
    let session = match get_session(
        state.session_manager.as_ref(),
        &jar,
        tenant_id,
    ).await {
        Ok(s) => s,
        Err(response) => return response,
    };

    // 権限チェック
    let required = Permission::new(&state.required_permission);
    let has_permission = session
        .permissions()
        .iter()
        .any(|p| Permission::new(p).satisfies(&required));

    if !has_permission {
        return forbidden_response("この操作を実行する権限がありません");
    }

    next.run(request).await
}
```

`middleware.rs` に authz モジュールを追加:

```rust
mod authz;
mod csrf;

pub use authz::{AuthzState, require_permission};
pub use csrf::{CsrfState, csrf_middleware};
```

### 設計判断

**State パターン（`AuthzState`）を採用する理由**:

- CSRF ミドルウェアと同じパターンで一貫性がある
- `required_permission` をルートグループ毎に変えられる
- axum の `from_fn_with_state` で自然に使える

**代替案**: axum の Extension を使う方法もあるが、State パターンの方が型安全で既存パターンと一貫する。

**`get_session` の再利用**: `error.rs` に既存の `get_session()` ヘルパーがあり、セッション取得ロジックを重複させない。

### テストリスト

- [ ] 権限を持つユーザーは 200 OK でリクエストが通過する
- [ ] 権限を持たないユーザーは 403 Forbidden を返す
- [ ] ワイルドカード権限（`*`）は任意のリクエストを通過させる
- [ ] セッションなし（未認証）は 401 Unauthorized を返す

### 変更ファイル

- `backend/apps/bff/src/middleware/authz.rs` — 新規作成
- `backend/apps/bff/src/middleware.rs` — authz モジュール追加

## Phase 4: ルーター統合

### 確認事項

- パターン: ルーター構造 → `backend/apps/bff/src/main.rs:181`（確認済み）
- パターン: CSRF ミドルウェア適用 → `main.rs:226`

### 設計

管理者向けルート（Story #428, #429 で追加予定）に認可ミドルウェアを適用する準備として、`main.rs` にルートグループを作成する。

現時点で存在する管理者向けルートは `/api/v1/users`（`list_users`）のみ。このルートに `user:read` 権限を適用する。

```rust
// main.rs の変更部分

use middleware::{AuthzState, CsrfState, csrf_middleware, require_permission};

// ...

// ユーザー管理 API（認可あり）
let user_admin_authz = AuthzState {
    session_manager: session_manager.clone(),
    required_permission: "user:read".to_string(),
};

let app = Router::new()
    // ... 既存ルート ...
    // ユーザー API（認可ミドルウェア適用）
    .route("/api/v1/users", get(list_users))
    .layer(from_fn_with_state(user_admin_authz, require_permission))
    .with_state(workflow_state)
    // ...
```

注: axum のルーター構造上、`.layer()` はその直前に定義されたルートに適用される。ルートグループの分離は axum の `Router::new().merge()` または `.nest()` で実現可能。具体的な構造は実装時にコードを確認して決定する。

### テストリスト

- [ ] `just check-all` が通過する（統合として）

### 変更ファイル

- `backend/apps/bff/src/main.rs` — 認可ミドルウェアの適用

## 検証方法

1. `just check` — コンパイル + ユニットテスト
2. `just check-all` — リント + テスト + API テスト
3. ユニットテスト: 各 Phase のテストリストが全て Green

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | 既存セッションとの後方互換性が未考慮 | 不完全なパス | `#[serde(default)]` を追加して既存セッション（permissions フィールドなし）のデシリアライズを保証 |
| 2回目 | `get_session()` ヘルパーの再利用が未検討 | 既存手段の見落とし | `error.rs` の `get_session()` を認可ミドルウェアで再利用する設計に変更 |
| 3回目 | DevAuth の permissions 未設定 | 不完全なパス | `DEV_USER_PERMISSIONS` 定数を追加し、`setup_dev_session()` に反映 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Permission, SessionData, ミドルウェア, ルーター統合, DevAuth の5箇所すべてカバー |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase で具体的なコードスニペットと関数シグネチャを記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | メソッド名（satisfies）、State パターン、serde(default) の理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「対象」「対象外」セクションで明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Redis JSON 後方互換性、axum layer 適用範囲を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 機能仕様書 AUTHZ-001、権限形式（`resource:action`）と整合 |
