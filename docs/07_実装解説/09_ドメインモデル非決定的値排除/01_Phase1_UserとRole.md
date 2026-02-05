# Phase 1: User と Role の非決定的値排除

## 概要

`User` と `Role` のコンストラクタ・状態遷移メソッドから `Utc::now()` / `Uuid::now_v7()` を排除し、呼び出し元から注入する形に変更した。

対応 Issue: [#222](https://github.com/ka2kama/ringiflow/issues/222)

## 設計書との対応

- [詳細設計書: ドメインモデル](../../../docs/03_詳細設計書/) — User, Role エンティティ

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`user.rs`](../../../backend/crates/domain/src/user.rs) | `User` エンティティ（コンストラクタ + 状態遷移に `id`, `now` 注入） |
| [`role.rs`](../../../backend/crates/domain/src/role.rs) | `Role`, `UserRole` エンティティ（コンストラクタに `id`, `now` 注入） |
| [`handler/auth.rs`](../../../backend/apps/core-service/src/handler/auth.rs) | テストコードの呼び出し元更新 |

## 実装内容

### User の変更

| メソッド | 追加パラメータ |
|---------|--------------|
| `User::new()` | `id: UserId`, `now: DateTime<Utc>` |
| `User::with_last_login_updated()` | `now: DateTime<Utc>` |
| `User::with_status()` | `now: DateTime<Utc>` |
| `User::deleted()` | `now: DateTime<Utc>` |

変更前:

```rust
pub fn new(
    tenant_id: TenantId,
    email: String,
    name: String,
    password_hash: String,
) -> Self {
    let now = Utc::now();
    Self {
        id: UserId::new(),
        // ...
        created_at: now,
        updated_at: now,
    }
}
```

変更後:

```rust
pub fn new(
    id: UserId,
    tenant_id: TenantId,
    email: String,
    name: String,
    password_hash: String,
    now: DateTime<Utc>,
) -> Self {
    Self {
        id,
        // ...
        created_at: now,
        updated_at: now,
    }
}
```

### Role の変更

| メソッド | 追加パラメータ |
|---------|--------------|
| `Role::new_system()` | `id: RoleId`, `now: DateTime<Utc>` |
| `Role::new_tenant()` | `id: RoleId`, `now: DateTime<Utc>` |
| `UserRole::new()` | `id: Uuid`, `now: DateTime<Utc>` |

### 追加したテスト

- `test_新規ユーザーのcreated_atとupdated_atは注入された値と一致する`
- `test_ロールのcreated_atは注入された値と一致する`

## テスト

```bash
cd backend && cargo test --package ringiflow-domain
cd backend && cargo test --package ringiflow-core-service
```

## 設計解説

### 1. Functional Core, Imperative Shell パターン

場所: [`user.rs:42-63`](../../../backend/crates/domain/src/user.rs)

なぜこの設計か:
- ドメインモデル（Functional Core）は副作用を持たず、入力に対して決定的な出力を返す
- タイムスタンプや ID の生成はユースケース層（Imperative Shell）が担当する
- テストでは固定値を注入でき、時刻に依存しない検証が可能

代替案:
- Clock トレイト注入: エンティティに `Clock` トレイトを持たせる方式。過度な抽象化であり、Rust のエンティティは通常トレイトオブジェクトを持たない
- テスト用 mock: `#[cfg(test)]` で `Utc::now()` を差し替える方式。グローバル状態の操作となり、並列テストで問題が起きる

### 2. テストでの固定タイムスタンプ

場所: [`user.rs` テスト](../../../backend/crates/domain/src/user.rs)（`now` フィクスチャ）

```rust
#[fixture]
fn now() -> DateTime<Utc> {
    DateTime::from_timestamp(1_700_000_000, 0).unwrap()
}
```

なぜこの設計か:
- Unix タイムスタンプで固定値を指定することで、テストが時刻に依存しない
- rstest の `#[fixture]` により、テスト関数の引数として自動注入される
- 同じ `now` を複数のフィクスチャ（`新規ユーザー`, `システムロール` 等）で共有できる

## 関連ドキュメント

- [ADR: 該当なし（リファクタリングのため新規 ADR 不要）]
- [ナレッジベース: Functional Core, Imperative Shell](https://blog.ploeh.dk/2020/03/02/impureim-sandwich/)
