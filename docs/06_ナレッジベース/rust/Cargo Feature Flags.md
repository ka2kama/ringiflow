# Cargo Feature Flags

## 概要

Cargo の feature flag は条件付きコンパイルの仕組みで、クレートの機能をオプションにできる。`#[cfg(feature = "...")]` と組み合わせて、ビルド時にコードの有効/無効を制御する。

## 2つのパターン

feature flag には性質の異なる2つの使い方がある。

### パターン1: ランタイム機能トグル

開発環境と本番環境でアプリケーションの挙動を切り替える。

```toml
[features]
default = ["dev-auth"]  # デフォルトで有効
dev-auth = []
```

```rust
#[cfg(feature = "dev-auth")]
fn bypass_auth() { /* 開発用認証バイパス */ }
```

特徴:
- `default` に含めることで通常は有効
- 本番では `--no-default-features` で明示的に除外する
- **制御主体はビルド構成**（Dockerfile、CI 設定など）
- 誤って有効にするとセキュリティリスクになり得る

### パターン2: テスト用コードのビルドゲート

テスト専用の型や関数をリリースビルドから除外する。

```toml
[features]
test-support = []  # デフォルトで無効
```

```rust
#[cfg(any(test, feature = "test-support"))]
pub struct FixedClock { /* テスト用の固定時刻 */ }
```

```toml
# 利用側クレートの Cargo.toml
[dependencies]
my-crate.workspace = true

[dev-dependencies]
my-crate = { workspace = true, features = ["test-support"] }
```

特徴:
- `default` に含めない（リリースから自動除外）
- `dev-dependencies` で features を指定すると、テスト時のみ有効化される
- **制御主体は Cargo の依存解決**（自動）
- 誤って無効にするとテストがコンパイルエラーになる（即座に気づく）

## `#[cfg(test)]` のクレート境界

`#[cfg(test)]` はそのクレート自身のテスト時にのみ有効になる。依存先クレートのテスト時には有効にならない。

```
cargo test -p core-service
├── core-service: cfg(test) = true   ← テスト対象
└── domain:       cfg(test) = false  ← 依存として通常コンパイル
```

このため、クロスクレートでテスト用コードを共有するには `#[cfg(any(test, feature = "test-support"))]` パターンが必要。`test` は同一クレートのテスト、`feature` は外部クレートのテストをカバーする。

## パターン比較

| 観点 | ランタイム機能トグル | テスト用ビルドゲート |
|------|---------------------|---------------------|
| default | 有効 | 無効 |
| リリースビルド | 明示除外が必要 | 自動除外 |
| 制御主体 | ビルド構成 | Cargo 依存解決 |
| 誤設定リスク | セキュリティリスク | コンパイルエラー（安全） |
| 例（エコシステム） | `serde/derive`、`tokio/full` | `tokio/test-util`、`sqlx/runtime-tokio` |

## プロジェクトでの使用箇所

| feature | クレート | パターン | 用途 |
|---------|---------|---------|------|
| `dev-auth` | `bff` | ランタイム機能トグル | 開発時の認証バイパス |
| `test-support` | `domain` | テスト用ビルドゲート | `FixedClock` の条件付きコンパイル |

2つのパターンが混在しているのは、用途が本質的に異なるため。統一する必要はない。

## 関連リソース

- [The Cargo Book: Features](https://doc.rust-lang.org/cargo/reference/features.html)
- [セッションログ: FixedClock リリース除外](../../../prompts/runs/2026-02/2026-02-09_2106_FixedClockリリース除外.md)
