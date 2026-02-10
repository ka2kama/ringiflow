# core-service handler/workflow.rs CQRS 分割

## 概要

ADR-043 で確定した分割戦略に基づき、`core-service/handler/workflow.rs`（780行、全てプロダクションコード）を CQRS パターンでディレクトリモジュール化した。usecase 層（ADR-039）で確立された command/query 分割軸を handler 層にも統一適用。

## 実施内容

1. 計画策定（plan mode）: ハンドラの分類（POST 7個 → command、GET 5個 → query）、型定義の配置、外部参照の維持を確認
2. `workflow/query.rs` 作成: 5 個の GET ハンドラを移動
3. `workflow/command.rs` 作成: 7 個の POST ハンドラを移動
4. `workflow.rs`（親モジュール）更新: ハンドラ関数を削除し、型定義・DTO のみ残す。`mod command; mod query;` + `pub use` re-export を追加。不要な use 文を整理
5. `just check-all` で検証（lint + unit test + API test 全通過）

## 判断ログ

- 特筆すべき判断なし（ADR-043 の方針を計画通りに適用した純粋リファクタリング）

## 成果物

### コミット

- `#290 Split core-service handler/workflow.rs into CQRS modules`

### 作成・更新ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/core-service/src/handler/workflow.rs` | 修正（型定義のみ残す + mod + re-export、274行） |
| `backend/apps/core-service/src/handler/workflow/command.rs` | 新規作成（350行） |
| `backend/apps/core-service/src/handler/workflow/query.rs` | 新規作成（187行） |

### PR

- [#385](https://github.com/ka2kama/ringiflow/pull/385) Split core-service handler/workflow.rs into CQRS modules（Draft）
