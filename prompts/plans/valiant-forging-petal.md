# 計画: #540 audit_log テスト並列実行の安定化

## Context

Issue #540: `just check-all` 実行時に audit_log テストが DynamoDB dispatch failure で失敗する問題。
現在は `--test-threads=1`（逐次実行）で回避しているが、全統合テストの実行時間が不必要に長くなっている。

精査の結果、元の dispatch failure は再現しないが、以下の構造的改善を実施する:
1. DynamoDB クライアント・テーブルセットアップの共有化（将来のテスト数増加に対する防御）
2. `--test-threads=1` の削除（テスト実行速度の改善）

## 対象

- `backend/crates/infra/tests/audit_log_repository_test.rs` — OnceCell による共有化
- `justfile` — `--test-threads=1` の削除

## 対象外

- `backend/crates/infra/tests/dynamodb_test.rs` — クライアント作成・テーブル作成自体をテストするため、毎テスト独立で正しい
- `backend/crates/infra/tests/common/mod.rs` — `dynamodb_endpoint()` の抽出は重複2箇所で DRY ルール（3回まで許容）の範囲内

## 設計判断

### `tokio::sync::OnceCell` の採用

`create_client()` が async のため、`std::sync::LazyLock`（Rust 1.80+, edition 2024 で利用可能）は使えない。
`tokio::sync::OnceCell` を使用する。tokio `"full"` feature が有効（`backend/Cargo.toml`）で依存追加不要。

代替案:
- `std::sync::OnceLock` + `block_on`: tokio ランタイム内で `block_on` はパニックするため不可
- `once_cell::sync::Lazy`: 外部依存の追加が必要。tokio に組み込みがあるため不要

### クライアント共有 vs テーブルセットアップ共有

実装中に発見: リポジトリ全体（= 単一クライアント）を `OnceCell` で共有すると、
内部 HTTP コネクションプールがボトルネックになり逆に dispatch failure が発生した（6/9 pass, 3/9 fail）。

対策: **テーブルセットアップのみ共有**し、各テストは独自のクライアントを作成する。
- テーブルセットアップ: `OnceCell<()>` で一度だけ実行
- クライアント: 各テストが独立に作成（コネクションプールを分離）
- データ分離: ランダム `TenantId` で既に保証されている

## Phase 1: テストコードの共有化

### 確認事項
- [x] 型: `DynamoDbAuditLogRepository` の `record`/`find_by_tenant` → `&self`（共有参照で OK）
- [x] ライブラリ: `tokio::sync::OnceCell::const_new()`, `get_or_init()` → tokio "full" feature で利用可能
- [x] パターン: 既存テストの `setup()` → 全9テストが `let repo = setup().await;` で呼び出し

### 変更内容

`audit_log_repository_test.rs` の `setup()` を以下のように変更:

```rust
use tokio::sync::OnceCell;

static TABLE_INITIALIZED: OnceCell<()> = OnceCell::const_new();

async fn setup() -> DynamoDbAuditLogRepository {
    let client = dynamodb::create_client(&dynamodb_endpoint()).await;
    TABLE_INITIALIZED
        .get_or_init(|| {
            let client = &client;
            async move {
                dynamodb::ensure_audit_log_table(client, TEST_TABLE_NAME)
                    .await
                    .expect("テーブルのセットアップに失敗");
            }
        })
        .await;
    DynamoDbAuditLogRepository::new(client, TEST_TABLE_NAME.to_string())
}
```

戻り値の型は `DynamoDbAuditLogRepository` のまま変更なし。各テストのコードも変更不要。

### テストリスト

ユニットテスト（該当なし — テストインフラの修正であり、テスト対象はテスト自体）

統合テスト:
- [x] 既存9テストが並列実行で全パス（3回連続成功）
- [x] `dynamodb_test.rs` の3テストが並列実行で全パス
- [x] 全統合テスト（`cargo test --all-features --test '*'`）が並列実行で全パス

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: justfile の `--test-threads=1` 削除

### 確認事項

確認事項: なし（既知のパターンのみ）

### 変更内容

`justfile` の `test-rust-integration`:
```diff
 # Rust 統合テスト（DB 接続が必要）
-# 順次実行（DynamoDB Local への同時接続過多を防ぐ）
 test-rust-integration:
-    cd backend && cargo test {{ _cargo_q }} --all-features --test '*' -- --test-threads=1
+    cd backend && cargo test {{ _cargo_q }} --all-features --test '*'
 ```

### テストリスト

統合テスト:
- [x] `just test-rust-integration` が成功（並列実行）
- [x] 3回連続実行で安定

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `dynamodb_test.rs` の扱いが未定義 | 不完全なパス | 対象外に明記（各テストが独自テーブル名で分離済み） |
| 2回目 | `dynamodb_endpoint()` 重複の扱い | 既存手段の見落とし | DRY ルール（3回まで許容）により対象外と判断 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 変更対象が全て特定されている | OK | test ファイル1つ + justfile 1箇所。dynamodb_test.rs は不要と判断 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | コードスニペットで具体的に示した |
| 3 | 設計判断の完結性 | 全ての選択に理由がある | OK | OnceCell 選択理由、対象外の理由を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象: 2ファイル、対象外: 2ファイルを明記 |
| 5 | 技術的前提 | 前提が確認済み | OK | tokio "full" feature、Rust edition 2024、Send+Sync を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | Issue #540 の精査コメントと整合 |

## 検証手順

1. Phase 1 完了後: `cd backend && cargo test --all-features --test audit_log_repository_test` を3回実行
2. Phase 2 完了後: `just test-rust-integration` を3回実行
3. 最終確認: `just check-all` で全体通過
