---
paths:
  - "backend/apps/*/src/handler/**/*.rs"
  - "backend/crates/infra/src/repository/**/*.rs"
  - "backend/apps/bff/src/client/**/*.rs"
  - "backend/crates/infra/src/session.rs"
---

# Observability 計装ルール

新しいハンドラ、リポジトリメソッド、サービスクライアントを追加する際に適用する。

## 計装パターン

すべての対象関数に `#[tracing::instrument]` を付与する。

| レイヤー | レベル | パターン | fields 例 |
|---------|--------|---------|----------|
| HTTP ハンドラ | INFO（既定） | `skip_all` + 識別子 | `fields(%id)`, `fields(display_number)` |
| BFF HTTP クライアント | DEBUG | `skip_all` + テナント/ユーザー | `fields(%tenant_id, %user_id)` |
| PostgreSQL リポジトリ | DEBUG | `skip_all` + テナント | `fields(%tenant_id)` |
| Redis セッション | DEBUG | `skip_all` + テナント | `fields(%tenant_id)` |
| DynamoDB 監査ログ | DEBUG | `skip_all` | — |

### 除外

- `health_check` — ノイズのため計装対象外
- ユースケース層 — ハンドラ + リポジトリで十分なため計装対象外

## `skip_all` と fields の使い分け

`skip_all` を既定とし、PII・大きな構造体がスパンに記録されることを防ぐ。安全なフィールド（ID、テナント ID 等）のみ `fields()` で明示的に記録する。

```rust
// Good: skip_all + 安全な識別子
#[tracing::instrument(skip_all, fields(%id))]

// Bad: 引数をそのまま記録（PII がスパンに含まれるリスク）
#[tracing::instrument]
```

### フィールド選択の判断基準

| 記録する | 記録しない |
|---------|----------|
| エンティティ ID（`%id`, `%user_id`） | リクエストボディ全体 |
| テナント ID（`%tenant_id`） | パスワード、メールアドレス |
| 表示番号（`display_number`） | セッションデータ |
| パスパラメータ（公開 URL の一部） | ヘッダー値 |

## 属性の配置順

外側から内側へ:

```rust
#[utoipa::path(...)]                          // OpenAPI メタデータ（BFF ハンドラのみ）
#[tracing::instrument(skip_all, fields(%id))] // スパン設定
pub async fn handler(...) -> ... { ... }
```

## CI チェック

`scripts/check/instrumentation.rs` が `syn` クレートの AST 解析でハンドラとリポジトリ impl の計装漏れを検出する。`just check-instrumentation` で実行。

## 参照

- Observability 設計: [docs/40_詳細設計書/14_Observability設計.md](../../docs/40_詳細設計書/14_Observability設計.md)
- ログスキーマ: [docs/80_ナレッジベース/backend/log-schema.md](../../docs/80_ナレッジベース/backend/log-schema.md)
