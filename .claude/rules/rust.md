---
paths:
  - "**/*.rs"
  - "**/Cargo.toml"
---

# Rust 実装ルール

このルールは Rust ファイル（`*.rs`、`Cargo.toml`）を編集する際に適用される。

## 品質基準（CLAUDE.md 理念2より）

### 型システムの活用

- Newtype パターン: ID 型は UUID をラップし、型安全性を確保（例: `UserId(Uuid)`）
- 不正な状態を表現不可能に: 型で制約を表現し、コンパイル時に検証
- Result 型の徹底: エラーは `Result` で明示的に扱う。`unwrap()`/`expect()` の濫用を避ける
  - `unwrap()` はプログラムのバグを示す場合のみ使用可能（例: 静的な文字列のパース）
  - それ以外は `?` 演算子または `match` で適切にハンドリング

### エラーハンドリング

| 状況 | 推奨される方法 |
|------|--------------|
| ドメイン層 | `DomainError` を使用 |
| インフラ層 | `anyhow::Result` または独自のエラー型 |
| API 層 | ドメイン/インフラのエラーを HTTP ステータスに変換 |

```rust
// Good: Result で明示的に
pub fn validate_email(email: &str) -> Result<Email, DomainError> {
    if email.is_empty() {
        return Err(DomainError::Validation("メールアドレスは必須です".to_string()));
    }
    Ok(Email(email.to_string()))
}

// Bad: パニックする
pub fn validate_email(email: &str) -> Email {
    assert!(!email.is_empty(), "メールアドレスは必須です");
    Email(email.to_string())
}
```

### コードの明確さ

- 意図が伝わる命名: 略語を避け、明確な名前を付ける
- コメントは「なぜ」を書く: 「何を」はコードで表現し、コメントは設計判断や理由を記述
- 過度な抽象化を避ける: 3回繰り返すまでは重複を許容（Rule of Three）

## テスト要件

### 必須テスト

以下のケースではテストが必須:

1. **ドメインロジック**: すべてのビジネスルールに対してテスト
2. **バリデーション**: 正常系・異常系の両方
3. **状態遷移**: エンティティのステータス変更ロジック
4. **エッジケース**: 境界値、NULL、空文字列など

### テストの書き方

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    // フィクスチャでセットアップを共通化
    #[fixture]
    fn アクティブなユーザー() -> User {
        let tenant_id = TenantId::new();
        let email = Email::new("user@example.com").unwrap();
        User::new(tenant_id, email, "Test User".to_string(), None)
    }

    // 単純なテスト
    #[test]
    fn test_メールアドレスは正常な形式を受け入れる() {
        assert!(Email::new("user@example.com").is_ok());
    }

    // パラメータ化テスト（バリデーション等）
    #[rstest]
    #[case("", "空文字列")]
    #[case("no-at-sign", "@記号なし")]
    #[case(&format!("{}@example.com", "a".repeat(256)), "255文字超過")]
    fn test_メールアドレスは不正な形式を拒否する(#[case] input: &str, #[case] _reason: &str) {
        assert!(Email::new(input).is_err());
    }

    // フィクスチャを使用したテスト
    #[rstest]
    fn test_削除されたユーザーはログインできない(アクティブなユーザー: User) {
        let deleted = アクティブなユーザー.deleted();

        assert!(!deleted.can_login());
    }
}
```

### テストのベストプラクティス

- `pretty_assertions::assert_eq` を使用（差分が見やすい）
- `rstest` でフィクスチャとパラメータ化テストを活用
- テスト名は日本語、`test_` プレフィクス必須（数字始まりに対応）
- 空行でセクション分割（Arrange/Act/Assert の代わり）

### テストの粒度

原則: 1テスト1検証観点

各テストは単一の振る舞いまたは条件を検証する。

```rust
// Good: 単一の検証観点
#[rstest]
fn test_削除されたユーザーはログインできない(アクティブなユーザー: User) {
    let deleted = アクティブなユーザー.deleted();
    assert!(!deleted.can_login());
}

// Good: 同一の検証観点に対する複数のケース（パラメータ化）
#[rstest]
#[case("", "空文字列")]
#[case("no-at-sign", "@記号なし")]
fn test_メールアドレスは不正な形式を拒否する(#[case] input: &str, #[case] _reason: &str) {
    assert!(Email::new(input).is_err());
}

// Bad: 無関係な複数の検証（失敗時にどこが問題か不明）
#[test]
fn test_ユーザー操作() {
    let user = User::new(...);
    assert!(user.is_active());      // 作成時の状態
    let updated = user.with_status(UserStatus::Inactive);
    assert!(!updated.is_active());   // ステータス変更
    let deleted = updated.deleted();
    assert!(deleted.status() == UserStatus::Deleted);  // 削除
}
```

判断基準:
- テスト名で検証内容が明確に表現できるか
- テストが失敗したとき、何が壊れたか即座に分かるか
- セットアップコードの重複よりも、テストの明確さを優先

## コーディング規約

### モジュール構造

`mod.rs` は使用しない。Rust 2018 以降のディレクトリ構造を採用する。

```
# Good: 新しいスタイル
src/
├── lib.rs
├── repository.rs          # pub mod user_repository;
└── repository/
    └── user_repository.rs

# Bad: 古いスタイル（mod.rs）
src/
├── lib.rs
└── repository/
    ├── mod.rs             # ← 使わない
    └── user_repository.rs
```

### 構造体とメソッド

```rust
// Good: 不変性を保ち、メソッド経由で変更
pub struct User {
    id: UserId,           // private
    name: String,         // private
    status: UserStatus,   // private
}

impl User {
    // Getter は参照を返す
    pub fn id(&self) -> &UserId {
        &self.id
    }

    // 状態変更はメソッド経由
    pub fn activate(&mut self) {
        self.status = UserStatus::Active;
    }
}

// Bad: public フィールド
pub struct User {
    pub id: UserId,     // 外部から直接変更可能
    pub name: String,
}
```

### エラーメッセージ

- 日本語でユーザーフレンドリーに
- 具体的な情報を含める（何が問題か、どうすべきか）
- エラーの context を提供

```rust
// Good
Err(DomainError::Validation(
    "メールアドレスは255文字以内である必要があります".to_string()
))

// Bad
Err(DomainError::Validation("Invalid email".to_string()))
```

## セキュリティ

- **入力値の検証**: すべての外部入力をドメイン層で検証
- **SQL インジェクション**: SQLx のパラメータ化クエリを使用
- **認可チェック**: 操作前に必ず権限を確認
- **センシティブ情報**: パスワード等はハッシュ化、ログに出力しない

```rust
// Good: パラメータ化クエリ
sqlx::query!(
    "SELECT * FROM users WHERE id = $1",
    user_id.as_uuid()
)

// Bad: 文字列結合
sqlx::query(&format!("SELECT * FROM users WHERE id = '{}'", user_id))
```

## パフォーマンス

- **不要なクローンを避ける**: 参照を活用
- **適切な型を選択**: `String` vs `&str`、`Vec` vs `&[T]`
- **N+1 クエリを避ける**: JOIN や IN 句で一括取得

```rust
// Good: 参照を受け取る
pub fn validate(&self, user: &User) -> Result<(), DomainError> {
    // ...
}

// Bad: 所有権を奪う（不要な clone を強制）
pub fn validate(&self, user: User) -> Result<(), DomainError> {
    // ...
}
```

## ドキュメントコメント

モジュールレベル（`//!`）のコメントには以下を含める:

1. モジュールの目的
2. 主要な型・関数の説明
3. 使用例（doctests）
4. 詳細ドキュメントへのリンク

```rust
//! # ユーザー管理
//!
//! ユーザーエンティティと関連する値オブジェクトを定義する。
//!
//! ## 使用例
//!
//! ```rust
//! use ringiflow_domain::user::{User, Email};
//!
//! let email = Email::new("user@example.com")?;
//! let user = User::new(tenant_id, email, "山田太郎".to_string(), None);
//! ```
//!
//! 詳細: [ユーザー管理設計](../../../docs/03_詳細設計書/ユーザー管理.md)
```

## AI エージェントへの指示

Rust コードを実装する際:

1. 型で制約を表現できないか常に検討する
2. エラーハンドリングを必ず実装する（`unwrap()` を避ける）
3. テストを必ず書く（最低でも正常系・異常系）
4. セキュリティリスクを考慮する（SQL インジェクション、認可等）
5. ビルドとテストを実行して動作を確認する
6. 依存関係を追加する際は workspace.dependencies で統一管理する
   - 手順:
     1. workspace の `Cargo.toml` の `[workspace.dependencies]` に追加
     2. 使用するクレートの `Cargo.toml` で `<crate>.workspace = true` と参照
   - バージョンの確認方法:
     - `cargo search <crate>` で最新のstableバージョンを確認
     - または crates.io で確認
   - `cargo add --package <pkg> <crate>` は直接依存を追加するため使用しない
7. SQL クエリを追加・変更したら SQLx オフラインキャッシュを更新する
   ```bash
   just setup-db  # DB を起動
   cd backend && cargo sqlx prepare --workspace -- --all-targets
   git add backend/.sqlx/
   ```
   - `--all-targets`: テストコード内のクエリもキャッシュに含める
   - CI 環境では `SQLX_OFFLINE=true` でビルドするため、キャッシュがないとビルドが失敗する
   - 詳細: [sqlx-cli 技術ノート](../../docs/06_技術ノート/sqlx-cli.md#オフラインモード詳細)

禁止事項:
- `unwrap()` / `expect()` の無思慮な使用
- public フィールドによる不変条件の破壊
- テストのないビジネスロジック
- パラメータ化されていない SQL クエリ

## 参照

- プロジェクト理念: [CLAUDE.md](../../CLAUDE.md)
- 実装ガイドライン: [docs/03_詳細設計書/05_実装ガイドライン.md](../../docs/03_詳細設計書/05_実装ガイドライン.md)
