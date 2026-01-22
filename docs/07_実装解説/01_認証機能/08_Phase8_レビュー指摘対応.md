# Phase 8: レビュー指摘対応

## 概要

PR #46 の自動レビューで指摘された セキュリティ・本番環境対応を実施した。

### 対応 Issue

[#34 ユーザー認証（ログイン/ログアウト）](https://github.com/ka2kama/ringiflow/issues/34) - Phase 8

---

## 修正内容

| 指摘 | 重大度 | 対応 |
|------|--------|------|
| CSRF トークン検証がタイミング攻撃に脆弱 | Medium | 定数時間比較を導入 |
| Cookie Secure フラグが TODO のまま | Low | 環境変数で切り替え可能に |
| tenant_name TODO の計画が不明確 | Low | Issue 番号を明記 |

---

## 実装したコンポーネント

| ファイル | 変更内容 |
|---------|---------|
| [`backend/Cargo.toml`](../../../backend/Cargo.toml) | `subtle` crate を追加 |
| [`backend/apps/bff/Cargo.toml`](../../../backend/apps/bff/Cargo.toml) | `subtle` 依存を追加 |
| [`backend/apps/bff/src/middleware/csrf.rs`](../../../backend/apps/bff/src/middleware/csrf.rs) | 定数時間比較を使用 |
| [`backend/apps/bff/src/handler/auth.rs`](../../../backend/apps/bff/src/handler/auth.rs) | Secure フラグを環境変数化、TODO 明確化 |

---

## 関連ドキュメント

- 前 Phase: [07_Phase7_CSRFトークン.md](./07_Phase7_CSRFトークン.md)
- 設計書: [07_認証機能設計.md](../../03_詳細設計書/07_認証機能設計.md)

---

# 設計解説

以下は設計判断の詳細解説。レビュー・学習用。

---

## 1. CSRF トークンの定数時間比較

**場所:** [`csrf.rs:129-132`](../../../backend/apps/bff/src/middleware/csrf.rs#L129-L132)

```rust
let is_valid: bool = stored_token
   .as_bytes()
   .ct_eq(provided_token.as_bytes())
   .into();
if !is_valid {
   return csrf_error_response("CSRF トークンが無効です");
}
```

**なぜ定数時間比較が必要か:**

通常の `==` 演算子は、最初に不一致を見つけた時点で即座に `false` を返す。
攻撃者は応答時間を測定することで、トークンを1文字ずつ推測できる可能性がある。

| 比較対象 | 処理時間 | 攻撃者の情報 |
|---------|---------|-------------|
| "aaaa..." vs "baaa..." | 短い | 1文字目が不一致 |
| "aaaa..." vs "abaa..." | やや長い | 1文字目は一致 |
| "aaaa..." vs "aaaa..." | 最長 | 完全一致 |

定数時間比較は、入力の内容に関係なく常に同じ時間で処理を完了する。

**subtle crate を選んだ理由:**

| 方式 | メリット | デメリット |
|------|---------|-----------|
| `subtle::ConstantTimeEq` | 暗号ライブラリで広く使用、安定 | 依存追加 |
| 手動実装 | 依存なし | 最適化で定数時間にならない可能性 |
| `std::str::constant_time_compare` | 標準ライブラリ | Rust 1.82+ 限定 |

`subtle` crate は `ring` や `rustls` など多くの暗号ライブラリで使用されており、
信頼性が高い。

**実際の攻撃リスク:**

CSRF トークンは 64 文字の hex 文字列（256 ビット）。
タイミング攻撃で 1 文字ずつ推測しても、各文字は 16 通り（0-9, a-f）あるため、
64 × 16 = 1024 回のリクエストで推測可能。

ただし、実際にはネットワーク遅延のノイズがあるため、
正確な測定には大量のリクエストが必要。
それでもベストプラクティスとして定数時間比較を使用する。

---

## 2. Cookie Secure フラグの環境変数化

**場所:** [`auth.rs:444-457`](../../../backend/apps/bff/src/handler/auth.rs#L444-L457)

```rust
let is_production = std::env::var("ENV").unwrap_or_default() == "production";

let mut builder = Cookie::build((SESSION_COOKIE_NAME, session_id.to_string()))
   .path("/")
   .max_age(time::Duration::seconds(SESSION_MAX_AGE))
   .http_only(true)
   .same_site(SameSite::Lax);

if is_production {
   builder = builder.secure(true);
}

builder.build()
```

**なぜ環境変数で切り替えるか:**

| 環境 | プロトコル | Secure フラグ |
|------|-----------|--------------|
| ローカル開発 | HTTP | なし |
| 本番 | HTTPS | あり |

`Secure` フラグがあると、HTTPS でのみ Cookie が送信される。
ローカル開発では HTTP を使うことが多いため、フラグなしにする必要がある。

**代替案:**

```rust
// 設定ファイルに追加する方法
#[derive(Deserialize)]
struct CookieConfig {
    secure: bool,
}

// ビルド時に切り替える方法
#[cfg(not(debug_assertions))]
builder = builder.secure(true);
```

設定ファイル方式は柔軟だが、設定項目が増える。
`#[cfg]` 方式はデバッグビルド/リリースビルドに紐づくため、
ステージング環境での検証が難しい。

環境変数方式は、同じバイナリで異なる環境に対応できる。

---

## 3. TODO コメントに Issue 番号を明記

**場所:** [`auth.rs:131`](../../../backend/apps/bff/src/handler/auth.rs#L131)

```rust
// TODO(#34): Core Service にテナント情報取得エンドポイントを追加して取得
tenant_name: "Development Tenant".to_string(),
```

**なぜ Issue 番号を明記するか:**

TODO コメントは放置されやすい。Issue 番号を明記することで:

1. 追跡可能性: どの Issue で対応予定かが明確
2. 検索性: `git grep "TODO(#34)"` で関連 TODO を一括検索
3. 責任の明確化: Issue がクローズされたら TODO も解消すべき

**TODO 規約:**

```
TODO(#<Issue番号>): <やるべきこと>
```

IDE によっては TODO コメントを自動検出してリスト化する機能があり、
Issue 番号があると GitHub と連携しやすい。
