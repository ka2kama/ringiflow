# Phase 4: Core Service 認証エンドポイント

## 概要

Core Service の内部認証エンドポイントを実装した。
`AuthUseCase` でパスワード検証・ユーザーステータス確認のロジックを提供し、
HTTP ハンドラで BFF からのリクエストを処理する。

### 対応 Issue

[#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34) - Phase 4

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [実装コンポーネント > 実装順序](../../03_詳細設計書/07_認証機能設計.md#実装順序) | Phase 4 の位置づけ |
| [Core Service 内部 API](../../03_詳細設計書/07_認証機能設計.md#core-service-内部-api) | API 仕様 |
| [パスワード検証フロー](../../03_詳細設計書/07_認証機能設計.md#パスワード検証フロー) | タイミング攻撃対策 |
| [テスト計画](../../03_詳細設計書/07_認証機能設計.md#テスト計画) | テストケース |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/apps/core-service/src/usecase/auth.rs`](../../../backend/apps/core-service/src/usecase/auth.rs) | AuthUseCase, AuthError |
| [`backend/apps/core-service/src/usecase/mod.rs`](../../../backend/apps/core-service/src/usecase/mod.rs) | ユースケース層モジュール |
| [`backend/apps/core-service/src/handler/auth.rs`](../../../backend/apps/core-service/src/handler/auth.rs) | 認証ハンドラ, リクエスト/レスポンス型 |

---

## 実装内容

### AuthUseCase（[`auth.rs:39-120`](../../../backend/apps/core-service/src/usecase/auth.rs#L39-L120)）

認証ロジックを提供するユースケース。

```rust
pub struct AuthUseCase<R, P>
where
    R: UserRepository,
    P: PasswordChecker,
{
    user_repository: R,
    password_checker: P,
}
```

**主要メソッド:**

```rust
// 認証情報を検証し、ユーザーとロールを返す
usecase.verify_credentials(&tenant_id, &email, &password).await
    // -> Ok((User, Vec<Role>))
    // -> Err(AuthError::AuthenticationFailed)
```

### AuthError（[`auth.rs:22-34`](../../../backend/apps/core-service/src/usecase/auth.rs#L22-L34)）

```rust
pub enum AuthError {
    /// 認証失敗（メール不存在、パスワード不一致、非アクティブ）
    AuthenticationFailed,
    /// インフラ層エラー
    Internal(#[from] InfraError),
}
```

セキュリティ上、認証失敗の詳細な理由は外部に公開しない。

### HTTP ハンドラ

**POST /internal/auth/verify:**

認証情報を検証し、ユーザー情報を返す。

```rust
// リクエスト
{
    "tenant_id": "uuid",
    "email": "user@example.com",
    "password": "password123"
}

// レスポンス（200 OK）
{
    "user": { "id": "...", "email": "...", "name": "...", "status": "active" },
    "roles": [{ "id": "...", "name": "user", "permissions": [...] }]
}
```

**GET /internal/users/{user_id}:**

ユーザー情報をロール・権限付きで取得する。

```rust
// レスポンス（200 OK）
{
    "user": { "id": "...", "email": "...", "name": "...", "status": "active" },
    "roles": ["user"],
    "permissions": ["workflow:read", "task:read"]
}
```

---

## テスト

### ユースケース層テスト（auth.rs）

| テスト | 検証内容 |
|-------|---------|
| `test_正しい認証情報でユーザーを取得できる` | 認証成功でユーザーとロールを返す |
| `test_不正なパスワードで認証失敗` | パスワード不一致で AuthenticationFailed |
| `test_存在しないユーザーで認証失敗` | ユーザー不存在で AuthenticationFailed |
| `test_非アクティブユーザーは認証失敗` | ステータス確認で AuthenticationFailed |

### ハンドラ層テスト（handler/auth.rs）

| テスト | 検証内容 |
|-------|---------|
| `test_verify_正しい認証情報で認証できる` | 200 OK |
| `test_verify_不正なパスワードで401` | 401 Unauthorized |
| `test_get_user_ユーザー情報を取得できる` | 200 OK |
| `test_get_user_存在しないユーザーで404` | 404 Not Found |

### テスト実行

```bash
cd backend && cargo test -p ringiflow-core-service
```

---

## 関連ドキュメント

- 設計書: [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md)

---

# 設計解説

以下は設計判断の詳細解説。レビュー・学習用。

---

## 1. タイミング攻撃対策

**場所:** [`auth.rs:91-97`](../../../backend/apps/core-service/src/usecase/auth.rs#L91-L97)

```rust
// ユーザーが存在しない場合、タイミング攻撃対策としてダミー検証を実行
let Some(user) = user_result else {
    let dummy_hash = PasswordHash::new(
        "$argon2id$v=19$m=65536,t=1,p=1$AAAAAAAAAAAAAAAAAAAAAA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    );
    let _ = self.password_checker.verify(password, &dummy_hash);
    return Err(AuthError::AuthenticationFailed);
};
```

**なぜダミー検証が必要か:**

攻撃者はレスポンス時間の差から情報を推測できる:

| ケース | 処理時間 | 推測可能な情報 |
|-------|---------|--------------|
| ユーザーが存在しない | 短い | 「このメールは登録されていない」 |
| パスワードが不一致 | 長い | 「このメールは登録されている」 |

ダミーのパスワード検証を実行することで、処理時間を均一化し、
ユーザーの存在有無を推測困難にする。

**代替案:**

1. **固定遅延を追加:** `tokio::time::sleep(Duration::from_millis(500)).await`
   - シンプルだが、実際の検証時間が変動すると差が出る可能性
2. **ダミー検証:** 実際の Argon2 検証を実行
   - 処理時間が実際の検証と同等になるため、より確実

---

## 2. ジェネリクスを使った依存注入

**場所:** [`auth.rs:39-52`](../../../backend/apps/core-service/src/usecase/auth.rs#L39-L52)

```rust
pub struct AuthUseCase<R, P>
where
    R: UserRepository,
    P: PasswordChecker,
{
    user_repository: R,
    password_checker: P,
}
```

**なぜ動的ディスパッチ（`dyn Trait`）ではなくジェネリクスか:**

| 方式 | 利点 | 欠点 |
|-----|------|------|
| ジェネリクス | ゼロコスト抽象化、インライン化可能 | コンパイル時に型が確定する必要 |
| `Box<dyn Trait>` | 実行時に型を切り替え可能 | vtable 参照のオーバーヘッド |

本プロジェクトでは:
- テスト時: スタブ実装を注入
- 本番時: PostgresUserRepository, Argon2PasswordChecker を注入

型はコンパイル時に確定するため、ジェネリクスで十分。

**テスト時のスタブ:**

```rust
struct StubUserRepository { /* ... */ }
struct StubPasswordChecker { result: PasswordVerifyResult }

let usecase = AuthUseCase::new(
    StubUserRepository::with_user(user, roles),
    StubPasswordChecker::matching(),
);
```

---

## 3. 認証エラーを単一化する理由

**場所:** [`auth.rs:22-34`](../../../backend/apps/core-service/src/usecase/auth.rs#L22-L34)

```rust
pub enum AuthError {
    /// 認証失敗（メール不存在、パスワード不一致、非アクティブ）
    AuthenticationFailed,
    /// インフラ層エラー
    Internal(#[from] InfraError),
}
```

**なぜ詳細なエラー型を作らないか:**

| アプローチ | セキュリティ | デバッグ容易性 |
|-----------|------------|--------------|
| 詳細なエラー型 | 情報漏洩リスク | 高い |
| 単一エラー型 | 安全 | ログで補完 |

```rust
// 危険な例（情報漏洩）
enum AuthError {
    UserNotFound,      // 「このメールは未登録」が分かる
    PasswordMismatch,  // 「メールは登録済み」が分かる
    UserInactive,      // 「アカウントがある」が分かる
}
```

単一の `AuthenticationFailed` を返すことで、
攻撃者に有用な情報を与えない。

内部ログには詳細を記録することで、デバッグ性を確保:

```rust
tracing::warn!(
    email = %email,
    reason = "user_not_found",
    "認証失敗"
);
```

---

## 4. ハンドラでのジェネリック型指定

**場所:** [`main.rs:119-125`](../../../backend/apps/core-service/src/main.rs#L119-L125)

```rust
.route(
    "/internal/auth/verify",
    post(verify::<PostgresUserRepository, Argon2PasswordChecker>),
)
```

**なぜ型パラメータを明示的に指定するか:**

axum のルーターはハンドラの型を静的に解決する必要がある。
ジェネリックなハンドラの場合、使用する具体型を明示しないとコンパイルできない。

**代替案:**

1. **ハンドラを具体型で定義:** ジェネリクスを使わない
   - テスタビリティが下がる
2. **型エイリアスを使用:**
   ```rust
   type AuthHandler = verify<PostgresUserRepository, Argon2PasswordChecker>;
   ```
   - 可読性は上がるが、本質的には同じ

---

## 5. State の Arc ラップ

**場所:** [`main.rs:112-114`](../../../backend/apps/core-service/src/main.rs#L112-L114)

```rust
let auth_state = Arc::new(AuthState {
    usecase: auth_usecase,
});
```

**なぜ Arc が必要か:**

axum の `State` エクストラクタは `Clone` を要求する。
複数のリクエストハンドラが同時に State にアクセスするため、
所有権を共有する必要がある。

`Arc` は参照カウントベースのスマートポインタで:
- Clone が低コスト（カウンタのインクリメントのみ）
- 複数スレッドから安全にアクセス可能
- 内部データは不変（`AuthUseCase` は変更しない）

**`Mutex` は不要か:**

`AuthUseCase` は内部で状態を変更しないため、`Mutex` は不要。
`UserRepository` や `PasswordChecker` も `&self` でメソッドを呼び出すため、
並行アクセスに対応している。
