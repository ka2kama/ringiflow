# 2026-02-10 CoreServiceClient サブトレイト分割

Issue: #290 Refactor oversized files (500+ lines)

## 概要

`CoreServiceClient` トレイト（20 メソッド、1212 行）を ISP（Interface Segregation Principle）に基づき 3 つのサブトレイトに分割した。スーパートレイト + ブランケット impl で後方互換性を維持しつつ、テストスタブのボイラープレートを 303 行削除した。

## 背景と目的

Issue #290 の残り対応として、500 行超過ファイルの分析を行った。前セッション（ADR-039）でワークフローモジュールの分割は完了済み。

残り 5 ファイルを分析した結果、行数超過の原因はファイルごとに異なることが判明:

- `core_service.rs`（1212 行）: ISP 違反 → **分割対象**
- `auth.rs`（1135 行）: ISP の症状（スタブ肥大化）→ core_service 分割で改善
- `task.rs`（1042 行）: 実装 225 行 + テスト 817 行 → 例外許容
- `New.elm`（1115 行）: TEA パターン → 例外許容（要検討）
- `Main.elm`（832 行）: SPA エントリポイント → 例外許容

## 実施内容

### Phase 1: 共有型の切り出し

`core_service.rs` から `error.rs`（CoreServiceError）と `types.rs`（全 DTO 型）を `core_service/` サブモジュールとして切り出し。親モジュール化して `pub use *` で re-export。

### Phase 2: サブトレイト定義 + impl 分割

3 つのサブトレイトを作成:

| サブトレイト | メソッド数 |
|------------|----------|
| `CoreServiceUserClient` | 3 |
| `CoreServiceWorkflowClient` | 12 |
| `CoreServiceTaskClient` | 4 |

`CoreServiceClient` をスーパートレイト + ブランケット impl に変更。既存テストスタブを 3 つの impl に分割（この時点では `unimplemented!()` は残存）。

### Phase 3: AuthState 型変更 + テストスタブ簡素化

`AuthState.core_service_client` を `Arc<dyn CoreServiceClient>` → `Arc<dyn CoreServiceUserClient>` に型変更。`main.rs` で具象型 `Arc<CoreServiceClientImpl>` を保持し、各 State への注入時に unsizing coercion で変換。

auth テスト/統合テストから `CoreServiceWorkflowClient` と `CoreServiceTaskClient` の impl を完全削除（303 行削減）。

### Phase 4: ADR-041 + structural-review.md 改善

ADR-041 でサブトレイト分割パターンを記録。structural-review.md の閾値基準を「行数超過 → 責務分析 → 判断」に改善。

## 設計上の判断

### 閾値基準の改善

500 行超過 = 分割という機械的な基準から、「行数超過は検討トリガー、判断は責務分析に基づく」方式に変更。テスト込みで高凝集なファイル（task.rs）やアーキテクチャパターンの帰結（TEA ページ）は例外として許容することとした。

### サブトレイトの粒度

20 メソッドを個別トレイト（過剰）でも 2 分割（不十分）でもなく、責務単位（User/Workflow/Task）の 3 分割を採用。Core Service の内部 API のリソース境界と一致している。

### 具象型保持パターン

`main.rs` で `Arc<CoreServiceClientImpl>` を具象型のまま保持する方式を採用。Rust では `Arc<dyn TraitA>` → `Arc<dyn TraitB>` の変換が不可能なため、具象型を起点に各 State のフィールド型への unsizing coercion を行う。

## 判断ログ

| 種別 | 判断 | 背景 |
|------|------|------|
| 発見 | 統合テストで `CoreServiceClient` の明示的 import が不要 | Rust の unsizing coercion は構造体フィールドの型注釈から推論される |
| ルール適用 | `CoreServiceClientImpl` のフィールドを `pub(super)` に変更 | サブモジュールからのアクセスに必要。計画のブラッシュアップループで事前に検出済み |

## 成果物

### コミット

| コミット | 内容 |
|---------|------|
| `b5892b7` | Phase 1: error.rs と types.rs のサブモジュール化 |
| `7b41367` | Phase 2: 3 サブトレイト分割 + スーパートレイト化 |
| `7f662f3` | Phase 3: AuthState 型変更 + テストスタブ簡素化 |
| `44bbb20` | Phase 4: ADR-041 + structural-review.md 改善 |

### 作成・更新ファイル

新規:
- `backend/apps/bff/src/client/core_service/client_impl.rs`
- `backend/apps/bff/src/client/core_service/error.rs`
- `backend/apps/bff/src/client/core_service/types.rs`
- `backend/apps/bff/src/client/core_service/user_client.rs`
- `backend/apps/bff/src/client/core_service/workflow_client.rs`
- `backend/apps/bff/src/client/core_service/task_client.rs`
- `docs/70_ADR/041_CoreServiceClientのサブトレイト分割.md`

更新:
- `backend/apps/bff/src/client/core_service.rs`（1212 行 → 約 30 行）
- `backend/apps/bff/src/handler/auth.rs`（1135 行 → 985 行、-150 行）
- `backend/apps/bff/tests/auth_integration_test.rs`（994 行 → 841 行、-153 行）
- `backend/apps/bff/src/main.rs`（具象型保持パターンに変更）
- `.claude/rules/structural-review.md`（閾値基準の改善）

## 議論の経緯

### 500 行閾値の妥当性

ユーザーから「行数超過の原因がファイルごとに異なるのに一律分割は不適切ではないか」という指摘があり、残り 5 ファイルの責務分析を実施。結果、core_service.rs のみが ISP 違反で分割が正当、他は例外として許容する判断に至った。structural-review.md の閾値基準を「検討トリガー → 責務分析 → 判断」に改善。

## 学んだこと

- Rust のブランケット impl（`impl<T> SuperTrait for T where T: Sub1 + Sub2 + Sub3 {}`）はサブトレイト分割のコストを最小化する。既存の `dyn SuperTrait` 利用箇所は変更不要
- 行数超過はプロキシメトリクスであり、真の問題は行数ではなく責務混在や ISP 違反。閾値は「分割命令」ではなく「検討トリガー」として運用すべき
- `Arc<dyn TraitA>` → `Arc<dyn TraitB>` の変換は Rust では不可能。複数のトレイトオブジェクト型が必要な場合、具象型を保持して各 coercion ポイントで変換するパターンが有効
