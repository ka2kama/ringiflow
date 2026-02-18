# DDD エンティティパターン

## 概要

ドメイン駆動設計（DDD）におけるエンティティの実装パターン。
エンティティは **一意の識別子を持つオブジェクト** であり、ライフサイクルと状態を管理する。

## エンティティの特徴

### 1. 一意の識別子

エンティティは ID により識別される。属性が同じでも ID が異なれば別物:

```rust
pub struct User {
    id: UserId,     // 識別子
    name: String,   // 属性
    email: Email,   // 属性
}

// 同じ名前・メールでも、ID が違えば別のユーザー
let user1 = User { id: UserId::new(), name: "Alice", email: alice@example.com };
let user2 = User { id: UserId::new(), name: "Alice", email: alice@example.com };
assert_ne!(user1.id(), user2.id());
```

### 2. 状態の変更可能性（Mutability）

エンティティは時間とともに状態が変わる:

```rust
impl User {
    pub fn change_status(&mut self, status: UserStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }
}

let mut user = User::new(/* ... */);
user.change_status(UserStatus::Inactive);  // 状態を変更
```

### 3. ライフサイクル

エンティティは作成→更新→削除のライフサイクルを持つ:

```rust
// 作成
let user = User::new(tenant_id, email, name, password_hash);

// 更新
user.update_last_login();
user.change_status(UserStatus::Active);

// 削除（論理削除）
user.delete();  // status を Deleted に変更
```

## Rust での実装パターン

### 基本構造

```rust
pub struct User {
    // 識別子（必須）
    id: UserId,
    tenant_id: TenantId,

    // ビジネス属性
    email: Email,
    name: String,
    status: UserStatus,

    // メタデータ
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

**設計判断**:
- フィールドは **private**: 外部から直接変更できない
- Getter で **参照** を返す（所有権を渡さない）
- 状態変更は **メソッド経由**: ビジネスルールを強制

### コンストラクタ

新規作成と DB からの復元で2つのコンストラクタを用意:

```rust
impl User {
    /// 新規作成（ビジネスロジック）
    pub fn new(
        tenant_id: TenantId,
        email: Email,
        name: String,
        password_hash: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::new(),
            tenant_id,
            email,
            name,
            password_hash,
            status: UserStatus::Active,  // デフォルト値
            last_login_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// DB から復元（インフラ層で使用）
    #[allow(clippy::too_many_arguments)]
    pub fn from_db(
        id: UserId,
        tenant_id: TenantId,
        email: Email,
        name: String,
        password_hash: Option<String>,
        status: UserStatus,
        last_login_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            tenant_id,
            email,
            name,
            password_hash,
            status,
            last_login_at,
            created_at,
            updated_at,
        }
    }
}
```

**設計判断**:
- `new()`: ビジネスルールを適用（デフォルト値、初期状態）
- `from_db()`: すべてのフィールドを受け取り、そのまま復元

### Getter メソッド

参照を返すことで、所有権の移動を避ける:

```rust
impl User {
    pub fn id(&self) -> &UserId {
        &self.id
    }

    pub fn tenant_id(&self) -> &TenantId {
        &self.tenant_id
    }

    pub fn email(&self) -> &Email {
        &self.email
    }

    pub fn name(&self) -> &str {
        &self.name  // String → &str に変換
    }

    pub fn status(&self) -> UserStatus {
        self.status  // Copy trait なので値渡し
    }
}
```

**設計判断**:
- 参照を返すことで、呼び出し側でクローンが必要になる
- `Copy` trait を実装した型（enum など）は値渡し可能

### 状態変更メソッド（不変アプローチ）

新しいインスタンスを返すことでビジネスロジックを強制:

```rust
impl User {
    /// 最終ログイン日時を更新した新しいインスタンスを返す
    pub fn with_last_login_updated(self) -> Self {
        Self {
            last_login_at: Some(Utc::now()),
            updated_at: Utc::now(),
            ..self
        }
    }

    /// ステータス変更した新しいインスタンスを返す
    pub fn with_status(self, status: UserStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self
        }
    }

    /// パスワード更新した新しいインスタンスを返す
    pub fn with_password_hash(self, password_hash: String) -> Self {
        Self {
            password_hash: Some(password_hash),
            updated_at: Utc::now(),
            ..self
        }
    }

    /// 論理削除した新しいインスタンスを返す
    pub fn deleted(self) -> Self {
        Self {
            status: UserStatus::Deleted,
            updated_at: Utc::now(),
            ..self
        }
    }
}

// 使用例
let user = User::new(/* ... */);
let updated_user = user.with_status(UserStatus::Inactive);  // 新しいインスタンス
```

**不変条件の保護**:
- `updated_at` は自動的に更新される（忘れることがない）
- 外部から直接 `status` を変更できない
- ビジネスルールをメソッド内で強制（例: 削除時は必ず Deleted ステータス）
- 所有権が移動するため、古いインスタンスは使えなくなる（不変性の保証）

### ビジネスロジックメソッド

エンティティに振る舞いを持たせる:

```rust
impl User {
    /// アクティブかどうか判定
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// ログイン可能かどうか判定
    pub fn can_login(&self) -> bool {
        self.is_active()
    }

    /// パスワード認証が必要か（SSO ユーザーは不要）
    pub fn requires_password(&self) -> bool {
        self.password_hash.is_some()
    }
}
```

**設計判断**:
- エンティティ自身に判定ロジックを持たせる（貧血ドメインモデルを避ける）
- 呼び出し側でフィールドを直接チェックしない

## エンティティ vs 値オブジェクト

| | エンティティ | 値オブジェクト |
|---|-------------|--------------|
| 識別子 | あり（ID） | なし |
| 可変性 | 可変（状態が変わる） | 不変（変更は新しいインスタンス生成） |
| 等価性 | ID で判定 | すべての属性で判定 |
| 例 | User, Workflow | Email, Money, Address |

### 値オブジェクトの例

```rust
#[derive(Debug, Clone, PartialEq, Eq)]  // 等価性は全フィールド
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        // バリデーション...
        Ok(Self(value))
    }

    // 不変なので、変更は新しいインスタンスを返す
    pub fn with_domain(&self, domain: &str) -> Result<Self, DomainError> {
        let local_part = self.0.split('@').next().unwrap();
        Email::new(format!("{}@{}", local_part, domain))
    }
}
```

## 集約（Aggregate）

関連するエンティティをまとめたもの。集約ルート（Aggregate Root）がトランザクション境界を定義:

```rust
// WorkflowInstance が集約ルート
pub struct WorkflowInstance {
    id: WorkflowInstanceId,
    // ...
}

// WorkflowStep は集約内のエンティティ
pub struct WorkflowStep {
    id: WorkflowStepId,
    instance_id: WorkflowInstanceId,  // 集約ルートへの参照
    // ...
}
```

**設計判断**:
- `WorkflowInstance` と `WorkflowStep` は常に一緒に扱う（トランザクション）
- 外部からは `WorkflowInstance` 経由でのみアクセス
- `WorkflowStep` 単独での保存・削除は行わない

## テスト

エンティティのテストは、ビジネスロジックに焦点を当てる:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_user_creation() {
        let tenant_id = TenantId::new();
        let email = Email::new("user@example.com").unwrap();
        let user = User::new(tenant_id, email, "Test User".to_string(), None);

        assert!(user.is_active());
        assert!(user.can_login());
        assert_eq!(user.name(), "Test User");
    }

    #[test]
    fn test_user_delete() {
        let mut user = create_test_user();

        user.delete();

        assert_eq!(user.status(), UserStatus::Deleted);
        assert!(!user.can_login());
    }

    #[test]
    fn test_update_last_login_updates_timestamp() {
        let mut user = create_test_user();
        let before = user.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(10));
        user.update_last_login();

        let after = user.updated_at();
        assert!(after > before);  // updated_at が更新されている
    }
}
```

## ベストプラクティス

### 1. フィールドを private に

外部から直接変更できないようにし、不変条件を保護:

```rust
// Good
pub struct User {
    id: UserId,        // private
    status: UserStatus, // private
}

// Bad
pub struct User {
    pub id: UserId,        // 外部から変更可能（不変条件が破れる）
    pub status: UserStatus,
}
```

### 2. 状態変更はメソッド経由

ビジネスルールをメソッド内で強制:

```rust
// Good
impl User {
    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
        self.updated_at = Utc::now();  // 自動的に更新
    }
}

// Bad: 外部で直接変更（updated_at の更新を忘れるリスク）
user.status = UserStatus::Active;
user.updated_at = Utc::now();  // 忘れる可能性
```

### 3. ビジネスロジックはエンティティに

判定ロジックをエンティティに持たせ、貧血ドメインモデルを避ける:

```rust
// Good: エンティティに振る舞い
impl User {
    pub fn can_login(&self) -> bool {
        self.status == UserStatus::Active
    }
}

// 呼び出し側
if user.can_login() {
    // ログイン処理
}

// Bad: 外部でフィールドを直接チェック
if user.status() == UserStatus::Active {
    // ログイン処理
}
```

### 4. DB 復元用のコンストラクタを分ける

新規作成（`new()`）と DB 復元（`from_db()`）を分離:

```rust
// 新規作成: デフォルト値、初期状態を設定
let user = User::new(tenant_id, email, name, password_hash);

// DB 復元: すべての値をそのまま設定
let user = User::from_db(id, tenant_id, email, name, password_hash, status, last_login_at, created_at, updated_at);
```

## まとめ

DDD エンティティの実装ポイント:
- ✅ 一意の識別子を持つ
- ✅ フィールドは private、メソッド経由でアクセス
- ✅ 状態変更はメソッド内でビジネスルールを強制
- ✅ ビジネスロジックをエンティティに持たせる
- ✅ 不変条件を保護する

RingiFlow での使用例:
- `User`, `Role`, `WorkflowInstance`, `WorkflowStep`

---

参考資料:
- Eric Evans『ドメイン駆動設計』
- Vaughn Vernon『実践ドメイン駆動設計』

関連ドキュメント:
- [Rust実装ルール](../../../.claude/rules/rust.md)
- [Newtype パターン](../rust/Newtypeパターン.md)
