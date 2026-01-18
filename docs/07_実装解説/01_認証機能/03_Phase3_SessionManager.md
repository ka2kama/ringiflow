# Phase 3: SessionManager

## 概要

Redis を使用したセッション管理機能を実装した。
`SessionManager` トレイトでセッションの作成・取得・削除を行い、
`RedisSessionManager` で Redis を使った具体実装を提供する。

### 対応 Issue

[#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34) - Phase 3

---

## 設計書との対応

本 Phase は以下の設計書セクションに対応する。

| 設計書セクション | 対応内容 |
|----------------|---------|
| [実装コンポーネント > 実装順序](../../03_詳細設計書/07_認証機能設計.md#実装順序) | Phase 3 の位置づけ |
| [インターフェース定義 > SessionManager](../../03_詳細設計書/07_認証機能設計.md#sessionmanager) | トレイト定義 |
| [セッション管理](../../03_詳細設計書/07_認証機能設計.md#セッション管理) | データ構造、Redis キー設計 |
| [テスト計画 > 統合テスト](../../03_詳細設計書/07_認証機能設計.md#統合テスト) | テストケース |

---

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/infra/src/session.rs`](../../../backend/crates/infra/src/session.rs) | SessionData, SessionManager トレイト, RedisSessionManager |
| [`backend/crates/infra/tests/session_test.rs`](../../../backend/crates/infra/tests/session_test.rs) | 統合テスト |

---

## 実装内容

### SessionData（[`session.rs:34-95`](../../../backend/crates/infra/src/session.rs#L34-L95)）

```rust
pub struct SessionData {
    user_id: UserId,
    tenant_id: TenantId,
    email: String,
    name: String,
    roles: Vec<String>,
    created_at: DateTime<Utc>,
    last_accessed_at: DateTime<Utc>,
}
```

Redis に JSON 形式で保存されるセッション情報。
ログイン成功時に作成され、ログアウトまたは TTL 経過で削除される。

### SessionManager トレイト（[`session.rs:101-142`](../../../backend/crates/infra/src/session.rs#L101-L142)）

```rust
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// セッションを作成し、セッション ID を返す
    async fn create(&self, data: &SessionData) -> Result<String, InfraError>;

    /// セッションを取得する
    async fn get(&self, tenant_id: &TenantId, session_id: &str) -> Result<Option<SessionData>, InfraError>;

    /// セッションを削除する
    async fn delete(&self, tenant_id: &TenantId, session_id: &str) -> Result<(), InfraError>;

    /// テナントの全セッションを削除する（テナント退会時）
    async fn delete_all_for_tenant(&self, tenant_id: &TenantId) -> Result<(), InfraError>;
}
```

### RedisSessionManager

Redis を使用したセッションマネージャの実装。

**主要メソッド:**

```rust
// 接続
RedisSessionManager::new(redis_url: &str) -> Result<Self, InfraError>

// セッション操作
manager.create(&session_data).await  // -> Ok(session_id)
manager.get(&tenant_id, &session_id).await  // -> Ok(Some(SessionData))
manager.delete(&tenant_id, &session_id).await  // -> Ok(())
manager.delete_all_for_tenant(&tenant_id).await  // -> Ok(())
```

**Redis キー設計:**

| キー | 値 | TTL |
|-----|-----|-----|
| `session:{tenant_id}:{session_id}` | SessionData (JSON) | 28800秒（8時間） |

---

## テスト

### 統合テスト（session_test.rs）

| テスト | 検証内容 |
|-------|---------|
| `test_セッションを作成できる` | create でセッション ID が返る |
| `test_セッションを取得できる` | get でセッションデータを取得 |
| `test_存在しないセッションはnoneを返す` | 存在しない ID は None |
| `test_セッションを削除できる` | delete が成功 |
| `test_削除後のセッションはnoneを返す` | 削除後は None |
| `test_テナント単位で全セッションを削除できる` | delete_all_for_tenant で一括削除 |
| `test_別テナントのセッションは削除されない` | テナント分離 |
| `test_セッションに有効期限が設定される` | TTL が 28800 秒以下 |
| `test_created_atとlast_accessed_atが設定される` | タイムスタンプの検証 |

### テスト実行

```bash
just dev-deps  # Redis 起動
cd backend && cargo test -p ringiflow-infra --test session_test
```

---

## 関連ドキュメント

- 設計書: [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md)

---

# 設計解説

以下は設計判断の詳細解説。レビュー・学習用。

---

## 1. セッション ID に UUID v4 を使用する理由

**場所:** [`session.rs:175-176`](../../../backend/crates/infra/src/session.rs#L175-L176)

```rust
let session_id = Uuid::new_v4().to_string();
```

**なぜ UUID v4 か:**

| 方式 | 特徴 | セキュリティ |
|------|------|------------|
| UUID v4 | 暗号論的ランダム | 推測困難 |
| UUID v7 | タイムスタンプ + ランダム | 生成時刻が推測可能 |
| 連番 | 単純 | 推測容易 |

セッション ID は外部に露出するため、推測困難であることが重要。
UUID v4 は 122 ビットのランダム値を持ち、暗号論的に安全。

**UUID v7 を使わない理由:**

UUID v7 はデータベースの主キーには適している（ソート可能、インデックス効率）が、
セッション ID には不要。むしろタイムスタンプ部分から生成時刻が推測できるのは
セキュリティ上好ましくない。

---

## 2. SCAN vs KEYS の選択

**場所:** [`session.rs:218-237`](../../../backend/crates/infra/src/session.rs#L218-L237)

```rust
// SCAN でパターンにマッチするキーを取得して削除
let mut cursor = 0u64;
loop {
    let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
        .arg(cursor)
        .arg("MATCH")
        .arg(&pattern)
        .arg("COUNT")
        .arg(100)
        .query_async(&mut conn)
        .await?;
    // ...
}
```

**なぜ KEYS ではなく SCAN か:**

| コマンド | 動作 | 本番環境での使用 |
|---------|------|----------------|
| KEYS | 全キーを一度に取得 | 危険（ブロッキング） |
| SCAN | カーソルで分割取得 | 安全（ノンブロッキング） |

`KEYS` は Redis 全体をブロックするため、本番環境では使用禁止。
`SCAN` はカーソルベースで少しずつ取得するため、他のリクエストをブロックしない。

**COUNT パラメータ:**

`COUNT 100` はヒントであり、正確に 100 件返すわけではない。
Redis はこの値を参考に、1 回のイテレーションで返すキー数を調整する。

---

## 3. get_ttl メソッドの追加（テスト用）

**場所:** [`session.rs:136-141`](../../../backend/crates/infra/src/session.rs#L136-L141)

```rust
/// セッションの TTL（残り秒数）を取得する（テスト用）
async fn get_ttl(
    &self,
    tenant_id: &TenantId,
    session_id: &str,
) -> Result<Option<i64>, InfraError>;
```

**なぜトレイトに含めたか:**

本来 `get_ttl` は実装の詳細であり、トレイトに含めるべきではない。
しかし、TTL が正しく設定されていることをテストで検証する必要があった。

**代替案:**

1. Redis クライアントを直接使ってテスト内で TTL を確認
2. `#[cfg(test)]` で条件付きメソッドにする
3. テスト専用のヘルパー構造体を作る

今回は「テスト用」と明示した上でトレイトに含めた。
将来的にセッション延長機能などで TTL 確認が必要になる可能性もある。

---

## 4. ConnectionManager の Clone

**場所:** [`session.rs:180`](../../../backend/crates/infra/src/session.rs#L180)

```rust
let mut conn = self.conn.clone();
```

**なぜ Clone が必要か:**

`ConnectionManager` は `Clone` が安価に実装されている（内部は `Arc`）。
各メソッドで Clone することで、以下を実現:

1. **&self で呼び出し可能** - `&mut self` を要求しない
2. **並行リクエスト対応** - 複数スレッドから同時に呼び出せる
3. **自動再接続** - 接続が切れても自動で再接続

**トレードオフ:**

Clone のコストは低いが、ゼロではない。
大量のリクエストがある場合は接続プールの検討が必要。

---

## 5. Redis キー設計の意図

**キー形式:** `session:{tenant_id}:{session_id}`

```
session:01234567-89ab-cdef-0123-456789abcdef:fedcba98-7654-3210-fedc-ba9876543210
```

**なぜこの形式か:**

1. **プレフィックス `session:`** - 他の用途のキー（csrf, cache など）と区別
2. **tenant_id を含める** - テナント単位での一括削除が可能
3. **session_id を末尾に** - SCAN のパターンマッチ `session:{tenant_id}:*` が使える

**テナント退会時の削除:**

```rust
let pattern = format!("session:{}:*", tenant_id.as_uuid());
// SCAN でマッチするキーを取得して DEL
```

テナント ID を含めることで、テナント退会時に全セッションを効率的に削除できる。

---

## 6. SessionData をドメイン層に置かない理由

**場所:** [`session.rs:34-95`](../../../backend/crates/infra/src/session.rs#L34-L95)

`SessionData` は infra 層に定義した。

**なぜドメイン層ではないか:**

| 観点 | 説明 |
|------|------|
| ビジネスロジック | セッションデータには特にビジネスルールがない |
| 技術的関心事 | TTL、Redis キー設計は技術的詳細 |
| 依存関係 | serde, chrono などインフラ的な依存 |

「セッション」は認証のための技術的な概念であり、
ワークフローや承認といったドメインの関心事ではない。

**ドメイン層に移す場合:**

セッションに対するビジネスルール（例: ロールによるセッション時間制限）が
追加される場合は、ドメイン層への移動を検討する。
