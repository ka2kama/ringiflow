# ErrorResponse 統一リファクタリング

## 概要

BFF・Core Service・Auth Service に散在していた `ErrorResponse` 構造体（5+1 箇所の重複定義）を `ringiflow-shared` クレートに統一した。便利コンストラクタにより error_type URI のハードコードも排除した。

## 背景と目的

- Issue: [#181](https://github.com/ka2kama/ringiflow/issues/181)
- PR: [#257](https://github.com/ka2kama/ringiflow/pull/257)

`ErrorResponse` 構造体が以下の 6 箇所に重複定義されていた:

| 場所 | 備考 |
|------|------|
| `auth-service/src/error.rs` | |
| `core-service/src/error.rs` | |
| `core-service/src/handler/auth.rs` | |
| `bff/src/handler/auth.rs` | ヘルパー関数 4 つも含む |
| `bff/src/handler/workflow.rs` | ヘルパー関数 6 つ + `TenantIdError` も含む |
| `bff/src/middleware/csrf.rs` | `CsrfErrorResponse`（同一構造） |

さらに `TenantIdError` が BFF の `auth.rs` と `workflow.rs` で重複定義されていた。

## 実施内容

4 Phase で段階的にリファクタリングした。

### Phase 1: shared に ErrorResponse を追加

`error_response.rs` を新規作成:

- `ErrorResponse` 構造体（`Serialize` / `Deserialize`）
- `ERROR_TYPE_BASE` 定数でベース URI を一元管理
- 便利コンストラクタ: `bad_request`, `unauthorized`, `forbidden`, `not_found`, `conflict`, `validation_error`, `internal_error`, `service_unavailable`
- 7 テストケース

### Phase 2: Auth Service の統一

- `ringiflow-shared` 依存を追加
- ローカルの `ErrorResponse` を削除、shared から import
- サービス固有エラー（`authentication-failed`, `credential-not-found`）は `ErrorResponse::new()` を使用

### Phase 3: Core Service の統一

- `error.rs` と `handler/auth.rs` のローカル定義を削除
- `IntoResponse for CoreError` を便利コンストラクタで書き換え

### Phase 4: BFF の統一

- `bff/src/error.rs` に `TenantIdError`・ヘルパー関数を集約
- `handler/auth.rs` と `handler/workflow.rs` から重複コードを削除（合計 -209 行）
- `CsrfErrorResponse` を shared `ErrorResponse` に統一
- `lib.rs` に `pub mod error;` を追加（`main.rs` からの移動）

## 設計上の判断

### ErrorResponse を pure data に留める

`IntoResponse` 実装を shared に入れず、各サービスの責務として残した。shared に axum 依存を入れないことで、shared の設計方針「外部クレートへの依存は最小限」を維持。

### error_type URI は便利コンストラクタで管理

enum ベース（過度な型安全）やconst 文字列（冗長）ではなく、コンストラクタに URI ロジックを内包するアプローチを採用。サービス固有のエラーは `ErrorResponse::new()` で自由に作成可能。

### validation_error のステータスコードを 400 に設定

当初 422 を計画していたが、プロジェクト全体で validation error に HTTP 400 を使用しており、API テストでも `status == 400` を検証していた。RFC 9457 の規定（`status` フィールドは HTTP ステータスコードと一致すべき）に従い 400 に修正。

## 成果物

### コミット

- `bbf0eb9` Unify ErrorResponse across all services into ringiflow-shared

### 変更ファイル

16 ファイル（+455 / -464 行）:

- 新規: `backend/crates/shared/src/error_response.rs`
- 修正: 15 ファイル（3 サービスのエラー関連ファイル + BFF ハンドラ群）

## 議論の経緯

特筆すべき議論はなし。計画に基づき実装を進めた。

## 学んだこと

### lib.rs と main.rs のクレート構造

BFF は `lib.rs` と `main.rs` の両方を持つクレート。この場合、Rust は 2 つの別々のクレートルートを作る。ハンドラモジュールが `lib.rs` 側にあるため、新しい `error` モジュールも `lib.rs` に宣言する必要があった。`main.rs` にだけ `mod error;` を置くと、ライブラリクレートのモジュールからは `crate::error` が見えない。

### cargo のキャッシュ不整合

`lib.rs` に `pub use` を追加した後、キャッシュが不整合を起こし「`ErrorResponse` が見つからない」エラーが発生。`cargo clean` で解消した。構造的な変更後はクリーンビルドが有効。

## 次のステップ

- PR レビュー対応後、マージ
