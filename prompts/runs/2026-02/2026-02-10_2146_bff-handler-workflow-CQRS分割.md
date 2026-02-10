# bff handler/workflow.rs CQRS 分割

## 概要

ADR-043 で確定した分割戦略に基づき、`bff/handler/workflow.rs`（754行、全てプロダクションコード）を CQRS パターンでディレクトリモジュール化した。PR #385（core-service handler）と同一パターンの適用。

## 実施内容

1. `bff/handler/workflow.rs` の構造分析: POST 4個（command）、GET 5個（query）に分類
2. `workflow/command.rs` 作成: 4 個の POST ハンドラを移動
3. `workflow/query.rs` 作成: 5 個の GET ハンドラを移動
4. `workflow.rs`（親モジュール）更新: ハンドラ関数を削除し、型定義・DTO のみ残す
5. `just check-all` で検証（全通過）

## 判断ログ

- `get_task_by_display_numbers` の `super::task::TaskDetailData` 参照を `crate::handler::task::TaskDetailData` に変更した。ディレクトリモジュール化により `super` の意味が変わるため、絶対パスで参照

## 成果物

### コミット

- `#290 Split bff handler/workflow.rs into CQRS modules`

### 作成・更新ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/bff/src/handler/workflow.rs` | 修正（型定義のみ残す + mod + re-export、202行） |
| `backend/apps/bff/src/handler/workflow/command.rs` | 新規作成（296行） |
| `backend/apps/bff/src/handler/workflow/query.rs` | 新規作成（291行） |

### PR

- [#386](https://github.com/ka2kama/ringiflow/pull/386) Split bff handler/workflow.rs into CQRS modules（Draft）
