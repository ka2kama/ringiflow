# #377 BFF Client レスポンスハンドリング共通化

## Context

BFF Client の Core Service 呼び出しにおける `match response.status()` パターンが 19 メソッド・3 ファイルで重複している（#373 の jscpd で検出）。各メソッドの match ブロックは 10〜30 行あり、全体で約 380 行の重複。保守性の低下（変更時に 19 箇所を修正）と一貫性の欠如（処理するステータスコードがメソッドにより異なる）が問題。

## 対象

- `backend/apps/bff/src/client/core_service/workflow_client.rs`（12 メソッド）
- `backend/apps/bff/src/client/core_service/task_client.rs`（4 メソッド）
- `backend/apps/bff/src/client/core_service/user_client.rs`（3 メソッド）

## 対象外

- `backend/apps/bff/src/client/auth_service.rs`（異なるエラー型体系のため別スコープ）
- トレイト定義（変更なし。ISP に基づくサブトレイト分割はそのまま維持）
- ハンドラ側のエラーハンドリング（変更なし）

## 設計

### 核心的な洞察

19 メソッドの差異を分析すると、**本質的に異なるのは NOT_FOUND のマッピング先だけ**。

| ステータス | マッピング | 差異 |
|-----------|----------|------|
| 2xx | `response.json::<ApiResponse<T>>()` | なし（全メソッド共通） |
| 404 | メソッドにより異なるバリアント | **唯一の差異** |
| 400 | `ValidationError(body)` | なし（処理する場合は常に同じ） |
| 403 | `Forbidden(body)` | なし |
| 409 | `Conflict(body)` | なし |
| その他 | `Unexpected(format!(...))` | なし |

### handle_response 関数

`response.rs`（新規）に以下の関数を作成:

```rust
pub(super) async fn handle_response<T: DeserializeOwned>(
    response: reqwest::Response,
    not_found_error: Option<CoreServiceError>,
) -> Result<ApiResponse<T>, CoreServiceError> {
    let status = response.status();

    if status.is_success() {
        let body = response.json::<ApiResponse<T>>().await?;
        return Ok(body);
    }

    if status == reqwest::StatusCode::NOT_FOUND {
        if let Some(err) = not_found_error {
            return Err(err);
        }
    }

    let body = response.text().await.unwrap_or_default();

    match status {
        reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::Unexpected(format!(
            "予期しないステータス {}: {}",
            status, body
        ))),
        reqwest::StatusCode::BAD_REQUEST => Err(CoreServiceError::ValidationError(body)),
        reqwest::StatusCode::FORBIDDEN => Err(CoreServiceError::Forbidden(body)),
        reqwest::StatusCode::CONFLICT => Err(CoreServiceError::Conflict(body)),
        _ => Err(CoreServiceError::Unexpected(format!(
            "予期しないステータス {}: {}",
            status, body
        ))),
    }
}
```

**設計判断:**

1. **関数 vs trait vs マクロ**: 単純な関数で十分。差異は NOT_FOUND のマッピング先だけであり、trait の表現力は過剰。マクロはデバッグ困難
2. **`Option<CoreServiceError>` for NOT_FOUND**: NOT_FOUND を処理しないメソッド（list 系）には `None`。`None` の場合は Unexpected にフォールスルー
3. **BAD_REQUEST/FORBIDDEN/CONFLICT を常に処理**: 一部メソッドが現在これらを処理しないのは「期待しない」だけで、処理しても実害なし。一貫性向上のメリットが上回る
4. **`pub(super)` visibility**: core_service モジュール内部でのみ使用
5. **response.rs 新規ファイル**: error.rs はエラー型定義、response.rs はレスポンス処理という責務分離

### 各メソッドの書き換え例

```rust
// Before (10-30行)
async fn approve_step(&self, ...) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
    let url = format!(...);
    let response = self.client.post(&url).json(&req).send().await?;
    match response.status() {
        status if status.is_success() => { ... }
        reqwest::StatusCode::NOT_FOUND => Err(CoreServiceError::StepNotFound),
        reqwest::StatusCode::BAD_REQUEST => { ... }
        reqwest::StatusCode::FORBIDDEN => { ... }
        reqwest::StatusCode::CONFLICT => { ... }
        status => { ... }
    }
}

// After (3行)
async fn approve_step(&self, ...) -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError> {
    let url = format!(...);
    let response = self.client.post(&url).json(&req).send().await?;
    handle_response(response, Some(CoreServiceError::StepNotFound)).await
}
```

## 変更対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/Cargo.toml` | `[workspace.dependencies]` に `http = "1"` 追加 |
| `backend/apps/bff/Cargo.toml` | `[dev-dependencies]` に `http.workspace = true` 追加 |
| `backend/apps/bff/src/client/core_service.rs` | `mod response;` 追加 |
| `backend/apps/bff/src/client/core_service/response.rs` | **新規** — `handle_response` 関数 + テスト |
| `backend/apps/bff/src/client/core_service/user_client.rs` | 3 メソッドを `handle_response` に置き換え |
| `backend/apps/bff/src/client/core_service/task_client.rs` | 4 メソッドを `handle_response` に置き換え |
| `backend/apps/bff/src/client/core_service/workflow_client.rs` | 12 メソッドを `handle_response` に置き換え |

## 完了基準

- [ ] `handle_response` 関数のユニットテストが全パターンをカバー
- [ ] 19 メソッドのレスポンスハンドリングが `handle_response` に置き換え済み
- [ ] `just check-all` 通過
- [ ] jscpd のクローン数が削減されている

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: handle_response 関数の実装

#### 確認事項
- 型: `ApiResponse<T>` → `backend/crates/shared/src/api_response.rs`（`{ data: T }`, Serialize + Deserialize）
- 型: `CoreServiceError` → `backend/apps/bff/src/client/core_service/error.rs`（9 バリアント）
- ライブラリ: `reqwest::Response::from(http::Response)` → reqwest 0.12 で `From<http::Response<T>> where T: Into<Body>` が実装済み（Cargo.lock で `http` v1.4.0 確認）
- ライブラリ: `serde::de::DeserializeOwned` → プロジェクト内初使用だが標準的な trait bound
- パターン: 既存 status match パターン → 3 クライアントファイルで確認済み

#### テストリスト
- [ ] 成功レスポンス (200) を `ApiResponse<T>` にデシリアライズする
- [ ] 404 + `not_found_error: Some(...)` で指定エラーを返す
- [ ] 404 + `not_found_error: None` で `Unexpected` を返す
- [ ] 400 で `ValidationError(body)` を返す
- [ ] 403 で `Forbidden(body)` を返す
- [ ] 409 で `Conflict(body)` を返す
- [ ] 500 で `Unexpected` を返す
- [ ] 成功だが不正な JSON で `Network` エラーを返す

#### 依存関係
- `backend/Cargo.toml` の `[workspace.dependencies]` に `http = "1"` を追加
- `backend/apps/bff/Cargo.toml` の `[dev-dependencies]` に `http.workspace = true` を追加

テストでの `reqwest::Response` 構築:
```rust
fn make_response(status: u16, body: &str) -> reqwest::Response {
    let http_resp = http::Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body.to_string())
        .unwrap();
    reqwest::Response::from(http_resp)
}
```

### Phase 2: クライアントメソッドのリファクタリング

#### 確認事項
なし（Phase 1 で確認済みのパターンのみ）

#### リファクタリング順序
1. `user_client.rs`（3 メソッド）— 最小ファイルから開始
2. `task_client.rs`（4 メソッド）
3. `workflow_client.rs`（12 メソッド）— 最大ファイル

各ファイル書き換え後に `just check` で確認。全完了後に `just check-all`。

#### not_found_error マッピング表

| メソッド | not_found_error |
|---------|----------------|
| `list_users` | `None` |
| `get_user_by_email` | `Some(UserNotFound)` |
| `get_user` | `Some(UserNotFound)` |
| `list_my_tasks` | `None` |
| `get_task` | `Some(StepNotFound)` |
| `get_dashboard_stats` | `None` |
| `get_task_by_display_numbers` | `Some(StepNotFound)` |
| `create_workflow` | `Some(WorkflowDefinitionNotFound)` |
| `submit_workflow` | `Some(WorkflowInstanceNotFound)` |
| `list_workflow_definitions` | `None` |
| `get_workflow_definition` | `Some(WorkflowDefinitionNotFound)` |
| `list_my_workflows` | `None` |
| `get_workflow` | `Some(WorkflowInstanceNotFound)` |
| `approve_step` | `Some(StepNotFound)` |
| `reject_step` | `Some(StepNotFound)` |
| `get_workflow_by_display_number` | `Some(WorkflowInstanceNotFound)` |
| `submit_workflow_by_display_number` | `Some(WorkflowInstanceNotFound)` |
| `approve_step_by_display_number` | `Some(StepNotFound)` |
| `reject_step_by_display_number` | `Some(StepNotFound)` |

## 検証

1. `just check` — lint + テスト（各 Phase 完了時）
2. `just check-all` — lint + テスト + API テスト（全完了時）
3. `just check-duplicates` — jscpd のクローン数が削減されていることを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `http` crate が workspace 依存に未登録 | 技術的前提 | Phase 1 の依存関係追加手順に明記 |
| 2回目 | NOT_FOUND + None の場合、body を読み取ってから Unexpected にする必要があるが、先に `if let Some` で early return するため NOT_FOUND の body が取れない | 不完全なパス | `if let Some` で early return した後、NOT_FOUND を body 読み取り後の match に含めて Unexpected にフォールスルーする設計に修正 |
| 3回目 | テスト「不正 JSON で Network エラー」の変換パスが曖昧 | 曖昧 | `reqwest::Error` → `From<reqwest::Error> for CoreServiceError` → `Network(String)` の変換パスを明記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 全 19 メソッドをコードから読み取り、not_found_error マッピング表で 1:1 対応を確認 |
| 2 | 曖昧さ排除 | OK | 関数シグネチャ、テストリスト、各メソッドのマッピング、依存関係の追加手順がすべて具体的 |
| 3 | 設計判断の完結性 | OK | 関数 vs trait vs マクロ、BAD_REQUEST 常時処理、Option for NOT_FOUND、visibility、ファイル配置を判断済み |
| 4 | スコープ境界 | OK | 対象（core_service 19 メソッド）と対象外（auth_service、トレイト定義、ハンドラ）を明記 |
| 5 | 技術的前提 | OK | reqwest 0.12 の `From<http::Response>` impl、`http` v1.4.0（Cargo.lock 確認）、`DeserializeOwned` trait bound |
| 6 | 既存ドキュメント整合 | OK | ISP サブトレイト分割を維持。response.rs は内部実装でトレイト構造を変更しない |
