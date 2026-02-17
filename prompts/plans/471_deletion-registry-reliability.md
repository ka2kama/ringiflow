# Issue #471: 削除レジストリの信頼性向上

## Context

PR #466 のレビューで指摘された削除レジストリの信頼性問題を改善する。
テナント退会時のデータ削除で、DynamoDB のスループット超過時に一部アイテムが削除されない問題と、複数データストアの削除で部分失敗が適切にハンドリングされない問題を解決する。

## 対象

- `backend/crates/infra/src/deletion/dynamodb_audit_log.rs` — unprocessed_items リトライ
- `backend/crates/infra/src/deletion/mod.rs` — `DeletionReport` 型追加
- `backend/crates/infra/src/deletion/registry.rs` — `delete_all` の部分失敗ハンドリング
- `backend/crates/infra/tests/postgres_deleter_test.rs` — 統合テストの適応

## 対象外

- `TenantDeleter` トレイトのシグネチャ変更（不要）
- 他の Deleter 実装の変更
- `count_all` の部分失敗ハンドリング（診断目的のため早期リターンで問題ない）

## 設計判断

### 判断 1: リトライ方式 — 手動 exponential backoff

| 案 | メリット | デメリット |
|----|---------|-----------|
| A) `tokio::time::sleep` + 手動実装 | 依存追加なし、1箇所でシンプル | 手動実装 |
| B) `backon` クレート導入 | 汎用リトライ抽象 | 1箇所のためだけに依存追加は過剰 |

選択: A。リトライ対象が1箇所のみ。将来リトライ箇所が増えたらクレート導入を検討（YAGNI）。

### 判断 2: リトライパラメータ

AWS ベストプラクティスに従い: 最大 5 回、初回 100ms、倍率 2、上限 5 秒、jitter なし（単一クライアントから1テナント削除の文脈で並列衝突リスクが低い）。

### 判断 3: 削除件数の正確化

修正前: `deleted_count += delete_requests.len()`（リクエスト件数を加算）
修正後: リトライ成功時にリクエスト件数を加算。リトライ上限超過時は `リクエスト件数 - 未処理件数` を加算。

### 判断 4: delete_all の戻り値型 — `DeletionReport`（`Result` ラップなし）

| 案 | メリット | デメリット |
|----|---------|-----------|
| A) `Result<HashMap<..>, InfraError>` のまま | 変更なし | 部分失敗を表現できない |
| B) `Result<DeletionReport, InfraError>` | Result で統一 | `?` で unwrap され部分失敗チェックが省略される |
| C) `DeletionReport` を直接返す | 呼び出し元に部分失敗チェックを型で強制 | `Result` パターンからの逸脱 |

選択: C。個別 Deleter のエラーは `DeletionReport::failed` に集約。`delete_all` 自体が失敗するケース（ループ前のエラー等）は存在しない。`Result` でラップしないことで `?` による安易な unwrap を防ぎ、`has_failures()` チェックを構造的に強制する。

```rust
pub struct DeletionReport {
    pub succeeded: HashMap<&'static str, DeletionResult>,
    pub failed: Vec<(&'static str, InfraError)>,
}

impl DeletionReport {
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }
}
```

### 判断 5: DeletionReport の配置

`DeletionResult` と同じ `deletion/mod.rs` に配置。削除結果を表す型群として自然な集約。

### 判断 6: ログ出力

`tracing` でオペレーション上重要な情報を出力:
- リトライ発生時: `tracing::warn!`
- リトライ上限超過時: `tracing::error!`
- `delete_all` 個別 Deleter 失敗時: `tracing::error!`

## 実装計画

### Phase 1: DynamoDB unprocessed_items リトライ

変更ファイル: `backend/crates/infra/src/deletion/dynamodb_audit_log.rs`

#### 確認事項

- [x] ライブラリ: `BatchWriteItemOutput::unprocessed_items()` の戻り値型 → `Option<&HashMap<String, Vec<WriteRequest>>>`（docs.rs 確認済み）
- [x] ライブラリ: `tokio::time::sleep` の使用 → infra Cargo.toml に `tokio.workspace = true` 存在
- [x] パターン: `tracing` の使用 → `dynamodb.rs` で `tracing::debug!`/`info!` 使用、infra Cargo.toml に `tracing.workspace = true`

#### テストリスト

ユニットテスト:
- [x] `compute_backoff_ms`: リトライ 0 回目 → 100ms
- [x] `compute_backoff_ms`: リトライ 1 回目 → 200ms
- [x] `compute_backoff_ms`: リトライ 4 回目 → 1600ms
- [x] `compute_backoff_ms`: 上限超過（10 回目）→ 5000ms（cap）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

注: DynamoDB Local はスループット制限がなく `unprocessed_items` を再現できないため、BatchWriteItem + リトライの統合テストは正常パス（unprocessed_items が空）の検証に限定される。backoff 計算ロジックの正確性はユニットテストで担保。

#### 実装方針

リトライ定数を定義:

```rust
const MAX_RETRIES: u32 = 5;
const INITIAL_BACKOFF_MS: u64 = 100;
const MAX_BACKOFF_MS: u64 = 5_000;
```

`compute_backoff_ms(retry: u32) -> u64` を純粋関数として抽出。

`delete` メソッド内の BatchWriteItem ループを改修:

```rust
let requested_count = delete_requests.len() as u64;
let mut remaining_requests = delete_requests;

for retry in 0..=MAX_RETRIES {
    if retry > 0 {
        let backoff = compute_backoff_ms(retry - 1);
        tracing::warn!(retry, unprocessed = remaining_requests.len(), backoff_ms = backoff,
            "DynamoDB BatchWriteItem: 未処理アイテムをリトライ");
        tokio::time::sleep(Duration::from_millis(backoff)).await;
    }

    let output = self.client.batch_write_item()
        .request_items(&self.table_name, remaining_requests)
        .send().await
        .map_err(|e| InfraError::DynamoDb(format!("監査ログの削除に失敗: {e}")))?;

    let unprocessed = output.unprocessed_items()
        .and_then(|items| items.get(&self.table_name))
        .cloned()
        .unwrap_or_default();

    if unprocessed.is_empty() {
        deleted_count += requested_count;
        break;
    }

    if retry == MAX_RETRIES {
        let unprocessed_count = unprocessed.len() as u64;
        tracing::error!(unprocessed = unprocessed_count,
            "DynamoDB BatchWriteItem: リトライ上限超過、未処理アイテムが残存");
        deleted_count += requested_count - unprocessed_count;
        return Err(InfraError::DynamoDb(format!(
            "監査ログの削除でリトライ上限超過: {}件が未処理", unprocessed_count)));
    }

    remaining_requests = unprocessed;
}
```

### Phase 2: DeletionReport 型と部分失敗ハンドリング

変更ファイル:
- `backend/crates/infra/src/deletion/mod.rs`
- `backend/crates/infra/src/deletion/registry.rs`
- `backend/crates/infra/tests/postgres_deleter_test.rs`

#### 確認事項

- [x] 型: `InfraError` が `Send + Sync` を満たすか → 全バリアント（sqlx::Error, redis::RedisError, serde_json::Error, String）が Send+Sync。OK
- [x] パターン: `DeletionResult` の pub export → `mod.rs` で定義、`lib.rs` で `pub mod deletion`

#### テストリスト

ユニットテスト（`registry.rs` 内）:
- [x] `delete_all`: 全成功時、`succeeded` に全結果が入り `failed` が空
- [x] `delete_all`: 1 つ目の Deleter が失敗しても残りの Deleter が実行される
- [x] `delete_all`: 最後の Deleter が失敗した場合、先行の成功結果が `succeeded` に入る
- [x] `delete_all`: 全 Deleter が失敗した場合、`succeeded` が空で `failed` に全エラー
- [x] `has_failures()`: 失敗あり → true、なし → false

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

統合テスト（`postgres_deleter_test.rs`）:
- [x] 既存テスト `test_delete_allがfk制約に違反せず全テーブルを削除できる` を `DeletionReport` 型に適応

#### 実装方針

`mod.rs` に `DeletionReport` 型を追加（上記設計判断 4 参照）。

`registry.rs` の `delete_all`:
- 戻り値を `Result<HashMap<..>, InfraError>` → `DeletionReport` に変更
- `?` 早期リターンを `match` に変更し、全 Deleter を実行

MockDeleter に `failing()` コンストラクタを追加（エラーを返す Deleter のシミュレート用）。

統合テストの適応:
```rust
// 変更前: let results = registry.delete_all(&tenant_id).await.unwrap();
// 変更後:
let report = registry.delete_all(&tenant_id).await;
assert!(!report.has_failures(), "削除に失敗した Deleter: {:?}", report.failed);
assert_eq!(report.succeeded["postgres:workflows"].deleted_count, 3);
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1 | DynamoDB Client のモック化困難でリトライの統合テストが書けない | 不完全なパス | backoff 計算を純粋関数として抽出しユニットテスト。BatchWriteItem 全体は DynamoDB Local（正常パスのみ） |
| 2 | `Result<DeletionReport, _>` だと `?` で部分失敗チェックが省略される | 型の活用 | `DeletionReport` を直接返し型レベルで部分失敗チェックを強制 |
| 3 | DynamoDB Local で unprocessed_items を再現不可 | 競合・エッジケース | テスト制約として明記。backoff 計算のユニットテストで補完 |
| 4 | `count_all` にも部分失敗ハンドリングが必要か | スコープ境界 | 対象外。診断目的なので早期リターンで問題ない |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | `dynamodb_audit_log.rs`（リトライ）、`mod.rs`（型追加）、`registry.rs`（部分失敗）、テスト3ファイル。`delete_all` の呼び出し元はテストのみ（Grep で確認） |
| 2 | 曖昧さ排除 | OK | リトライパラメータ（5回、100ms、2倍、5秒）、型定義（フィールド名・型）、テストケース（入出力）すべて具体的 |
| 3 | 設計判断の完結性 | OK | 6 つの設計判断に選択肢・理由・トレードオフを記載 |
| 4 | スコープ境界 | OK | 対象（3ファイル + テスト）と対象外（`count_all`、他 Deleter、トレイトシグネチャ）を明記 |
| 5 | 技術的前提 | OK | DynamoDB Local のスループット制限なし、`BatchWriteItemOutput::unprocessed_items()` の API、`backon` は redis 内部依存 |
| 6 | 既存ドキュメント整合 | OK | 詳細設計書の削除フロー、TODO(#471) コメントと対応 |

## 検証方法

1. `cd backend && cargo test -p ringiflow-infra` — ユニットテスト（backoff 計算、MockDeleter 部分失敗）
2. `just test-rust-integration` — 統合テスト（PostgreSQL delete_all）
3. `just check-all` — 全体のリント + テスト
