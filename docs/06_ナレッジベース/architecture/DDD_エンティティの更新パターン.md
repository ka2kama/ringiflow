# DDD エンティティの更新パターン

## 概要

エンティティの状態を更新する際、**可変アプローチ（`&mut self`）** と **不変アプローチ（`copy` メソッド）** の2つの方法がある。
どちらも有効で、状況に応じて使い分けることができる。

## 1. 可変アプローチ（現在の実装）

### 実装例

```rust
impl User {
    pub fn change_status(&mut self, status: UserStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }
}

// 使用例
let mut user = User::new(/* ... */);
user.change_status(UserStatus::Inactive);  // その場で変更
```

### メリット

- **パフォーマンス**: メモリ確保が不要、インプレースで更新
- **DB との親和性**: 永続化層で既存のレコードを UPDATE するモデルと一致
- **直感的**: 「ユーザーのステータスが変わる」という現実世界のモデルに近い

### デメリット

- **履歴が残らない**: 変更前の状態は失われる
- **並行処理**: 複数スレッドから変更する場合、排他制御が必要
- **テスト**: 変更前の状態を保持したい場合、事前にクローンが必要

### 適しているケース

- 通常の CRUD アプリケーション
- DB の UPDATE で状態を更新するモデル
- パフォーマンスが重要な場合
- エンティティが大きい場合（クローンコストが高い）

## 2. 不変アプローチ（Scala の `copy` 相当）

### 実装例

```rust
impl User {
    /// ステータスを変更した新しいインスタンスを返す
    pub fn with_status(self, status: UserStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self  // 他のフィールドはそのまま
        }
    }

    /// 参照から新しいインスタンスを作る場合
    pub fn with_status_cloned(&self, status: UserStatus) -> Self {
        Self {
            id: self.id.clone(),
            tenant_id: self.tenant_id.clone(),
            email: self.email.clone(),
            name: self.name.clone(),
            password_hash: self.password_hash.clone(),
            status,
            last_login_at: self.last_login_at,
            created_at: self.created_at,
            updated_at: Utc::now(),
        }
    }
}

// 使用例
let user = User::new(/* ... */);
let updated_user = user.with_status(UserStatus::Inactive);  // 新しいインスタンス
```

### メリット

- **不変性**: 元のインスタンスは変更されない
- **履歴管理**: 変更前の状態も保持できる
- **テストが容易**: 変更前後を比較しやすい
- **並行処理**: 複数スレッドで安全（データ競合がない）
- **関数型プログラミング**: 関数型の慣習に従う

### デメリット

- **メモリコスト**: 毎回新しいインスタンスを確保
- **クローンコスト**: すべてのフィールドをコピー（特に `String` や `Vec` は高コスト）
- **DB との不一致**: 新しいインスタンスを作るが、DB では UPDATE する（概念のズレ）

### 適しているケース

- イベントソーシング（変更履歴を保持）
- 並行処理が多い場合
- エンティティが小さい場合（クローンコストが低い）
- 関数型プログラミングスタイルを重視する場合

## 3. ハイブリッドアプローチ

両方のメソッドを提供することも可能:

```rust
impl User {
    /// 可変メソッド（パフォーマンス重視）
    pub fn change_status(&mut self, status: UserStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// 不変メソッド（関数型スタイル）
    pub fn with_status(self, status: UserStatus) -> Self {
        Self {
            status,
            updated_at: Utc::now(),
            ..self
        }
    }
}

// 使用例：状況に応じて選択
// パフォーマンス重視
let mut user = load_user();
user.change_status(UserStatus::Inactive);
save_user(&user);

// 履歴管理が必要
let user = load_user();
let updated = user.with_status(UserStatus::Inactive);
history.push(user);  // 変更前の状態を保持
save_user(&updated);
```

## 4. Builder パターン

複数のフィールドを一度に更新する場合は Builder パターンも有効:

```rust
pub struct UserBuilder {
    user: User,
}

impl UserBuilder {
    pub fn new(user: User) -> Self {
        Self { user }
    }

    pub fn status(mut self, status: UserStatus) -> Self {
        self.user.status = status;
        self
    }

    pub fn name(mut self, name: String) -> Self {
        self.user.name = name;
        self
    }

    pub fn build(mut self) -> User {
        self.user.updated_at = Utc::now();
        self.user
    }
}

// 使用例
let updated = UserBuilder::new(user)
    .status(UserStatus::Active)
    .name("New Name".to_string())
    .build();
```

## RingiFlow での推奨アプローチ

### 採用：不変アプローチ

RingiFlow では **不変アプローチ（`self` を消費）** を採用している。

**理由**:
1. **Phase 4 への準備**: イベントソーシング導入時の書き直しを避ける
2. **並行処理**: 将来的な並行処理で安全
3. **関数型スタイル**: Rust の所有権システムと相性が良い
4. **履歴管理**: 変更前の状態を保持しやすい

### Phase 4 でのイベントソーシング

Phase 4（イベントソーシング導入）では、既存の不変アプローチをそのまま活用できる:

```rust
// イベントソーシングでは、イベントの適用で新しい状態を作る
impl User {
    pub fn apply_event(self, event: UserEvent) -> Self {
        match event {
            UserEvent::StatusChanged { status, .. } => self.with_status(status),
            UserEvent::PasswordUpdated { hash, .. } => self.with_password(hash),
            // ...
        }
    }
}
```

## エンティティの「同一性」は変わらない

**重要**: どちらのアプローチでも、エンティティの **同一性（ID）は変わらない**:

```rust
// 可変アプローチ
let mut user = User::new(/* ... */);
let id = user.id().clone();
user.change_status(UserStatus::Inactive);
assert_eq!(user.id(), &id);  // ID は同じ

// 不変アプローチ
let user = User::new(/* ... */);
let id = user.id().clone();
let updated = user.with_status(UserStatus::Inactive);
assert_eq!(updated.id(), &id);  // ID は同じ
```

エンティティの本質は「ID による同一性」であり、更新方法は実装の詳細。

## 比較表

| 観点 | 可変アプローチ | 不変アプローチ |
|------|--------------|--------------|
| メモリ | ✅ 低（再利用） | ❌ 高（毎回確保） |
| パフォーマンス | ✅ 高速 | ❌ クローンコスト |
| 並行処理 | ⚠️ 排他制御必要 | ✅ 安全 |
| 履歴管理 | ❌ 困難 | ✅ 容易 |
| テスト | ⚠️ クローンが必要 | ✅ 容易 |
| DB との親和性 | ✅ UPDATE と一致 | ⚠️ 概念のズレ |
| 関数型スタイル | ❌ 手続き型 | ✅ 関数型 |

## まとめ

- **可変アプローチ**: CRUD ベースの通常のアプリケーションに適している
- **不変アプローチ**: イベントソーシングや並行処理が多い場合に適している
- **ハイブリッド**: 両方のメソッドを提供し、状況に応じて使い分ける

**RingiFlow では Phase 1 から不変アプローチを採用**し、Phase 4（イベントソーシング）への移行をスムーズにする。

---

関連ドキュメント:
- [DDD エンティティパターン](./DDD_エンティティパターン.md)
- [EventSourcing とデータ削除](./EventSourcingとデータ削除.md)
- [実装ロードマップ](../../03_詳細設計書/00_実装ロードマップ.md)
