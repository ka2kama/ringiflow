# #559 デプロイ時のデータベースマイグレーション自動化

## Context

Lightsail 環境でコメント取得 API が 500 エラーを返す事象が発生した。原因は `workflow_comments` テーブルのマイグレーションが Lightsail DB に未適用だったこと。現在のデプロイパイプラインにはマイグレーション実行ステップがなく、手動運用に依存している。

## 方針: アプリケーション起動時にマイグレーションを実行

Issue の案 2 を採用する。`sqlx::migrate!()` マクロでマイグレーション SQL をバイナリに埋め込み、起動時に自動実行する。

案 1（デプロイスクリプト変更）を見送る理由:
- 2 つのデプロイパス（GitHub Actions / ローカル）の両方を変更する必要がある
- sqlx-cli のインストール待ちが発生する
- マイグレーションとアプリケーションコードの同期が保証されない

案 2 の利点:
- マイグレーションがバイナリに埋め込まれるため、コードとスキーマが常に同期
- sqlx が advisory lock で並行実行を制御するため、複数サービスから呼んでも安全
- デプロイスクリプトの変更不要
- 適用済みマイグレーションはスキップされるため冪等

## 対象

- `backend/crates/infra/src/db.rs` — `run_migrations()` 関数を追加
- `backend/apps/core-service/src/main.rs` — プール作成後にマイグレーション呼び出し
- `backend/apps/auth-service/src/main.rs` — 同上
- `backend/Dockerfile` — Builder ステージに `migrations/` をコピー

## 対象外

- デプロイスクリプト（`scripts/deploy-lightsail.sh`, `infra/lightsail/deploy.sh`）の変更
- ロールバック戦略（現時点では単一インスタンスのデモ環境のため不要）
- `ringiflow_app` ロールへの切り替え（別 Issue のスコープ）

## Phase 1: infra crate にマイグレーション関数を追加

### 確認事項

- [x] パターン: `db::create_pool` の使用パターン → Core Service `main.rs` L170-173, Auth Service `main.rs` L100-103
- [x] ライブラリ: `sqlx::migrate!()` マクロ — `Cargo.toml` で `migrate` フィーチャ有効済み

### 変更内容

`backend/crates/infra/src/db.rs` に追加:

```rust
/// データベースマイグレーションを実行する
///
/// `sqlx::migrate!()` マクロで埋め込まれたマイグレーションファイルを
/// 順番に適用する。適用済みのマイグレーションはスキップされる。
///
/// sqlx が PostgreSQL の advisory lock を使用するため、
/// 複数プロセスから同時に呼び出しても安全。
pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}
```

### テストリスト

ユニットテスト（該当なし — `sqlx::migrate!()` はマクロで、実行には DB 接続が必要）

統合テスト（該当なし — 既存の `#[sqlx::test(migrations = ...)]` が同等のカバレッジを提供）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 2: Core Service / Auth Service で呼び出し

### 確認事項

確認事項: なし（Phase 1 で確認済み）

### 変更内容

両サービスの `main()` でプール作成直後に追加:

```rust
// マイグレーション実行
db::run_migrations(&pool)
    .await
    .expect("マイグレーションの実行に失敗しました");
tracing::info!("マイグレーションを適用しました");
```

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 3: Dockerfile に migrations/ を追加

### 確認事項

確認事項: なし（既知のパターンのみ）

### 変更内容

Builder ステージ（Stage 3）でソースコードコピーの後に追加:

```dockerfile
# マイグレーションファイルをコピー（sqlx::migrate! マクロがコンパイル時に埋め込む）
COPY migrations/ migrations/
```

Planner ステージ（Stage 2）への追加は不要（cargo-chef はソースコードの依存解析のみで、マイグレーションファイルは対象外）。

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## 検証方法

1. `just check-all` が通過すること
2. `just dev-deps && just dev-core-service` で Core Service 起動時にマイグレーションログが出力されること
3. `just dev-auth-service` で Auth Service 起動時にマイグレーションログが出力されること

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Dockerfile に migrations/ が含まれていない | 技術的前提 | Phase 3 を追加。`sqlx::migrate!()` はコンパイル時にファイルを埋め込むため、Builder ステージで必要 |
| 2回目 | テストリストに全層の記載がない | テストピラミッド | 各 Phase に全テスト層を明記（該当なし含む） |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Core Service, Auth Service, Dockerfile の 3 箇所を網羅。BFF は DB 未使用のため対象外 |
| 2 | 曖昧さ排除 | OK | 各 Phase の変更内容がコードスニペットで一意に確定 |
| 3 | 設計判断の完結性 | OK | 案 1 vs 案 2 の判断理由を記載。`infra` crate に配置する理由（DRY + 両サービスが依存済み） |
| 4 | スコープ境界 | OK | 対象・対象外を明記 |
| 5 | 技術的前提 | OK | `sqlx::migrate!()` のパス解決（CARGO_MANIFEST_DIR 相対）、advisory lock による並行安全性を確認 |
| 6 | 既存ドキュメント整合 | OK | Issue #559 の案 2 に沿った実装 |
