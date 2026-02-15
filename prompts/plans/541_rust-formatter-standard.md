# Epic #467: jscpd クローン削減 + ファイルサイズ削減の Story 分割

## Context

Epic #467「`just check` / `check-all` の警告をゼロにする」の残タスク:
- jscpd クローン削減の Story 分割・Issue 化
- ファイルサイズ削減の Story 分割・Issue 化

現状: Rust 179 クローン（重複率 10.16%）、Elm 16 クローン（重複率 5.25%）、500行超ファイル 18 件（Rust 14 + Elm 4）。

クローンとファイルサイズは同じコンポーネントに集中しているため、**コンポーネント領域別**に Story を分割し、各 Story 内でクローン削減とファイルサイズ削減を同時に扱う。

## 方針

### Story 分割の観点

- コンポーネント領域ごとに Story を切る（クローン削減 + ファイルサイズ削減を統合）
- 各 Story は独立してマージ可能な単位
- 優先度: クローン数・ファイルサイズ・影響範囲で判断

### Issue 化の粒度

各 Story Issue に以下を記載する:
- 対象クローン一覧（jscpd 出力から抽出）
- 対象ファイルサイズ超過一覧
- 想定アプローチ（共通化 / 分割 / 例外許容）

## Story 一覧

### Story 2: Core Service ユースケース層のクローン削減・分割

対象:
- `usecase/task.rs`（796行）: 内部重複 + `workflow/command/comment.rs` との重複（約20クローン）
- `usecase/dashboard.rs`: task.rs / comment.rs との重複（約7クローン）
- `usecase/workflow/command/decision.rs`（1889行）: ファイルサイズ超過
- `usecase/workflow/command/lifecycle.rs`（1292行）: ファイルサイズ超過

優先度: 高（最大クローン集中地帯 + 最大ファイル）

### Story 3: BFF ハンドラ層のクローン削減

対象:
- `bff/handler/workflow/command.rs`（620行）: query.rs との重複（約15クローン）
- `bff/handler/workflow/query.rs`: command.rs との重複
- `bff/handler/user.rs`（609行）: role.rs / task.rs / dashboard.rs / audit_log.rs との共通パターン（約15クローン）
- `bff/handler/role.rs`: user.rs との重複
- `bff/handler/task.rs`, `dashboard.rs`, `audit_log.rs`: 共通レスポンス変換パターン

優先度: 高（クローン数多い）

### Story 4: Core Service ハンドラ層のクローン削減・分割

対象:
- `core-service/handler/auth.rs`（965行）: 内部重複 + role.rs との重複（約10クローン）
- `core-service/handler/role.rs`: auth.rs との重複
- `core-service/handler/workflow/command.rs`（600行）: 内部重複 + query.rs との重複（約10クローン）
- `core-service/handler/task.rs`: BFF types.rs との重複

優先度: 中

### Story 5: ドメイン層のクローン削減・分割

対象:
- `domain/workflow/step.rs`（809行）: 内部重複（約10クローン）
- `domain/workflow/instance.rs`（1003行）: ファイルサイズ超過
- `domain/workflow/definition.rs`, `comment.rs`: step.rs とのボイラープレート重複
- `domain/value_objects.rs`（650行）: user.rs との重複
- `domain/role.rs`（560行）, `domain/user.rs`（539行）

優先度: 中

### Story 6: リポジトリ・インフラ層のクローン削減

対象:
- `infra/repository/user_repository.rs`（782行）: 内部重複 + role_repository.rs との重複（約12クローン）
- `infra/repository/role_repository.rs`: user_repository.rs との重複
- `infra/repository/workflow_*_repository.rs`: 内部重複
- `infra/deletion/`: postgres_role / postgres_user / postgres_display_id / auth_credentials 間の重複（4クローン）
- `infra/session.rs`: 内部重複

優先度: 中

### Story 7: サービス間共通コード抽出

対象:
- Health Check ハンドラ: auth-service vs core-service（完全一致、36行）
- BFF client types.rs vs Core Service handler response types（約5クローン）
- `bff/middleware/authz.rs`: 内部重複
- `auth-service/handler/auth.rs` vs `core-service/handler/auth.rs`

優先度: 低（件数少ないが横断的）

### Story 8: テストコード共通化

対象:
- `auth_integration_test.rs`（934行）: 内部重複（約8クローン）
- `user_repository_test.rs`（698行）: 内部重複
- `workflow_step_repository_test.rs`, `rls_test.rs`（614行）, `postgres_deleter_test.rs`, `db_test.rs`, `session_test.rs`

優先度: 低（テストは構造レビュー閾値の例外対象だが、重複削減は保守性向上）

### Story 9: Elm フロントエンドのクローン削減・分割

対象:
- `Page/Role/Edit.elm`（484行）vs `Page/Role/New.elm`（378行）: フォーム重複（約6クローン）
- `Page/User/Edit.elm`（446行）vs `Page/User/New.elm`（458行）: フォーム重複（約5クローン）
- `Page/User/Detail.elm` vs `Page/User/List.elm`: 共通パターン
- Role pages vs User pages: 横断的重複
- `Page/Workflow/Detail.elm`（1367行）, `Main.elm`（1139行）, `Page/Workflow/New.elm`（1046行）: ファイルサイズ超過

優先度: 中（Elm の重複率は 5.25% で比較的低い）

### BFF `handler/auth.rs`（1071行）について

ファイルサイズ超過だが jscpd クローンは auth-service との重複のみ。Story 4（Core Service ハンドラ）または Story 7（サービス間共通コード）で対応するか、個別 Story にするか要検討。→ Story 4 に含める。

## 実施手順

1. 各 Story を GitHub Issue として作成（Epic #467 のサブ Issue）
2. Epic #467 のタスクリストに各 Story Issue のリンクを追加
3. Issue 本文に対象クローン・ファイルサイズ・想定アプローチを記載

## 検証

- Epic #467 のタスクリストに全 Story がリンクされていること
- 各 Story Issue に対象一覧と想定アプローチが記載されていること
- jscpd 出力の全クローンがいずれかの Story にカバーされていること
