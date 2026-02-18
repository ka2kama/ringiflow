# Phase 2: PasswordHasher 実装

## 概要

Issue #34（ユーザー認証）の Phase 2 として、パスワードハッシュ機能を TDD で実装した。

## 背景

認証機能の実装順序:
1. Phase 1: UserRepository ✅
2. **Phase 2: PasswordHasher** ← 今回
3. Phase 3: SessionManager
4. Phase 4: Core API（AuthUseCase）
5. Phase 5: BFF（認証ハンドラ）

## 変更内容

### 追加したファイル

| ファイル | 内容 |
|---------|------|
| `backend/crates/domain/src/password.rs` | RawPassword, PasswordHash, PasswordVerifyResult |
| `backend/crates/infra/src/password.rs` | PasswordHasher トレイト + Argon2PasswordHasher |
| `backend/migrations/20260115000009_update_seed_password_hash.sql` | シードデータのパスワード更新 |
| `docs/07_実装解説/01_認証機能/02_Phase2_PasswordHasher.md` | 実装解説ドキュメント |
| `docs/04_手順書/04_開発フロー/03_マイグレーション運用.md` | マイグレーション運用手順書 |

### 変更したファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/Cargo.toml` | workspace.dependencies に argon2, rand を追加 |
| `backend/crates/domain/src/lib.rs` | password モジュールを公開 |
| `backend/crates/infra/Cargo.toml` | argon2, rand の依存を追加 |
| `backend/crates/infra/src/lib.rs` | password モジュールを公開 |
| `.claude/rules/rust.md` | 依存関係管理のルールを追加 |
| `justfile` | reset-db タスクを追加 |
| `.claude/rules/data-store.md` | マイグレーション適用ルールを追加 |

## 技術詳細

### ドメイン層の型

| 型 | 責務 |
|----|------|
| `RawPassword` | 入力パスワード（バリデーション付き Newtype） |
| `PasswordHash` | ハッシュ化されたパスワード（Newtype） |
| `PasswordVerifyResult` | 検証結果（Match/Mismatch） |

`RawPassword` はパスワードポリシー（8〜128文字、英字+数字必須）をバリデーション。
Debug 出力では値をマスク（`[REDACTED]`）。

### Argon2id パラメータ

OWASP 推奨（RFC 9106）に従う:

| パラメータ | 値 |
|-----------|-----|
| Memory | 64 MB |
| Iterations | 1 |
| Parallelism | 1 |

### rand_core バージョン互換性

`argon2` 0.5 は `rand_core` 0.6 を使用。`rand` 0.9 は `rand_core` 0.9 を使用し互換性がない。
`rand = "0.8"` を使用することで解決。

### シードデータ

開発用ユーザー（admin@example.com, user@example.com）のパスワードを `password123` に設定。
パスワード要件（英字+数字）を満たす。

## テストケース

### ドメイン層（RawPassword）

| テスト | 検証内容 |
|-------|---------|
| `test_有効なパスワードを受け入れる` | 英字+数字、最大長 |
| `test_短すぎるパスワードを拒否する` | 8文字未満はエラー |
| `test_長すぎるパスワードを拒否する` | 128文字超はエラー |
| `test_英字なしパスワードを拒否する` | 数字のみはエラー |
| `test_数字なしパスワードを拒否する` | 英字のみはエラー |

### インフラ層（Argon2PasswordHasher）

| テスト | 検証内容 |
|-------|---------|
| `test_パスワードをハッシュ化できる` | Argon2id 形式のハッシュが生成される |
| `test_正しいパスワードを検証できる` | 正しいパスワードで Match |
| `test_不正なパスワードを検証できる` | 不正なパスワードで Mismatch |
| `test_不正なハッシュ形式はエラー` | 不正なハッシュ形式はエラー |
| `test_同じパスワードでも異なるハッシュになる` | ソルトにより異なるハッシュ |

## 学んだこと

### workspace.dependencies の統一管理

`cargo add` は直接依存を追加するため使用しない。
手順:
1. workspace の `Cargo.toml` の `[workspace.dependencies]` に追加
2. 使用するクレートの `Cargo.toml` で `<crate>.workspace = true` と参照

### YAGNI

当初 `for_testing()` メソッドを追加したが、使用箇所がなかったため削除。
「将来使うかもしれない」コードは書かない。

## 関連

- Issue: [#34](https://github.com/ka2kama/ringiflow/issues/34)
- 設計書: [07_認証機能設計.md](../../../docs/03_詳細設計書/07_認証機能設計.md)
- 技術ノート: [パスワードハッシュ.md](../../../docs/06_ナレッジベース/security/パスワードハッシュ.md)
- 実装解説: [02_認証機能_コード解説.md](../../../docs/07_実装解説/PR46_認証機能/02_認証機能_コード解説.md)
