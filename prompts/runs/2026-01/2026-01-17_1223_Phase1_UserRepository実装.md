# Phase 1: UserRepository 実装

## 概要

認証機能 Phase 1 として UserRepository を TDD で実装した。async-trait を採用し、開発ルールの整備も行った。

## 背景と目的

Issue #34（ユーザー認証）の実装計画に基づき、Phase 1: UserRepository から着手。設計書で定義されたインターフェースを実装し、統合テストで動作を確認する。

## 実施内容

### UserRepository の実装

1. **InfraError 定義** (`crates/infra/src/error.rs`)
   - Database, Redis, Serialization, Unexpected の4種類

2. **UserRepository トレイト** (`crates/infra/src/repository/user_repository.rs`)
   - `find_by_email`: テナント内でメールアドレス検索
   - `find_by_id`: ID で検索
   - `find_with_roles`: ユーザーとロールを JOIN で取得
   - `update_last_login`: 最終ログイン日時を更新

3. **PostgresUserRepository 実装**
   - sqlx の `query!` マクロでコンパイル時 SQL 検証
   - テナント分離を考慮したクエリ

4. **統合テスト**
   - `#[sqlx::test]` マクロでテストごとにトランザクション管理
   - 6つのテストケースすべてパス

### async-trait の導入

Native AFIT から async-trait に移行。

**理由:**
- `dyn Trait` によるオブジェクト安全性を確保
- DI やモック化が容易
- デファクトスタンダードで可読性が高い

### 開発ルールの整備

1. **Rust モジュール構造規約** (PR #49)
   - `mod.rs` を使わず、新しいスタイル（`repository.rs` + `repository/`）を採用

2. **Issue 進捗更新ルール** (PR #50)
   - Phase 完了時に Issue のチェックボックスを都度更新
   - CLAUDE.md と手順書に明記

## 成果物

### コミット

| コミット | 内容 |
|---------|------|
| `53240e8` | Phase 1: UserRepository を実装 #34 |

### 作成/更新ファイル

| ファイル | 種別 |
|---------|------|
| `backend/crates/infra/src/error.rs` | 新規 |
| `backend/crates/infra/src/repository.rs` | 新規 |
| `backend/crates/infra/src/repository/user_repository.rs` | 新規 |
| `backend/crates/infra/src/lib.rs` | 更新 |
| `backend/crates/infra/Cargo.toml` | 更新 |
| `backend/Cargo.toml` | 更新 |

### 関連 PR

| PR | 状態 | 内容 |
|----|------|------|
| #49 | auto-merge 待ち | Rust モジュール構造規約 |
| #50 | auto-merge 待ち | Issue 進捗更新ルール |

## 設計判断と実装解説

### なぜ async-trait を採用したか

| 方式 | メリット | デメリット |
|------|---------|-----------|
| Native AFIT | 依存なし、ゼロコスト | `dyn Trait` 不可 |
| async-trait | オブジェクト安全、可読性高 | Box 割り当て |

DI でモック化する可能性を考慮し、async-trait を採用。Rust エコシステムのデファクトスタンダードでもある。

### sqlx の選択

ORM（Diesel、SeaORM）ではなく sqlx を採用。

**理由:**
- SQL の知識がそのまま使える
- 複雑なクエリも自由に書ける
- コンパイル時 SQL 検証で型安全
- Aurora PostgreSQL との互換性が高い

## 議論の経緯

### モジュール構造の規約

ユーザーから、`mod.rs` は古い書き方なので使わないようにという指摘があった。Rust 2018 以降の新しいモジュール構造を採用し、ルールに明記した。

### async-trait の採用

async-trait を使った方がいいかという議論があった。デファクトスタンダードかつベストプラクティスとして async-trait を採用することに決定した。

### Issue 進捗管理

ユーザーから、Issue の進捗管理ができていないという指摘があり、都度更新すべきとの指導があった。Phase 完了時に Issue を更新するルールを CLAUDE.md と手順書に追加した。

### コミット分離の方針

ルールの追加だけでもコミットを分けるべきかという議論があった。関心事の分離のため、別 PR でマージする方針に決定した。

## 学んだこと

1. **async-trait vs Native AFIT**: オブジェクト安全性が必要なら async-trait
2. **モジュール構造**: `mod.rs` より `foo.rs` + `foo/` の方がモダン
3. **Issue 駆動開発**: 進捗の可視化は都度更新で担保する
4. **コミットの分離**: 異なる関心事は別 PR でマージ

## 次のステップ

- Phase 2: PasswordHasher の実装
- PR #49, #50 のマージ確認
