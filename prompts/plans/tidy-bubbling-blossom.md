# #651 PII マスキング / ログサニタイゼーション基盤

## コンテキスト

運用設計書で MUST NOT 要件として「ログには PII/シークレットを出力してはならない」が定義されているが、PII マスキング機構が未実装。`Email`、`UserName` 等の PII 型が `derive(Debug)` と平文 `Display` を持ち、tracing ログに漏洩するリスクがある。

既存の `PlainPassword` が `[REDACTED]` パターンを実装済みであり、このパターンを PII 型に拡張する。

## アプローチ

**Newtype + カスタム Debug**（`secrecy` クレートではなく既存の PlainPassword パターンに統一）

- `secrecy` はクリプト用途向けで PII には過剰（zeroize 不要）
- PlainPassword パターンが既にプロジェクトに存在し、テスト済み
- ゼロコスト、新規依存なし、tracing と自然に統合

## 対象

| 型 | ファイル | PII フィールド | 現状 |
|---|---------|--------------|------|
| `Email` | `domain/src/user.rs` L66 | メールアドレス | derive(Debug) + 平文 Display |
| `UserName` | マクロ生成 `domain/src/value_objects.rs` L323 | 氏名 | derive(Debug) + 平文 Display |
| `SessionData` | `infra/src/session.rs` L40 | email, name (String) | derive(Debug) |
| `LoginRequest` | `bff/handler/auth/mod.rs` L38 | email, password (String) | derive(Debug) |
| DevAuth ログ | `bff/src/main.rs` L161 | CSRF Token | 平文ログ出力 |

## 対象外

- `SessionData` のフィールドを String → ドメイン型に変更（Redis シリアライゼーション互換の破壊）
- カスタム tracing Layer（型レベルマスキングで十分）
- 監査ログの `actor_name` フィールド（DynamoDB 保存用、ログ出力されない）
- `WorkflowName`（PII ではない）
- API レスポンス型の String フィールド（API 契約）
- `PasswordHash`（ハッシュ値は PII ではない、Issue スコープ外）

---

## Phase 1: Email — カスタム Debug、Display 削除

### 変更ファイル

- `backend/crates/domain/src/user.rs`
  - L66: `derive(Debug)` → `derive(Clone, PartialEq, Eq, Serialize, Deserialize)` + カスタム Debug
  - L123-127: `Display` impl 削除
  - `User` 構造体は `derive(Debug)` のまま（Email の Debug がマスクされるため自動的に安全）

### 影響分析

- `user.email().as_str().to_string()` パターンが既に使われている（`core-service/handler/auth/mod.rs:68`）— 影響なし
- `user.email().to_string()` の呼び出し — **なし**（Grep 確認済み）
- Email の Display 削除はコンパイルエラーで検出可能

### 確認事項

- [x] 型: Email の derive リスト → `domain/src/user.rs` L66, `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`
- [x] パターン: PlainPassword の Debug impl → `domain/src/password.rs` L24-28, `f.debug_tuple("PlainPassword").field(&"[REDACTED]").finish()`
- [x] 使用: Email の Display 呼び出し箇所 → Grep 確認済み、0 件

### テストリスト

ユニットテスト:
- [x] Email の Debug 出力が `[REDACTED]` を含み、実際のメールアドレスを含まない
- [x] Email の `as_str()` が実際の値を返す（機能維持）
- [x] User の Debug 出力が平文のメールアドレスを含まない

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 2: `define_validated_string!` マクロ — `pii` オプション追加

### 変更ファイル

- `backend/crates/domain/src/macros.rs`
  - L87-138: `pii: true` を受け付ける新しいマクロアームを追加
  - PII アーム: Debug を derive から外し、カスタム Debug（`[REDACTED]`）を生成。Display は生成しない
  - 非 PII アーム: 既存の動作をそのまま維持

- `backend/crates/domain/src/value_objects.rs`
  - L323-336: `UserName` に `pii: true` を追加

- `backend/apps/core-service/src/handler/auth/mod.rs`
  - L69, L121, L414: `user.name().to_string()` → `user.name().as_str().to_string()`

### マクロ設計

```rust
// PII アーム（新規）
macro_rules! define_validated_string {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident {
            label: $label:expr,
            max_length: $max_length:expr,
            pii: true $(,)?
        }
    ) => {
        // derive から Debug を除外
        #[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
        // カスタム Debug: [REDACTED]
        // Display: 生成しない
        // new(), as_str(), into_string() は共通
    };
    // 非 PII アーム（既存のまま）
    ( ... ) => { ... };
}
```

### 影響分析

- `user.name().to_string()` — 3 箇所（上記）で Display を使用 → `as_str().to_string()` に変更
- `session_data.name().to_string()` — `SessionData::name()` は `&str` を返す → `str::to_string()` であり影響なし
- `WorkflowName` — `pii` パラメータなし → 既存動作を維持

### 確認事項

- [x] 型: 現在のマクロ定義 → `domain/src/macros.rs` L87-138, derive(Debug) + Display impl 生成
- [x] 使用: `user.name().to_string()` の全箇所 → `core-service/handler/auth/mod.rs` L69, L121, L414（3箇所、計画通り）
- [x] 検証: WorkflowName が Display を維持すること → `pii` パラメータなしの既存アームで動作維持

### テストリスト

ユニットテスト:
- [x] UserName の Debug 出力が `[REDACTED]` を含み、実際の名前を含まない
- [x] UserName の `as_str()` が実際の値を返す（機能維持）
- [x] WorkflowName の Debug 出力が実際の値を表示する（既存動作維持）
- [x] WorkflowName の Display 出力が実際の値を表示する（既存動作維持）
- [x] User の Debug 出力が平文のユーザー名を含まない

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 3: SessionData — カスタム Debug

### 変更ファイル

- `backend/crates/infra/src/session.rs`
  - L40: `derive(Debug, ...)` → `derive(Clone, Serialize, Deserialize)` + カスタム Debug
  - カスタム Debug: `email` と `name` フィールドを `[REDACTED]`、他フィールドは通常出力

### 影響分析

- `Serialize`/`Deserialize` は `Debug` に依存しない → Redis シリアライゼーション影響なし
- 既存テスト 2 件は serde テストであり Debug を使っていない → 影響なし

### 確認事項

- [x] 型: SessionData のフィールドと derive リスト → `infra/src/session.rs` L40-51, derive(Debug, Clone, Serialize, Deserialize), email/name は String
- [x] パターン: PlainPassword Debug パターン → Phase 1 で確認済み
- [x] 検証: serde Serialize/Deserialize が Debug に依存しないこと → Serialize/Deserialize は独立トレイト

### テストリスト

ユニットテスト:
- [x] SessionData の Debug 出力が email と name フィールドで `[REDACTED]` を含む
- [x] SessionData の Debug 出力が実際の email/name 値を含まない
- [x] SessionData の Debug 出力が非 PII フィールド（user_id, tenant_id, roles）を通常表示する
- [x] SessionData の JSON シリアライゼーション/デシリアライゼーションが正常動作する（回帰）— 既存テスト 2 件で確認

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 4: LoginRequest Debug マスキング + DevAuth ログ修正

### 変更ファイル

- `backend/apps/bff/src/handler/auth/mod.rs`
  - L38: `#[derive(Debug, Deserialize, ToSchema)]` → `#[derive(Deserialize, ToSchema)]` + カスタム Debug
  - カスタム Debug: `email` と `password` の両方を `[REDACTED]`

- `backend/apps/bff/src/main.rs`
  - L161: `tracing::info!("  CSRF Token: {}", csrf_token)` → `tracing::info!("  CSRF Token: {}...", &csrf_token[..8])`

### 影響分析

- LoginRequest は axum の `Json<LoginRequest>` でデシリアライズ → `Deserialize` があれば動作。Debug は不要
- DevAuth は `#[cfg(feature = "dev-auth")]` でコンパイル時ゲート → 本番影響なし

### 確認事項

- [x] 型: LoginRequest の定義と derive → `bff/handler/auth/mod.rs` L38-42, derive(Debug, Deserialize, ToSchema)
- [x] 使用: LoginRequest が Debug 出力される箇所 → Grep 確認、0 件（予防的マスキング）
- [x] パターン: DevAuth ログ行 → `bff/src/main.rs` L161, CSRF Token 平文出力

### テストリスト

ユニットテスト:
- [x] LoginRequest の Debug 出力が email と password で `[REDACTED]` を含む
- [x] LoginRequest の Debug 出力が実際の email/password 値を含まない

ハンドラテスト（該当なし — 既存のログインテストが機能検証）

API テスト（該当なし）

E2E テスト（該当なし）

---

## Phase 5: 既存 tracing 呼び出しの監査

全 62 箇所の tracing 呼び出しを監査し、PII 漏洩リスクがないことを確認する。

### 監査対象

- エラー型の Display impl が PII を含まないか（`CoreError`, `BffError`, `InfraError`）
- format 文字列が PII 値を直接補間していないか
- 構造化フィールドが PII を含まないか（`user_id = %user_id` は UUID で安全）

### 確認事項

- [x] エラー型: CoreError の Display → `core-service/src/error.rs` — thiserror の固定文字列のみ、PII なし
- [x] エラー型: BffError の Display → `bff/src/error.rs` — 一般的なエラーメッセージのみ、PII なし
- [x] 監査ログ detail JSON: `bff/handler/user.rs` の audit log — DynamoDB 保存用で tracing ログには出力されない（対象外）

### テストリスト

ユニットテスト（該当なし — 監査フェーズ）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | LoginRequest の `derive(Debug)` が password を平文出力 | 未定義 | Phase 4 に追加 |
| 2回目 | `user.name().to_string()` が 3 箇所で UserName の Display を使用 | 不完全なパス | Phase 2 の影響分析に追加、`as_str().to_string()` への変更を明記 |
| 3回目 | `session_data.name().to_string()` が UserName Display 削除の影響を受けるか | 曖昧 | SessionData::name() は &str を返すため影響なし。確認結果を記録 |
| 4回目 | `define_validated_string!` マクロの PII/非 PII アームで共通コード（new, as_str, into_string）が重複する | 重複の排除 | 内部ヘルパーマクロで共通部分を共有する設計を検討。ただし Rust のマクロ制約を考慮し、実装時に判断 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全 PII 型が対象に含まれている | OK | Email, UserName, SessionData, LoginRequest, DevAuth CSRF — DB スキーマと型定義から PII フィールドを網羅的に特定済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase で変更ファイル・行番号・コードスニペットを明示。マクロ設計は 2 アーム構造を具体化 |
| 3 | 設計判断の完結性 | 全ての選択肢に判断が記載されている | OK | secrecy vs newtype+Debug（newtype 採用、理由: 既存パターン、ゼロコスト）、マスキング形式（`[REDACTED]`、理由: PlainPassword 踏襲） |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象 5 件、対象外 6 件を明示 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | serde は Debug に依存しない、マクロの 2 アーム設計は Rust で標準パターン |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 運用設計書 8.6/9.4 の MUST NOT 要件と整合。PlainPassword パターンと一貫 |

## 検証方法

```bash
# 全テスト + lint
just check-all

# PII マスキングの手動検証
cd backend && cargo test pii       # PII 関連テストを実行
cd backend && cargo test redacted  # REDACTED 関連テストを実行

# JSON ログモードでの検証（開発サーバー起動後）
LOG_FORMAT=json just dev-all
# → ログイン操作を実行し、ログ出力にメールアドレスや氏名が平文で含まれないことを確認
```
