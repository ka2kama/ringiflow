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

    // フィクスチャでセットアップを共通化（英語 snake_case）
    #[fixture]
    fn active_user() -> User {
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
    fn test_削除されたユーザーはログインできない(active_user: User) {
        let deleted = active_user.deleted();

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
fn test_削除されたユーザーはログインできない(active_user: User) {
    let deleted = active_user.deleted();
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

## 推奨クレート

以下のクレートは workspace に追加済み。ボイラープレートコードを避けるため、積極的に活用する。

### derive_more

トレイト実装の自動生成。手動 `impl` より derive を優先する。

| derive | 用途 | 手動実装を避ける |
|--------|------|-----------------|
| `Display` | 表示形式 | `impl Display for ...` |
| `From` | 型変換（単純なラッパー） | `impl From<T> for ...` |
| `Constructor` | `new()` メソッド生成 | 単純な `fn new(...) -> Self` |

```rust
use derive_more::Display;

// Good: derive で自動生成（Newtype パターンでは #[display] 属性が必要）
#[derive(Display)]
#[display("{_0}")]
pub struct UserId(Uuid);

// Bad: 手動実装
impl Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
```

### strum

enum と文字列の相互変換。手動 `match` より derive を優先する。

| derive | 用途 | 手動実装を避ける |
|--------|------|-----------------|
| `IntoStaticStr` | `enum.into() -> &'static str` | `fn as_str(&self) -> &str { match ... }` |
| `Display` | `enum.to_string()` | `impl Display for ...`（strum の Display を使う） |

```rust
use strum::{IntoStaticStr, Display};

// Good: strum で自動生成
#[derive(IntoStaticStr, Display)]
#[strum(serialize_all = "lowercase")]
pub enum UserStatus {
    Active,
    Inactive,
    Deleted,
}

// 使い方
let status = UserStatus::Active;
let status_str: &str = status.into();            // IntoStaticStr（型アノテーション必須）
assert_eq!(status_str, "active");
assert_eq!(status.to_string(), "active");        // Display
```

注意:
- `IntoStaticStr` は `Into<&'static str>` を実装する。型推論が効かない場合は明示的な型アノテーションが必要
- `EnumString` はエラー型が `strum::ParseError` になるため、`DomainError` を返したい場合は `FromStr` を手動実装する
- 旧来の `as_str()` メソッドは `into()` に置き換える（型アノテーション付き）

### itertools

イテレータ操作の拡張。標準ライブラリで冗長になるパターンに活用する。

| メソッド | 用途 | 代替パターン |
|---------|------|-------------|
| `unique()` | 重複排除 | `HashSet` 経由で collect |
| `sorted()` | ソート（新しい Vec を返す） | `collect()` 後に `sort()` |
| `collect_vec()` | `Vec` への collect | `.collect::<Vec<_>>()` |

```rust
use itertools::Itertools;

// Good: itertools で簡潔に
let unique_ids: Vec<_> = ids.into_iter().unique().collect();

// Bad: HashSet 経由で冗長
let unique_ids: Vec<_> = ids.into_iter().collect::<HashSet<_>>().into_iter().collect();
```

### maplit

コレクションリテラルのマクロ。テストや設定初期化で活用する。

```rust
use maplit::{hashmap, hashset};

// Good: マクロで簡潔に
let permissions = hashset! { "read", "write" };
let config = hashmap! {
    "host" => "localhost",
    "port" => "8080",
};

// Bad: 手動で insert
let mut permissions = HashSet::new();
permissions.insert("read");
permissions.insert("write");
```

### 使用しない場面

以下の場合は手動実装を維持する:

- `From` 実装に複雑なロジックがある場合（メソッド呼び出し、条件分岐など）
- カスタムフォーマットが必要な `Display` 実装
- `FromStr` でカスタムエラーメッセージが必要な場合

## ドキュメントコメント

モジュールレベル（`//!`）のコメントには以下を含める:

1. モジュールの目的
2. 主要な型・関数の説明
3. 使用例（doctests）
4. 詳細ドキュメントへのリンク

### doctest でのエラーハンドリング

doctest では `unwrap()` ではなく `?` 演算子を使う。`fn main()` を `Result` 返り値にし、ボイラープレートは `#` で隠す:

```rust
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let email = Email::new("user@example.com")?;
//! # Ok(())
//! # }
//! ```
```

| コンテキスト | `unwrap()` | `?` 演算子 |
|-------------|-----------|-----------|
| doctest（`//!`, `///`） | 使わない | `fn main() -> Result` ラッパーで使用 |
| テストコード（`#[cfg(test)]`） | 許容 | どちらでもよい |

### ドキュメントコメントの構成例

```rust
//! # ユーザー管理
//!
//! ユーザーエンティティと関連する値オブジェクトを定義する。
//!
//! ## 使用例
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use ringiflow_domain::user::{User, Email};
//!
//! let email = Email::new("user@example.com")?;
//! let user = User::new(tenant_id, email, "山田太郎".to_string(), None);
//! # Ok(())
//! # }
//! ```
//!
//! 詳細: [ユーザー管理設計](../../../docs/01_要件定義書/機能仕様書/02_ユーザー管理.md)
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
   - バージョン指定ルール:
     - 1.x 以上: メジャーのみ（`"1"`）— semver 準拠が期待できる
     - 0.x 系: マイナーまで（`"0.14"`）— 破壊的変更がありうるため
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
   - 詳細: [sqlx-cli](../../docs/06_ナレッジベース/rust/sqlx-cli.md#オフラインモード詳細)

禁止事項:
- `unwrap()` / `expect()` の無思慮な使用
- public フィールドによる不変条件の破壊
- テストのないビジネスロジック
- パラメータ化されていない SQL クエリ

## 参照

- プロジェクト理念: [CLAUDE.md](../../CLAUDE.md)
- 最新プラクティス方針: [latest-practices.md](latest-practices.md)
- 実装ガイドライン: [docs/03_詳細設計書/05_実装ガイドライン.md](../../docs/03_詳細設計書/05_実装ガイドライン.md)
