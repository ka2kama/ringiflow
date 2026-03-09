# 計画: ApiResponse エンベロープパターンの廃止

## コンテキスト

### 目的
- Issue: #1095
- Want: ADR-065 で決定した ApiResponse エンベロープパターンを廃止し、T を直接返す形式に変更する
- 完了基準:
  - バックエンド: `ApiResponse::new(data)` → `data` に置換し、`ApiResponse` 型を削除
  - BFF クライアント: トレイトの戻り値型から `ApiResponse<T>` を除去し、`.data` アクセスを削除
  - フロントエンド: `Api.elm` の `expectJson` から `Decode.field "data"` を除去。`getRaw` / `expectJsonRaw` を廃止
  - OpenAPI: アノテーションから `ApiResponse<T>` を除去
  - API テスト: Hurl の `jsonpath "$.data.xxx"` → `jsonpath "$.xxx"` に置換
  - `PaginatedResponse` の `data` フィールドを `items` にリネーム
  - `just check-all` が通る

### ブランチ / PR
- ブランチ: `feature/1095-remove-api-response-envelope`
- PR: #1097（Draft）

### As-Is（探索結果の要約）
- `ApiResponse` 定義: `backend/crates/shared/src/api_response.rs` — `{ data: T }` の単純なラッパー
- Core Service: 9 ファイル・66 箇所で `ApiResponse` を使用（ハンドラ + ハンドラテスト）
- BFF クライアント: `response.rs` で `ApiResponse<T>` にデシリアライズ、各クライアントトレイトが `Result<ApiResponse<T>, ...>` を返す
- BFF ハンドラ: クライアント結果の `.data` アクセス + 自前の `ApiResponse::new(...)` でレスポンス生成
- BFF テスト: `auth_integration_test.rs`（14箇所）, `workflow_definition_authz_test.rs`（22箇所）, OpenAPI スナップショット
- OpenAPI アノテーション: `body = ApiResponse<T>` 形式（BFF の utoipa アノテーション内）
- フロントエンド: `Api.elm` の `expectJson` で `Decode.field "data"` を自動適用、`getRaw`/`expectJsonRaw` が `PaginatedResponse` 用に存在
- フロントエンド: `Data/AuditLog.elm` で `PaginatedResponse` の `data` フィールドをデコード
- Hurl テスト: `$.data.` パターンが 538 行
- `PaginatedResponse`: `backend/crates/shared/src/paginated_response.rs` — `{ data: Vec<T>, next_cursor: Option<String> }`
- Core Service ハンドラテスト: `response_body` ヘルパーで `ApiResponse<T>` にデシリアライズ（folder.rs, document.rs）

### 進捗
- [ ] Phase 1: Core Service ハンドラ — ApiResponse 除去
- [ ] Phase 2: BFF クライアント — ApiResponse 除去
- [ ] Phase 3: BFF ハンドラ — ApiResponse 除去
- [ ] Phase 4: PaginatedResponse — `data` → `items` リネーム
- [ ] Phase 5: フロントエンド — auto-unwrap 除去、getRaw 廃止
- [ ] Phase 6: OpenAPI — アノテーション更新、スナップショット再生成
- [ ] Phase 7: Hurl テスト — `$.data.xxx` → `$.xxx` 置換
- [ ] Phase 8: クリーンアップ — `ApiResponse` 型削除

## 仕様整理

### スコープ
- 対象: `ApiResponse` エンベロープの全面廃止、`PaginatedResponse` の `data` → `items` リネーム
- 対象外: エラーレスポンス（RFC 9457 — 変更なし）、E2E テスト（Playwright — BFF の JSON 構造変更で自動対応）

### 操作パス

全ての変更は機械的置換であり、ユーザー操作パスの新規追加・変更はない。既存の操作パスが変更後も同じように動作することを `just check-all` で検証する。

操作パス: 該当なし（リファクタリング — 外部から見た振る舞いは変わらない。ただしレスポンス JSON 構造が変わるため API テスト・フロントエンドの同時更新が必要）

## 設計

### 設計判断

| # | 判断 | 選択肢 | 選定理由 | 状態 |
|---|------|--------|---------|------|
| 1 | Phase の粒度 | A: レイヤー単位で分割（8 Phase）/ B: 一括置換 | レイヤーごとに `cargo test` で動作確認しながら進める。一括だとエラー箇所の特定が困難 | 確定 |
| 2 | Phase 間のテスト可能性 | `cargo test` で各 Phase 完了後に検証 / Hurl は Phase 7 で一括 | Core Service / BFF はユニット・ハンドラテストで独立検証可能。Hurl は全レイヤー通るため最後 | 確定 |
| 3 | `PaginatedResponse` の `data` → `items` リネーム | Phase 4 で独立実施 | ApiResponse 除去とは独立した変更。混在すると差分が読みにくい | 確定 |

### Phase 1: Core Service ハンドラ — ApiResponse 除去

#### 確認事項
- パターン: Core Service ハンドラの `ApiResponse::new(data)` パターン → `backend/apps/core-service/src/handler/` 各ファイル
- パターン: Core Service ハンドラテストの `ApiResponse<T>` デシリアライズパターン → `response_body` ヘルパー使用箇所

#### 変更内容
- ハンドラ: `let response = ApiResponse::new(data); Ok((StatusCode::OK, Json(response)))` → `Ok((StatusCode::OK, Json(data)))`
- ハンドラテスト: `let body: ApiResponse<T> = response_body(response).await;` → `let body: T = response_body(response).await;`
- ハンドラテスト: `body.data.xxx` → `body.xxx`
- `use ringiflow_shared::ApiResponse;` の import 除去

#### テストリスト

ユニットテスト: 該当なし（ロジック変更なし）
ハンドラテスト: 既存テストが `.data` なしで通ることを確認
API テスト（該当なし — Phase 7 で対応）
E2E テスト（該当なし）

### Phase 2: BFF クライアント — ApiResponse 除去

#### 確認事項
- 型: `handle_response` の戻り値型 → `backend/apps/bff/src/client/core_service/response.rs`
- パターン: クライアントトレイトの戻り値型 → `user_client.rs`, `folder_client.rs`, `document_client.rs`, `task_client.rs` 等

#### 変更内容
- `response.rs`: `handle_response` の戻り値を `Result<ApiResponse<T>, _>` → `Result<T, _>` に変更
- `response.rs`: `response.json::<ApiResponse<T>>()` → `response.json::<T>()` に変更
- 各クライアントトレイト: `Result<ApiResponse<T>, _>` → `Result<T, _>` に変更（トレイト定義 + 実装）
- `response.rs` テスト: `ApiResponse<TestData>` → `TestData` に変更、`.data` アクセス除去
- テストレスポンスの JSON: `{"data": {"value": "hello"}}` → `{"value": "hello"}` に変更

#### テストリスト

ユニットテスト: `response.rs` のテスト 7 件が通ることを確認
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: BFF ハンドラ — ApiResponse 除去

#### 確認事項
- パターン: BFF ハンドラの `.data` アクセスパターン → 各ハンドラファイル
- パターン: BFF ハンドラの `ApiResponse::new(...)` パターン → 各ハンドラファイル
- パターン: BFF テスト（`auth_integration_test.rs`, `workflow_definition_authz_test.rs`）のモック戻り値

#### 変更内容
- ハンドラ: `core_response.data` → `core_response`（クライアントが `T` を直接返すようになったため）
- ハンドラ: `ApiResponse::new(data)` → `data`（BFF のレスポンスも T を直接返す）
- テストのモック戻り値: `Ok(ApiResponse::new(...))` → `Ok(...)`
- `use ringiflow_shared::ApiResponse;` の import 除去

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト: BFF ハンドラテスト + 統合テストが通ることを確認
API テスト（該当なし — Phase 7 で対応）
E2E テスト（該当なし）

### Phase 4: PaginatedResponse — `data` → `items` リネーム

#### 確認事項
- 型: `PaginatedResponse` 定義 → `backend/crates/shared/src/paginated_response.rs`
- パターン: BFF ハンドラでの `PaginatedResponse` 使用 → `audit_log.rs`
- パターン: フロントエンドのデコーダー → `frontend/src/Data/AuditLog.elm`
- パターン: Hurl テストでの `$.data` → `$.items` 変更（監査ログのみ）

#### 変更内容
- `paginated_response.rs`: `pub data: Vec<T>` → `pub items: Vec<T>`
- BFF ハンドラ: `PaginatedResponse { data: items, ... }` → `PaginatedResponse { items, ... }` （変数名 `items` なのでフィールド省略記法が使える）
- フロントエンド: `Data/AuditLog.elm` の `AuditLogList` 型の `data` → `items`、デコーダーの `"data"` → `"items"`
- Hurl テスト: 監査ログの `$.data[N]` → `$.items[N]`

#### テストリスト

ユニットテスト: `Data/AuditLog.elm` 関連の Elm テスト
ハンドラテスト（該当なし）
API テスト（該当なし — Phase 7 で対応）
E2E テスト（該当なし）

### Phase 5: フロントエンド — auto-unwrap 除去、getRaw 廃止

#### 確認事項
- 型: `Api.elm` の公開 API → `frontend/src/Api.elm`
- パターン: `getRaw` 使用箇所 → `Api/AuditLog.elm` のみ

#### 変更内容
- `Api.elm`: `expectJson` から `Decode.field "data"` を除去（`handleResponse decoder` に直接渡す）
- `Api.elm`: `getRaw` 関数を削除（`get` と同じになるため）
- `Api.elm`: `expectJsonRaw` 関数を削除
- `Api.elm`: エクスポートリストから `getRaw` を除去
- `Api/AuditLog.elm`: `Api.getRaw` → `Api.get` に変更
- `Api.elm`: モジュールドキュメントから auto-unwrap 関連の説明を更新

#### テストリスト

ユニットテスト: Elm テスト全体（`elm-test`）が通ることを確認
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 6: OpenAPI — アノテーション更新、スナップショット再生成

#### 確認事項
- パターン: BFF の `#[utoipa::path]` アノテーション内の `body = ApiResponse<T>` → 各ハンドラファイル
- 生成物: `openapi/openapi.yaml` は `just openapi-generate` で生成
- 生成物: `backend/apps/bff/tests/snapshots/openapi_spec__openapi_spec.snap`

#### 変更内容
- BFF ハンドラ: `body = ApiResponse<T>` → `body = T` に置換
- `just openapi-generate` で `openapi.yaml` を再生成
- `cargo test -p ringiflow-bff` で OpenAPI スナップショットを更新（`UPDATE_EXPECT=1`）
- `generate_openapi.rs` の `ApiResponse` 関連コメント更新

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト: OpenAPI スナップショットテストが通ることを確認
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 7: Hurl テスト — `$.data.xxx` → `$.xxx` 置換

#### 確認事項
- パターン: Hurl テストの `$.data.` パターン → `backend/tests/api/` 配下

#### 変更内容
- `jsonpath "$.data.xxx"` → `jsonpath "$.xxx"` に機械的置換
- `jsonpath "$.data[N]"` → `jsonpath "$[N]"` に変更
- `jsonpath "$.data" count` → `jsonpath "$" count` に変更（ルート配列の場合は `jsonpath "$" count`）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト: `just test-api` で全 Hurl テストが通ることを確認
E2E テスト（該当なし）

### Phase 8: クリーンアップ — `ApiResponse` 型削除

#### 確認事項
- 型: `api_response.rs` が他から参照されていないこと → Grep で確認
- パターン: `shared/src/lib.rs` のエクスポート

#### 変更内容
- `backend/crates/shared/src/api_response.rs` を削除
- `backend/crates/shared/src/lib.rs` から `pub use api_response::ApiResponse;` と `mod api_response;` を除去
- `paginated_response.rs` のドキュメントコメントから `ApiResponse` への言及を更新

#### テストリスト

ユニットテスト: `cargo test` が通ることを確認
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップ

### ギャップ発見の観点 進行状態

| 観点 | 状態 | メモ |
|------|------|------|
| 未定義 | 完了 | 全変更箇所を Grep で特定済み |
| 曖昧 | 完了 | 各 Phase の変更パターンが具体的 |
| 競合・エッジケース | 完了 | `PaginatedResponse` の `data` と `ApiResponse` の `data` の名前衝突は Phase 4 で解消 |
| 不完全なパス | 完了 | Hurl テストの `$.data` パターンには配列参照 `$.data[N]` も含まれる。Phase 7 で対応 |
| アーキテクチャ不整合 | 完了 | 変更後のレイヤー構造は ADR-065 の決定に沿う |
| 責務の蓄積 | 完了 | 該当なし（責務は変更しない） |
| 既存手段の見落とし | 完了 | sed 等での機械的置換も可能だが、Rust/Elm コンパイラでの検証を優先 |
| テスト層網羅漏れ | 完了 | 各 Phase でテスト層を明記 |
| 操作パス網羅漏れ | 完了 | 操作パスの変更なし（リファクタリング） |

### ループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Core Service ハンドラテストが `response_body` ヘルパーで `ApiResponse<T>` にデシリアライズしている | 不完全なパス | Phase 1 にテスト変更を追加 |
| 2回目 | BFF 統合テスト（`auth_integration_test.rs`, `workflow_definition_authz_test.rs`）にも `ApiResponse` が使われている | 未定義 | Phase 3 に BFF テスト変更を追加 |
| 3回目 | Hurl テストの `$.data` パターンに配列参照（`$.data[N]`）と count（`$.data count`）がある | 曖昧 | Phase 7 の変更内容に明記 |
| 4回目 | `generate_openapi.rs` に `ApiResponse` に関するコメントがある | 未定義 | Phase 6 に追加 |

### 未解決の問い
- なし

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全 ApiResponse 使用箇所が計画に含まれている | OK | Grep で全使用箇所を特定し、Phase 1-8 にマッピング済み |
| 2 | 曖昧さ排除 | 各 Phase の変更パターンが具体的 | OK | コードスニペットで変換前後を明記 |
| 3 | 設計判断の完結性 | Phase 粒度、テスト戦略が確定 | OK | 設計判断テーブルに 3 件記載、全て確定 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | エラーレスポンス、E2E テストを対象外として明記 |
| 5 | 技術的前提 | utoipa の body 記法が確認済み | OK | 既存コードの `body = ApiResponse<T>` パターンを確認 |
| 6 | 既存ドキュメント整合 | ADR-065 と矛盾なし | OK | ADR-065 の「今後のアクション」と完了基準が一致 |
