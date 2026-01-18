# Newtype パターン

## 概要

Newtype パターンは、既存の型をラップした新しい型を作成するパターン。
Rust では tuple struct（タプル構造体）を使用して実装する。

## 基本的な使い方

### 定義

```rust
pub struct UserId(Uuid);
pub struct TenantId(Uuid);
pub struct Email(String);
```

内部の値には private でアクセスし、public メソッドで公開:

```rust
impl UserId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}
```

### 利点

#### 1. 型安全性

同じ内部型（UUID）でも、Newtype で区別できる:

```rust
// Good: コンパイルエラー
fn update_user(user_id: UserId, tenant_id: TenantId) { /* ... */ }

let user = UserId::new();
let tenant = TenantId::new();
update_user(tenant, user);  // ❌ コンパイルエラー: 型が一致しない

// Bad: 型がないと取り違えに気づかない
fn update_user(user_id: Uuid, tenant_id: Uuid) { /* ... */ }
update_user(tenant, user);  // ✅ コンパイル成功（バグ！）
```

#### 2. ゼロコスト抽象化

Newtype は実行時にオーバーヘッドがない:

```rust
// メモリレイアウトは Uuid と同じ
#[repr(transparent)]
pub struct UserId(Uuid);
```

最適化により、実行時には `UserId` と `Uuid` の区別は消える。

#### 3. 意図の明確化

型名自体がドメインの概念を表現:

```rust
// Bad: 意図が不明
fn find(id: Uuid) -> Option<User> { /* ... */ }

// Good: 意図が明確
fn find(id: UserId) -> Option<User> { /* ... */ }
```

## 実装パターン

### シリアライズ・デシリアライズ

Serde の `#[serde(transparent)]` で、内部型と同じフォーマットで扱える:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UserId(Uuid);
```

JSON での表現:

```json
{
  "id": "01234567-89ab-cdef-0123-456789abcdef"  // UserId は UUID として扱われる
}
```

### 表示の実装

`Display` トレイトで人間可読な出力を提供:

```rust
impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// 使用例
let user_id = UserId::new();
println!("User ID: {}", user_id);  // "User ID: 01234567-..."
```

### ハッシュと等価性

比較やハッシュが必要な場合は derive で自動実装:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);

// HashMap や HashSet で使用可能
let mut map = HashMap::new();
map.insert(user_id, user);
```

### デフォルト値

`Default` トレイトで新しい ID を生成:

```rust
impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

// 使用例
let user_id: UserId = Default::default();
```

## バリデーション付き Newtype

値オブジェクトとして使用する場合、生成時にバリデーションを実行:

```rust
pub struct Email(String);

impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();

        // バリデーション
        if value.is_empty() {
            return Err(DomainError::Validation("メールアドレスは必須です".to_string()));
        }
        if !value.contains('@') {
            return Err(DomainError::Validation("メールアドレスの形式が不正です".to_string()));
        }
        if value.len() > 255 {
            return Err(DomainError::Validation("メールアドレスは255文字以内".to_string()));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

**不正な状態を表現不可能にする（Make Illegal States Unrepresentable）**:

```rust
// ❌ これはコンパイルできない（Email のコンストラクタが private）
let invalid = Email("".to_string());

// ✅ 正しい方法: バリデーションを通過した Email のみ生成可能
let email = Email::new("user@example.com")?;
```

## ベストプラクティス

### 内部フィールドは private に

外部から直接アクセスできないようにし、メソッド経由でのみ操作:

```rust
// Good
pub struct UserId(Uuid);  // private field

impl UserId {
    pub fn as_uuid(&self) -> &Uuid {
        &self.0  // メソッド経由でアクセス
    }
}

// Bad
pub struct UserId(pub Uuid);  // public field（カプセル化が破れる）
```

### Into/From トレイトの実装

必要に応じて変換トレイトを実装:

```rust
// Uuid → UserId
impl From<Uuid> for UserId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

// &UserId → &Uuid（参照の変換）
impl AsRef<Uuid> for UserId {
    fn as_ref(&self) -> &Uuid {
        &self.0
    }
}

// 使用例
let uuid = Uuid::new_v4();
let user_id: UserId = uuid.into();
```

### SQLx との統合

データベースとのやり取りでは、内部型への変換が必要:

```rust
// クエリで使用
sqlx::query!(
    "SELECT * FROM users WHERE id = $1",
    user_id.as_uuid()  // &Uuid に変換
)
.fetch_one(&pool)
.await?;

// 結果から復元
let row = sqlx::query!("SELECT id FROM users")
    .fetch_one(&pool)
    .await?;
let user_id = UserId::from_uuid(row.id);
```

## よくある使用例

### ID 型

```rust
pub struct UserId(Uuid);
pub struct TenantId(Uuid);
pub struct WorkflowId(Uuid);
```

### 値オブジェクト

```rust
pub struct Email(String);
pub struct PhoneNumber(String);
pub struct Money(i64);  // 通貨の最小単位（例: 円、セント）
```

### 単位付き数値

```rust
pub struct Meters(f64);
pub struct Seconds(u64);
pub struct Percentage(f64);
```

## 関連パターン

### 複数フィールドを持つ値オブジェクト

Newtype は1つのフィールドだけだが、複数フィールドが必要な場合は通常の struct:

```rust
pub struct Address {
    postal_code: String,
    prefecture: String,
    city: String,
    street: String,
}

impl Address {
    pub fn new(/* ... */) -> Result<Self, DomainError> {
        // バリデーション
        // ...
    }
}
```

### Enum による型区別

異なる種類の ID を区別する場合:

```rust
pub enum EntityId {
    User(UserId),
    Tenant(TenantId),
    Workflow(WorkflowId),
}
```

## まとめ

Newtype パターンの利点:
- ✅ コンパイル時の型安全性
- ✅ ゼロコスト抽象化
- ✅ 意図の明確化
- ✅ 不正な状態の排除（バリデーション付き）

RingiFlow での使用例:
- `UserId`, `TenantId`, `RoleId` などの ID 型
- `Email` などの値オブジェクト
- `Permission` などのドメイン概念
- `Version`, `UserName`, `WorkflowName` などの共通値オブジェクト（`value_objects` モジュール）

### value_objects モジュール

複数のエンティティで共有される値オブジェクトは `value_objects.rs` に集約:

```rust
// backend/crates/domain/src/value_objects.rs

/// バージョン番号（1 以上を保証）
pub struct Version(u32);

/// ユーザー表示名（空文字禁止、100 文字以内）
pub struct UserName(String);

/// ワークフロー名（空文字禁止、200 文字以内）
pub struct WorkflowName(String);
```

Newtype 化の判断基準は [ADR-016](../05_ADR/016_プリミティブ型のNewtype化方針.md) を参照。

---

参考資料:
- [The Rust Programming Language - Using the Newtype Pattern](https://doc.rust-lang.org/book/ch19-04-advanced-types.html#using-the-newtype-pattern-for-type-safety-and-abstraction)
- [Rust Design Patterns - Newtype](https://rust-unofficial.github.io/patterns/patterns/behavioural/newtype.html)

関連ドキュメント:
- [Rust実装ルール](../../.claude/rules/rust.md)
- [DDD エンティティパターン](./DDD_エンティティパターン.md)
