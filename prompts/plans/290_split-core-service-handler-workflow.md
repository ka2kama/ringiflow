# Issue #290: core-service/handler/workflow.rs CQRS 分割

## Context

ADR-043 で確定した分割戦略に基づき、`backend/apps/core-service/src/handler/workflow.rs`（780行、全てプロダクションコード）を CQRS パターンで分割する。usecase 層（ADR-039）で確立された command.rs / query.rs の分割軸を handler 層にも統一適用する。

## スコープ

### 対象
- `backend/apps/core-service/src/handler/workflow.rs` → ディレクトリモジュール化

### 対象外
- `backend/apps/bff/src/handler/workflow.rs`（後続 PR）
- `frontend/src/Page/Workflow/New.elm`（後続 PR）
- テスト追加（純粋リファクタリングのためテストなし。既存のハンドラテストは handler 層テスト戦略 ADR-036 により API テスト（Hurl）でカバー）

## 分割方針

### 分割後の構造

```
handler/
├── workflow.rs          # 親モジュール（型定義 + mod + re-export）
└── workflow/
    ├── command.rs       # 状態変更系ハンドラ（7個）
    └── query.rs         # 読み取り系ハンドラ（5個）
```

### ハンドラの分類

**command.rs（POST ハンドラ: 7個）:**
- `create_workflow` (POST /internal/workflows)
- `submit_workflow` (POST /internal/workflows/{id}/submit)
- `approve_step` (POST .../{step_id}/approve)
- `reject_step` (POST .../{step_id}/reject)
- `submit_workflow_by_display_number` (POST .../by-display-number/{dn}/submit)
- `approve_step_by_display_number` (POST .../by-display-number/{dn}/.../approve)
- `reject_step_by_display_number` (POST .../by-display-number/{dn}/.../reject)

**query.rs（GET ハンドラ: 5個）:**
- `list_workflow_definitions` (GET /internal/workflow-definitions)
- `get_workflow_definition` (GET /internal/workflow-definitions/{id})
- `list_my_workflows` (GET /internal/workflows)
- `get_workflow` (GET /internal/workflows/{id})
- `get_workflow_by_display_number` (GET .../by-display-number/{dn})

### 親モジュール（workflow.rs）に残すもの

行1-292 の全型定義（~292行）:
- リクエスト型: `CreateWorkflowRequest`, `SubmitWorkflowRequest`, `ApproveRejectRequest`
- パスパラメータ型: `StepPathParams`, `StepByDisplayNumberPathParams`
- クエリパラメータ型: `TenantQuery`, `UserQuery`
- DTO 型: `UserRefDto`, `WorkflowDefinitionDto`, `WorkflowStepDto`, `WorkflowInstanceDto`
- ユーティリティ: `to_user_ref()`
- State: `WorkflowState`
- mod 宣言 + pub use re-export

### 外部参照の維持

`dashboard.rs` が `crate::handler::workflow::UserQuery` を参照している。型は親モジュールに残るため、パスは変更なし。

### Visibility の考慮

Rust のモジュールプライバシールールにより、子モジュール（command.rs, query.rs）は親モジュール（workflow.rs）の private アイテムにアクセス可能（descendent modules can access private items）。そのため、visibility 変更は不要。

## 確認事項

- 型: `WorkflowState`, DTO 型 → `handler/workflow.rs` 行44-292
- パターン: usecase/workflow.rs の親モジュール構造 → `usecase/workflow.rs`
- パターン: handler.rs の re-export → `handler.rs` 行25-39
- 外部参照: `UserQuery` の使用箇所 → `handler/dashboard.rs` 行18

## 実装手順

純粋リファクタリング（テストなし）のため、TDD サイクルは適用しない。

### Step 1: ディレクトリとファイルを作成

1. `backend/apps/core-service/src/handler/workflow/` ディレクトリ作成
2. `command.rs` と `query.rs` を作成

### Step 2: query.rs を作成

行383-399, 411-427, 438-464, 476-499, 607-634 のハンドラを移動。
必要な use 文を追加（`super::*` で親モジュールの型を参照）。

### Step 3: command.rs を作成

行303-332, 344-370, 513-547, 559-593, 646-670, 682-725, 737-780 のハンドラを移動。
必要な use 文を追加。

### Step 4: 親モジュール（workflow.rs）を更新

1. ハンドラ関数を削除（型定義のみ残す）
2. `mod command;` と `mod query;` を追加
3. `pub use command::*;` と `pub use query::*;` を追加
4. 不要になった use 文を整理

### Step 5: handler.rs の re-export を確認

現在の `pub use workflow::{...}` は変更不要（親モジュールの re-export 経由で解決される）。

### Step 6: 検証

```bash
just check-all
```

## 変更対象ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/core-service/src/handler/workflow.rs` | 修正（型定義のみ残す + mod + re-export） |
| `backend/apps/core-service/src/handler/workflow/command.rs` | 新規作成 |
| `backend/apps/core-service/src/handler/workflow/query.rs` | 新規作成 |

**変更しないファイル:**
- `handler.rs` — re-export はそのまま動作
- `main.rs` — ルーティングは変更なし
- `dashboard.rs` — `UserQuery` のパスは維持

## 推定行数

| ファイル | 推定行数 | 内訳 |
|---------|---------|------|
| workflow.rs (親) | ~310 | 型定義 292行 + mod/re-export ~18行 |
| command.rs | ~260 | 7 ハンドラ + use 文 |
| query.rs | ~220 | 5 ハンドラ + use 文 |

全ファイルが 500 行未満。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `dashboard.rs` が `handler::workflow::UserQuery` を参照 | 不完全なパス | 外部参照の維持セクションに記載。型は親モジュールに残るため影響なし |
| 2回目 | `WorkflowInstanceDto::from_instance` が private だが子モジュールからアクセス可能か | 技術的前提 | Rust のモジュールプライバシールールを確認。子モジュールは親の private アイテムにアクセス可能 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 12 ハンドラ全てを command/query に分類。型定義・ユーティリティも配置先を確認済み |
| 2 | 曖昧さ排除 | OK | 各ハンドラの分類は HTTP メソッド（GET vs POST）で機械的に決定。曖昧な判断なし |
| 3 | 設計判断の完結性 | OK | 型の配置（親モジュール）、visibility（変更不要）、外部参照（維持）の判断が完了 |
| 4 | スコープ境界 | OK | 対象: core-service handler のみ。bff handler と frontend は後続 PR |
| 5 | 技術的前提 | OK | Rust のモジュールプライバシールール（子は親の private にアクセス可能）を確認 |
| 6 | 既存ドキュメント整合 | OK | ADR-043 の方針（CQRS 分割）に準拠。usecase 層のパターンを踏襲 |
