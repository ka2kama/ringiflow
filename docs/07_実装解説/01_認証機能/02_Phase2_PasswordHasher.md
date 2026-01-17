# Phase 2: PasswordChecker

## 概要

パスワード検証を担当する `PasswordChecker` トレイトと Argon2id 実装を追加した。
開発用シードデータのパスワードハッシュも更新した。

### 対応 Issue

[#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34) - Phase 2

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [実装コンポーネント > 実装順序](../../03_詳細設計書/07_認証機能設計.md#実装順序) | Phase 2 の位置づけ |
| [インターフェース定義 > PasswordHasher](../../03_詳細設計書/07_認証機能設計.md#passwordhasher) | トレイト定義 |
| [パスワード管理](../../03_詳細設計書/07_認証機能設計.md#パスワード管理) | アルゴリズム選定 |
| [テスト計画 > 単体テスト](../../03_詳細設計書/07_認証機能設計.md#単体テスト) | テストケース |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/domain/src/password.rs`](../../../backend/crates/domain/src/password.rs) | PlainPassword, PasswordHash, PasswordVerifyResult |
| [`backend/crates/infra/src/password.rs`](../../../backend/crates/infra/src/password.rs) | PasswordChecker トレイト + Argon2 実装 |
| [`backend/migrations/20260115000009_update_seed_password_hash.sql`](../../../backend/migrations/20260115000009_update_seed_password_hash.sql) | シードデータのパスワード更新 |

---

## 実装内容

### ドメイン層の型（[`domain/password.rs`](../../../backend/crates/domain/src/password.rs)）

```rust
/// 平文パスワード（ログイン時の入力値）
pub struct PlainPassword(String);

/// ハッシュ化されたパスワード
pub struct PasswordHash(String);

/// 検証結果
pub enum PasswordVerifyResult {
    Match,
    Mismatch,
}
```

`PlainPassword` はログイン時のユーザー入力を受け取る。バリデーションは行わない。
Debug 出力では値をマスク（`[REDACTED]`）してセキュリティを確保。

### PasswordChecker トレイト（[`infra/password.rs`](../../../backend/crates/infra/src/password.rs)）

```rust
pub trait PasswordChecker: Send + Sync {
    /// パスワードを検証する
    fn verify(
        &self,
        password: &PlainPassword,
        hash: &PasswordHash,
    ) -> Result<PasswordVerifyResult, InfraError>;
}
```

### Argon2PasswordChecker

OWASP 推奨パラメータ（RFC 9106）を使用した Argon2id 実装。

**パラメータ:**

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| Memory | 65536 KB (64 MB) | メモリコスト |
| Iterations | 1 | 時間コスト |
| Parallelism | 1 | 並列度 |

**主要メソッド:**

```rust
Argon2PasswordChecker::new() -> Self

let password = PlainPassword::new("password123");
let hash = PasswordHash::new("$argon2id$...");
checker.verify(&password, &hash)  // -> Ok(PasswordVerifyResult::Match/Mismatch)
```

### シードデータ更新

開発用ユーザー（admin@example.com, user@example.com）のパスワードを `password123` に設定。

---

## テスト

### ドメイン層（PlainPassword）

| テスト | 検証内容 |
|-------|---------|
| `test_平文パスワードを作成できる` | 任意の文字列でインスタンス作成可能 |
| `test_平文パスワードのDebug出力はマスクされる` | `[REDACTED]` でマスク |

### インフラ層（Argon2PasswordChecker）

| テスト | 検証内容 |
|-------|---------|
| `test_正しいパスワードを検証できる` | 正しいパスワードで Match |
| `test_不正なパスワードを検証できる` | 不正なパスワードで Mismatch |
| `test_不正なハッシュ形式はエラー` | 不正なハッシュ形式はエラー |

### テスト実行

```bash
cd backend && cargo test -p ringiflow-domain password
cd backend && cargo test -p ringiflow-infra password
```

---

## 関連ドキュメント

- 設計書: [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md)
- 技術ノート: [パスワードハッシュ.md](../../06_技術ノート/パスワードハッシュ.md)

---

# 設計解説

以下は設計判断の詳細解説。レビュー・学習用。

---

## 1. なぜ hash メソッドを実装しないのか（YAGNI）

**判断:**

#34（ログイン/ログアウト）では `verify` のみ必要。`hash` は不要。

**理由:**

1. **ログインでは既存ハッシュと照合するだけ** - 新しいハッシュを生成する必要がない
2. **ユーザー作成・パスワード変更は別機能** - その時点で `hash` を実装すればよい
3. **YAGNI（You Aren't Gonna Need It）** - 必要になるまで作らない

**将来の拡張:**

ユーザー作成機能を実装する際に追加する予定:
- `NewPassword` 型（パスワードポリシーでバリデーション）
- `hash(&self, password: &NewPassword) -> Result<PasswordHash, InfraError>` メソッド

---

## 2. PlainPassword にバリデーションがない理由

**判断:**

`PlainPassword` はログイン時の入力値であり、バリデーションを行わない。

**理由:**

1. **ログイン時の役割**: ユーザーが入力したパスワードを受け取り、既存ハッシュと照合する
2. **バリデーションの不要性**: 不正なパスワードは照合で `Mismatch` になるだけ
3. **型の分離**: パスワード作成/更新時は別の型（`NewPassword`）でポリシーを適用する

**PlainPassword vs NewPassword（将来）:**

| 型 | 用途 | バリデーション |
|----|------|--------------|
| `PlainPassword` | ログイン時の入力 | なし |
| `NewPassword`（将来） | パスワード作成/更新 | ポリシー適用 |

---

## 3. Argon2id の選定理由

**場所:** [`password.rs:31-54`](../../../backend/crates/infra/src/password.rs#L31-L54)

**なぜ Argon2id か:**

| アルゴリズム | 特徴 | 評価 |
|-------------|------|------|
| bcrypt | 歴史あり、CPU バウンド | GPU 攻撃に弱い |
| scrypt | メモリハード | パラメータ調整が難しい |
| Argon2id | メモリハード + GPU/サイドチャネル両対策 | OWASP 推奨 |

Argon2id は Argon2d（GPU 攻撃に強い）と Argon2i（サイドチャネル攻撃に強い）のハイブリッド。
汎用的なパスワードハッシュに最適。

---

## 4. パラメータの意味

**場所:** [`password.rs:43-49`](../../../backend/crates/infra/src/password.rs#L43-L49)

```rust
let params = Params::new(
    65536, // memory (KB) = 64 MB
    1,     // iterations
    1,     // parallelism
    None,  // output length (default: 32)
)
```

**なぜこの値か:**

1. **Memory: 64 MB** - RFC 9106 推奨値。攻撃者が GPU で並列にクラックする際のコストを上げる
2. **Iterations: 1** - Argon2 はメモリハードなので、反復回数を増やす必要性が低い
3. **Parallelism: 1** - サーバー負荷を抑える初期設定

---

## 5. verify の戻り値設計

**場所:** [`password.rs:64-78`](../../../backend/crates/infra/src/password.rs#L64-L78)

```rust
fn verify(&self, password: &PlainPassword, hash: &PasswordHash) -> Result<PasswordVerifyResult, InfraError> {
    let parsed = Argon2PasswordHash::new(hash.as_str())
        .map_err(|e| InfraError::Unexpected(format!("不正なハッシュ形式: {e}")))?;

    let matched = self.argon2.verify_password(password.as_str().as_bytes(), &parsed).is_ok();
    Ok(PasswordVerifyResult::from(matched))
}
```

**`Result<PasswordVerifyResult, ...>` を選んだ理由:**

| 戻り値 | 「一致」| 「不一致」| 「ハッシュ形式不正」|
|--------|--------|----------|------------------|
| `bool` | `true` | `false` | panic? 無視? |
| `Result<bool, ...>` | `Ok(true)` | `Ok(false)` | `Err(...)` |
| `Result<PasswordVerifyResult, ...>`（採用） | `Ok(Match)` | `Ok(Mismatch)` | `Err(...)` |

- `bool` より意図が明確（`is_match()` で確認）
- パスワード不一致は「正常な処理結果」なので `Ok(Mismatch)`
- ハッシュ形式不正は「異常な状態」なので `Err(...)`

---

## 6. トレイト名を PasswordChecker にした理由

**背景:**

当初は `PasswordHasher` という名前で `hash` と `verify` の両方を持っていた。
`hash` メソッドを削除した際、「Hasher」という名前が不適切になったため改名を検討。

`PasswordVerifier` が自然な名前だが、argon2 クレートに同名のトレイトが存在。

```rust
use argon2::PasswordVerifier;  // argon2 クレートのトレイト
```

**解決策:**

`PasswordChecker` に改名して名前衝突を回避。

```rust
use argon2::PasswordVerifier as _;  // スコープに持ち込むが名前は使わない

pub trait PasswordChecker: Send + Sync { ... }
```
