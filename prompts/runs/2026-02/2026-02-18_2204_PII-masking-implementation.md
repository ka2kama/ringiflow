# PII マスキング基盤の実装

## 概要

Issue #651 に基づき、PII（個人識別情報）がログに平文出力されることを防ぐマスキング基盤を実装した。運用設計書の MUST NOT 要件（セクション 8.6/9.4）に対応するもの。

既存の `PlainPassword` パターン（カスタム `Debug` で `[REDACTED]` を出力）を PII 型に拡張し、型レベルでの漏洩防止を実現した。

## 実施内容

全 5 Phase を TDD で実装:

### Phase 1: Email のカスタム Debug + Display 削除

- `Email` 型から `derive(Debug)` を除去し、カスタム `Debug` で `[REDACTED]` を出力するように変更
- `Display` impl を削除し、`to_string()` による平文取得をコンパイル時に防止
- `Email` を含む `User` 構造体は `derive(Debug)` のまま。子型のカスタム Debug が自動伝播する

### Phase 2: `define_validated_string!` マクロの PII 対応

- マクロに `pii: true` オプションを追加（2 アーム設計）
- 内部ヘルパーマクロ `_validated_string_common!` を導入し、共通コード（`new`, `as_str`, `into_string`）の重複を排除
- PII アーム: `derive(Debug)` を除外 + カスタム Debug + Display 非生成
- 非 PII アーム: 既存動作を完全維持
- `UserName` に `pii: true` を適用
- `user.name().to_string()` の 3 箇所を `user.name().as_str().to_string()` に変更

### Phase 3: SessionData のカスタム Debug

- `SessionData` から `derive(Debug)` を除去し、`email` と `name` フィールドのみ `[REDACTED]` でマスク
- 他のフィールド（`user_id`, `tenant_id`, `roles` 等）は通常表示を維持
- `Serialize`/`Deserialize` は `Debug` に依存しないため、Redis シリアライゼーションへの影響なし

### Phase 4: LoginRequest Debug マスキング + DevAuth ログ修正

- `LoginRequest` のカスタム Debug 実装（`email` と `password` を `[REDACTED]`）
- DevAuth の CSRF トークンログ出力を全文 → 先頭 8 文字に切り詰め

### Phase 5: tracing 呼び出し監査

- 全 62 箇所の tracing 呼び出しを監査し、PII 漏洩リスクがないことを確認
- エラー型の `Display` impl も検証済み（固定文字列のみ）

## 判断ログ

- マクロの共通コード抽出: PII/非 PII アームで `new`, `as_str`, `into_string` の実装が完全に同一であるため、`_validated_string_common!` 内部ヘルパーマクロで DRY 化した
- 各 Phase の Refactor で Simple Design の問いを適用し、追加のリファクタリング対象はなかった

## 成果物

コミット:
- `82d7a98` #651 Add PII masking for Email, UserName, SessionData, and LoginRequest

変更ファイル:
- `backend/crates/domain/src/user.rs` — Email カスタム Debug + テスト
- `backend/crates/domain/src/macros.rs` — マクロ 2 アーム + ヘルパーマクロ
- `backend/crates/domain/src/value_objects.rs` — UserName `pii: true` + テスト
- `backend/crates/infra/src/session.rs` — SessionData カスタム Debug + テスト
- `backend/apps/bff/src/handler/auth/mod.rs` — LoginRequest カスタム Debug + テスト
- `backend/apps/bff/src/main.rs` — DevAuth CSRF ログ切り詰め
- `backend/apps/core-service/src/handler/auth/mod.rs` — `name().to_string()` → `name().as_str().to_string()`

PR: #668
