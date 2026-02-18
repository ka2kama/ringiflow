# 表示用 ID: Phase B（WorkflowStep）

## 概要

Issue #208 の実装として、WorkflowStep に `STEP-N` 形式の表示用 ID を導入した。Phase A（WorkflowInstance）で確立したパターンをそのまま適用したフルスタック実装。

## 背景と目的

表示用 ID システムの Phase B。Phase A で WorkflowInstance に導入した `WF-N` 形式の表示用 ID を、WorkflowStep にも展開する。ユーザーがステップを識別しやすくするため。

## 実施内容

Phase A と同じ 5 フェーズ構成で実装:

| Phase | 内容 | ファイル |
|-------|------|---------|
| 1 | DB スキーマ変更 | `migrations/20260205000001_*.sql`, `20260205000002_*.sql` |
| 2 | ドメインモデル | `domain/src/workflow.rs` |
| 3 | インフラ層 | `infra/src/repository/workflow_step_repository.rs` |
| 4 | ユースケース層 | `core-service/src/usecase/workflow.rs` |
| 5 | API + フロントエンド | DTO, OpenAPI, Elm 型/デコーダー/UI |

## 成果物

### コミット

- `#208 Implement display_id for WorkflowStep (Phase B)`

### 作成/更新ファイル

- `backend/migrations/20260205000001_add_display_number_to_workflow_steps.sql`
- `backend/migrations/20260205000002_set_workflow_steps_display_number_not_null.sql`
- `backend/crates/domain/src/workflow.rs`
- `backend/crates/infra/src/repository/workflow_step_repository.rs`
- `backend/crates/infra/tests/workflow_step_repository_test.rs`
- `backend/apps/core-service/src/usecase/workflow.rs`
- `backend/apps/core-service/src/usecase/dashboard.rs`（テスト修正）
- `backend/apps/core-service/src/usecase/task.rs`（テスト修正）
- `backend/apps/core-service/src/handler/workflow.rs`
- `backend/apps/bff/src/handler/workflow.rs`
- `backend/apps/bff/src/client/core_service.rs`
- `openapi/openapi.yaml`
- `frontend/src/Data/WorkflowInstance.elm`
- `frontend/src/Page/Workflow/Detail.elm`

### PR

- #240: `#208 Implement display_id for WorkflowStep (Phase B)`

## 議論の経緯

特になし。Phase A で確立済みのパターンを適用したため、設計判断は発生しなかった。

## 発見した問題

1. PR を `--draft` オプションなしで作成した
   → 改善記録: [PRをDraftで作成しなかった](../../../process/improvements/2026-02/2026-02-05_2104_PRをDraftで作成しなかった.md)

2. ユーザー確認なしに PR を Ready にしようとした
   → 改善記録: [ユーザー確認なしにPRをReadyにした](../../../process/improvements/2026-02/2026-02-05_2104_ユーザー確認なしにPRをReadyにした.md)
   → Issue #242 で恒久対策（PR 完了フローを手順書に明示）

## 学んだこと

- Phase A で確立したパターン（表示用 ID）は、別エンティティにも効率的に適用できた
- 横断検証（全レイヤーのデータフロー確認）は Silent Failure 防止に有効

## 次のステップ

- auto review のコメントを確認し、必要に応じて対応
- Phase C（User）への展開は Issue #203 で対応予定
