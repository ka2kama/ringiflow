# Issue #247: タスク API の display_number 対応

## 概要

Issue #247 の全 Phase を完了し、タスク詳細取得 API の URL パスパラメータを UUID から階層的な display_number 形式に変更した。また、自己検証プロトコルの実装フェーズへの適用を強化した。

## 背景と目的

Issue #229 で Workflow API を display_number ベースに移行済み。Task API も同様に対応することで、URL の一貫性を確保する。

タスク詳細取得 API の課題:
- WorkflowStep の display_number はワークフロー内でのみ一意（グローバル一意ではない）
- `GET /api/v1/tasks/{display_number}` では曖昧性が生じる

解決策:
```
Before: GET /api/v1/tasks/{uuid}
After:  GET /api/v1/workflows/{workflow_display_number}/tasks/{step_display_number}
```

## 実施内容

### Phase 構成

| Phase | 内容 |
|-------|------|
| 1 | Core Service: `get_task_by_display_numbers` ユースケースとハンドラ追加 |
| 2 | BFF: 新規エンドポイント追加、クライアント実装 |
| 3 | Frontend: Route を `TaskDetail String` → `TaskDetail Int Int` に変更、API 呼び出し更新 |
| 4 | OpenAPI 仕様更新、Hurl テスト追加 |

### 自己検証プロトコルの強化

実装中に自己検証が自動作動しなかった問題を特定し、改善記録を作成。

対策:
- `self-review.md` に「コミット前の必須確認」を明記
- TDD Refactor ステップとの統合を明確化

## 設計上の判断

### 階層的 URL 設計

グローバル一意でない ID を含む API では、親リソースをパスに含める階層的 URL が適切。

| パターン | 例 | 適用条件 |
|---------|-----|---------|
| フラット | `/tasks/{id}` | ID がグローバル一意 |
| 階層的 | `/workflows/{wf_id}/tasks/{step_id}` | ID が親スコープ内でのみ一意 |

GitHub の Issue/PR コメントも同様: `/repos/{owner}/{repo}/issues/{issue_number}/comments/{comment_id}`

### タスク一覧 API への display_number 追加

タスク一覧 API のレスポンスにも `display_number` フィールドを追加。リンク生成に必要。

```json
{
  "data": [{
    "id": "...",
    "display_number": 1,
    "workflow": {
      "display_number": 42
    }
  }]
}
```

## 成果物

### コミット

```
9876df2 #247 WIP: Use display_number in Task API URL path parameters
3072341 #247 Implement get_task_by_display_numbers in Core Service
180e902 #247 Implement task detail endpoint using display numbers in BFF
059a6b4 #247 Add display_number to task list response and update frontend routing
9806d2d #247 Update OpenAPI spec and add Hurl tests for task API
5561db6 #247 Fix Hurl test assertions for task API
9547139 #247 Add pre-commit check requirement to self-review protocol
```

### 変更ファイル（主要なもの）

| レイヤー | ファイル |
|---------|----------|
| Core Service | `handler/task.rs`, `usecase/task.rs` |
| BFF | `client/core_service.rs`, `handler/task.rs`, `handler/workflow.rs` |
| Frontend | `Route.elm`, `Api/Task.elm`, `Page/Task/*.elm`, `Data/Task.elm` |
| 仕様 | `openapi/openapi.yaml` |
| テスト | `tests/api/hurl/task/*.hurl` |
| プロセス | `.claude/rules/self-review.md`, 改善記録 |

## 議論の経緯

### 自己検証の自動実行欠如

ユーザーから「自己検証は充分？」と問われて初めて自己検証を実施。「コミット前に検証すること」という行動規範だけでは自動作動しないことが判明。

設計フェーズでは「計画ファイルに自己検証セクションを必須化」という成果物要件で構造的に強制していたが、実装フェーズには同様の構造がなかった。

対策として、self-review.md に「コミット前の必須確認」を明記し、TDD Refactor ステップとの統合を強調した。

## 学んだこと

1. **行動規範より成果物要件のほうが強制力が高い**: 「検証すること」ではなく「検証結果が記載されていること」を要件にすることで、検証の欠落を構造的に防げる

2. **Elm の型システムによる変更箇所自動検出**: `TaskDetail String` → `TaskDetail Int Int` への変更で、コンパイラが全ての関連箇所（パターンマッチ、リンク生成）を指摘

3. **階層的 URL は親子関係を明示する**: `/workflows/{n}/tasks/{m}` はタスクがワークフローに従属することを URL で表現

## 次のステップ

- PR #248 を Ready for Review にする
- Claude Code Action の自動レビュー対応
- マージ後、Issue #247 をクローズ
