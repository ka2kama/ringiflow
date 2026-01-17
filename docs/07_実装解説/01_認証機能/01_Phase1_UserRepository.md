# Phase 1: UserRepository

## 概要

認証機能の基盤となるユーザーリポジトリを実装した。
メールアドレスによるユーザー検索、ロール付き取得、最終ログイン日時更新の機能を提供する。

### 対応 Issue

[#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34) - Phase 1

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [実装コンポーネント > 実装順序](../../03_詳細設計書/07_認証機能設計.md#実装順序) | Phase 1 の位置づけ |
| [実装コンポーネント > ファイル構成](../../03_詳細設計書/07_認証機能設計.md#ファイル構成) | ディレクトリ構造 |
| [インターフェース定義 > UserRepository](../../03_詳細設計書/07_認証機能設計.md#userrepository) | トレイト定義 |
| [テスト計画 > 統合テスト](../../03_詳細設計書/07_認証機能設計.md#統合テスト) | テストケース |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/domain/src/user.rs`](../../../backend/crates/domain/src/user.rs) | User エンティティ、UserId、Email、UserStatus |
| [`backend/crates/domain/src/role.rs`](../../../backend/crates/domain/src/role.rs) | Role エンティティ、RoleId、Permission |
| [`backend/crates/infra/src/repository/user_repository.rs`](../../../backend/crates/infra/src/repository/user_repository.rs) | UserRepository トレイト + PostgreSQL 実装 |
| [`backend/crates/infra/tests/user_repository_test.rs`](../../../backend/crates/infra/tests/user_repository_test.rs) | 統合テスト |

---

## 実装内容

### User エンティティ（[`user.rs`](../../../backend/crates/domain/src/user.rs)）

**値オブジェクト:**

| 型 | 説明 |
|----|------|
| `UserId` | UUID v7 ベースのユーザー ID |
| `Email` | バリデーション済みメールアドレス |
| `UserStatus` | Active / Inactive / Deleted |

**主要メソッド:**

```rust
// 新規作成（ビジネスルールを適用）
User::new(tenant_id, email, name, password_hash) -> Self

// DB から復元
User::from_db(...) -> Self

// 状態変更（不変、新インスタンスを返す）
user.with_last_login_updated() -> Self
user.with_status(status) -> Self
user.deleted() -> Self

// 判定
user.is_active() -> bool
user.can_login() -> bool
```

### Role エンティティ（[`role.rs`](../../../backend/crates/domain/src/role.rs)）

**権限モデル:**

```
*              → 全権限
workflow:*     → workflow 関連すべて
workflow:read  → 読み取りのみ
```

**主要メソッド:**

```rust
Role::new_system(name, description, permissions) -> Self  // システムロール
Role::new_tenant(tenant_id, name, ...) -> Self            // テナント固有

role.has_permission(&permission) -> bool
role.can_delete() -> Result<(), DomainError>  // システムロールは不可
```

### UserRepository トレイト（[`user_repository.rs`](../../../backend/crates/infra/src/repository/user_repository.rs)）

```rust
#[async_trait]
pub trait UserRepository: Send + Sync {
    // テナント内でメール検索
    async fn find_by_email(&self, tenant_id: &TenantId, email: &Email)
        -> Result<Option<User>, InfraError>;

    // ID で検索（内部 API 向け）
    async fn find_by_id(&self, id: &UserId)
        -> Result<Option<User>, InfraError>;

    // ロール付きで取得
    async fn find_with_roles(&self, id: &UserId)
        -> Result<Option<(User, Vec<Role>)>, InfraError>;

    // 最終ログイン日時を更新
    async fn update_last_login(&self, id: &UserId) -> Result<(), InfraError>;
}
```

---

## テスト

### テストケース

| テスト | 検証内容 |
|-------|---------|
| `test_メールアドレスでユーザーを取得できる` | 正常系 |
| `test_存在しないメールアドレスの場合noneを返す` | None ケース |
| `test_別テナントのユーザーは取得できない` | テナント分離 |
| `test_idでユーザーを取得できる` | ID 検索 |
| `test_ユーザーとロールを一緒に取得できる` | JOIN 取得 |
| `test_最終ログイン日時を更新できる` | UPDATE |

### テスト実行

```bash
just setup-db
cd backend && cargo test -p ringiflow-infra --test user_repository_test
```

---

## 関連ドキュメント

- 設計書: [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md)
- 技術ノート: [エンタープライズ認証とID管理.md](../../06_技術ノート/エンタープライズ認証とID管理.md)

---

# 設計解説

以下は設計判断の詳細解説。レビュー・学習用。

---

## 1. Newtype パターン

**場所:** [`user.rs:41-59`](../../../backend/crates/domain/src/user.rs#L41-L59)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(Uuid);
```

**なぜこの設計か:**

1. **型安全性**: `UserId` と `TenantId` は両方 UUID だが、コンパイラが混同を防ぐ
   - 誤って `find_by_email(user_id, email)` のように呼び出すとコンパイルエラー
   - 実行時エラーではなくコンパイルエラーで検出できる

2. **意図の明確化**: 関数シグネチャが自己文書化される
   ```rust
   // 良い: 引数の意味が明確
   fn find_by_email(&self, tenant_id: &TenantId, email: &Email) -> ...

   // 悪い: 引数の意味が不明確
   fn find_by_email(&self, tenant_id: &Uuid, email: &str) -> ...
   ```

3. **UUID バージョンの使い分け**:
   - `UserId::new()` は `Uuid::now_v7()` を使用（時系列ソート可能）
   - 将来的に v4 に変更しても、呼び出し側のコードは変更不要

**代替案:**

| 案 | メリット | デメリット |
|----|---------|-----------|
| 生の Uuid を使う | シンプル | 型安全性なし、意図不明確 |
| type alias `type UserId = Uuid` | 読みやすい | コンパイラが区別しない |
| **Newtype（採用）** | 型安全、拡張可能 | ボイラープレート増加 |

---

## 2. 値オブジェクトとバリデーション

**場所:** [`user.rs:80-121`](../../../backend/crates/domain/src/user.rs#L80-L121)

```rust
impl Email {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        if value.is_empty() { return Err(...); }
        // バリデーション...
        Ok(Self(value))
    }
}
```

**設計原則: 「不正な状態を表現不可能にする」（Make Illegal States Unrepresentable）**

- `Email` 型が存在すれば、それは必ず有効なメールアドレス
- バリデーションは生成時に1回だけ行われる
- 後続のコードでは「このメールは有効か？」を気にする必要がない

**なぜ `Result` を返すか:**

- 呼び出し側にエラーハンドリングを強制
- `unwrap()` や `expect()` を避けることを促す
- 型システムがエラーの可能性を表現

**バリデーションの範囲:**

本実装では簡易的なバリデーション（`@` の存在確認）に留めている。

理由:
- RFC 5322 完全準拠は複雑すぎる（正規表現で数百文字）
- 実際の検証はメール送信で行う（確認メール）
- 入力時は「明らかな間違い」を弾く程度で十分

---

## 3. エンティティの不変性と状態変更

**場所:** [`user.rs:323-329`](../../../backend/crates/domain/src/user.rs#L323-L329)

```rust
pub fn with_last_login_updated(self) -> Self {
    Self {
        last_login_at: Some(Utc::now()),
        updated_at: Utc::now(),
        ..self
    }
}
```

**なぜ「新しいインスタンスを返す」のか:**

1. **不変性の保証**: 元のインスタンスは変更されない
   - 並行処理でのデータ競合を防ぐ
   - 意図しない副作用を防ぐ

2. **変更の明示化**: `self` を消費するので、変更が明示的
   ```rust
   let user = user.with_last_login_updated();  // 明示的に再代入
   ```

3. **Rust の所有権システムとの親和性**:
   - `&mut self` より `self` を消費する方が安全
   - ビルダーパターンのようにチェーン可能

**代替案:**

| 案 | メリット | デメリット |
|----|---------|-----------|
| `&mut self` で変更 | 効率的 | 副作用が隠れる、並行処理で問題 |
| **`self` を消費して新インスタンス（採用）** | 安全、明示的 | 若干のオーバーヘッド |
| 内部可変性 (`RefCell`) | 柔軟 | ランタイムコスト、複雑性増 |

---

## 4. ファクトリメソッドのパターン

**場所:** [`user.rs:224-268`](../../../backend/crates/domain/src/user.rs#L224-L268)

**2つのファクトリメソッドの使い分け:**

| メソッド | 用途 | 特徴 |
|---------|------|------|
| `new()` | 新規作成 | ビジネスルールを適用（ID生成、初期状態設定） |
| `from_db()` | DB復元 | バリデーションなし（すでに検証済みデータ） |

**なぜ分けるか:**

- `new()`: ドメインの不変条件を保証
  - 「新規ユーザーは必ず Active」などのルールをコードで表現
- `from_db()`: DB のデータを信頼
  - DB に保存されたデータは過去に検証済み
  - 毎回バリデーションすると無駄なオーバーヘッド

---

## 5. 権限のワイルドカードマッチング

**場所:** [`role.rs:112-129`](../../../backend/crates/domain/src/role.rs#L112-L129)

```rust
pub fn includes(&self, other: &Permission) -> bool {
    if self.is_wildcard() { return true; }  // * は全権限を包含
    if self.0 == other.0 { return true; }   // 完全一致

    // resource:* 形式のチェック
    if let Some(resource) = self.0.strip_suffix(":*")
        && let Some((other_resource, _)) = other.0.split_once(':')
    {
        return resource == other_resource;
    }
    false
}
```

**権限の階層構造:**

```
*                  → 全権限（システム管理者）
├── workflow:*     → workflow 関連すべて
│   ├── workflow:read
│   ├── workflow:create
│   └── workflow:delete
└── task:*         → task 関連すべて
    ├── task:read
    └── task:update
```

**let-else と if-let チェーンの活用:**

Rust 1.65+ の `let-else` と Rust nightly の `if-let chains` を活用して、
ネストを減らしつつ早期リターンを実現している。

---

## 6. リポジトリパターン

**場所:** [`user_repository.rs:23-59`](../../../backend/crates/infra/src/repository/user_repository.rs#L23-L59)

**なぜトレイト（インターフェース）を定義するか:**

1. **テスタビリティ**: モック実装に差し替え可能
   ```rust
   struct MockUserRepository { ... }
   impl UserRepository for MockUserRepository { ... }
   ```

2. **依存性逆転（DIP）**: ユースケース層はトレイトに依存
   - 具体的な DB 実装には依存しない
   - PostgreSQL → MySQL への変更がユースケース層に影響しない

3. **関心の分離**: ドメイン層は「何をするか」を定義、インフラ層は「どうやるか」を実装

**`Send + Sync` の意味:**

- `Send`: スレッド間で所有権を移動できる
- `Sync`: スレッド間で参照を共有できる
- 非同期ランタイム（tokio）で使用するために必須

---

## 7. テナント分離

**場所:** [`user_repository.rs:81-100`](../../../backend/crates/infra/src/repository/user_repository.rs#L81-L100)

**マルチテナント設計の原則:**

1. **API 設計でテナント分離を強制**: `tenant_id` を引数に含めることで、クエリ漏れを防ぐ
2. **WHERE 句にテナント ID を含める**: 他テナントのデータが見えない
3. **テストで検証**: `test_別テナントのユーザーは取得できない` で確認

**`find_by_id` はテナント ID 不要な理由:**

- 内部 API 向け（BFF → Core API 間）
- 呼び出し元ですでに認証・認可済み
- User ID 自体がテナントに紐づいているため、クロステナントアクセスは発生しない

---

## 8. N+1 問題の回避

**場所:** [`user_repository.rs:165-218`](../../../backend/crates/infra/src/repository/user_repository.rs#L165-L218)

**N+1 問題とは:**

```
悪い例:
1. SELECT * FROM users WHERE id = ?           -- 1クエリ
2. SELECT * FROM roles WHERE id = ?           -- ロールごとに1クエリ
3. SELECT * FROM roles WHERE id = ?           -- ロールが3つなら3クエリ
→ 合計 1 + N クエリ
```

**本実装:**

```
1. SELECT * FROM users WHERE id = ?           -- 1クエリ
2. SELECT r.* FROM roles r                    -- 1クエリ（JOIN で一括）
   INNER JOIN user_roles ur ON ur.role_id = r.id
   WHERE ur.user_id = ?
→ 合計 2 クエリ（固定）
```

---

## 9. sqlx::test マクロ

**場所:** [`user_repository_test.rs:68`](../../../backend/crates/infra/tests/user_repository_test.rs#L68)

```rust
#[sqlx::test(migrations = "../../migrations")]
async fn test_メールアドレスでユーザーを取得できる(pool: PgPool) { ... }
```

**特徴:**

1. **トランザクション自動ロールバック**: テストごとにクリーンな DB 状態
2. **マイグレーション自動適用**: スキーマを最新化
3. **並列実行可能**: 各テストが独立したトランザクション
4. **接続プール自動管理**: `PgPool` を引数で受け取るだけ
