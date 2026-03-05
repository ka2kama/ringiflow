# #1050 WorkflowInstanceDto を一覧用と詳細用に型分離する

## コンテキスト

### 目的
- Issue: #1050
- Want: API レスポンスの型がデータの意味を正確に表現すること。一覧 API で steps が不要であることを型レベルで明確にする
- 完了基準:
  - 一覧用 DTO（`WorkflowInstanceSummaryDto`）が `steps` フィールドを持たない
  - 詳細用 DTO（`WorkflowInstanceDetailDto`）が `steps` フィールドを持つ
  - 共通フィールドの重複が最小化されている（共通構造体の抽出、`From` 実装等）
  - 一覧 API のレスポンスに `steps` フィールドが含まれない
  - OpenAPI 仕様書が更新されている
  - 既存テストが通過する

### ブランチ / PR
- ブランチ: `feature/1050-split-workflow-instance-dto`
- PR: #1057（Draft）

### As-Is（探索結果の要約）

#### Core Service
- `WorkflowInstanceDto`（`backend/apps/core-service/src/handler/workflow.rs:226-242`）が一覧と詳細で共用
- `from_instance`（行246）: 一覧用、`steps: Vec::new()` というダミー値
- `from_workflow_with_steps`（行271）: 詳細用、実際の steps を設定
- `resolve_from_instance`（行304）: 一覧用ヘルパー。create, submit, cancel で使用
- `resolve_from_workflow_with_steps`（行314）: 詳細用ヘルパー。detail, approve, reject, resubmit 等で使用
- `task.rs:110`: `TaskDetailDto` が `WorkflowInstanceDto` を embed（detail 用途）

#### BFF Client
- `types.rs:151`: `WorkflowInstanceDto` に `#[serde(default)] steps` で両方を吸収
- `workflow_client.rs`: 全メソッドが `WorkflowInstanceDto` を返す

#### BFF Handler
- `WorkflowData`（`handler/workflow.rs:156`）: `steps: Vec<WorkflowStepData>` を持つ
- 一覧・詳細・全コマンドで同一型 `WorkflowData` を使用
- utoipa の `ToSchema` で OpenAPI スキーマが自動生成

#### OpenAPI
- `openapi/openapi.yaml` は `just openapi-generate` で BFF の utoipa アノテーションから自動生成（VCS 管理の生成物）

### 進捗
- [x] Phase 1: Core Service DTO 分離
- [x] Phase 2: BFF Client 型分離
- [x] Phase 3: BFF Handler 型分離 + OpenAPI 更新

## 仕様整理

### スコープ
- 対象:
  - Core Service の `WorkflowInstanceDto` を Summary / Detail に分離
  - BFF Client の `WorkflowInstanceDto` に Summary 型を追加
  - BFF Handler の `WorkflowData` に Summary 型を追加（一覧用）
  - OpenAPI 仕様（自動生成）
- 対象外:
  - Core Service コマンドハンドラの返り型変更（create/submit/cancel は現状 summary 相当だが、BFF との互換性を保つため今回は変更しない）
  - フロントエンド（Elm）の変更（`steps` が存在しなくなるだけで、Elm のデコーダは `#[serde(default)]` 相当の処理で対応済み）

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーがワークフロー一覧を取得すると、レスポンスに steps が含まれない | 正常系 | 既存テスト（API テスト / E2E） |
| 2 | ユーザーがワークフロー詳細を取得すると、レスポンスに steps が含まれる | 正常系 | 既存テスト |

## 設計

### 設計判断

| # | 判断 | 選択肢 | 選定理由 | 状態 |
|---|------|--------|---------|------|
| 1 | 共通フィールドの重複回避方法 | A: `#[serde(flatten)]` で基底構造体を embed / B: ヘルパー関数で共通マッピング | A: 型レベルで共通性を表現でき、フィールド追加時に基底のみ変更すればよい。serde の flatten は JSON では問題なく動作 | 確定 |
| 2 | BFF 側の対応範囲 | A: 一覧 API のみ変更 / B: コマンドも含め全変更 | A: 最小スコープ。コマンドの返り型変更は別 Issue でよい | 確定 |
| 3 | Core Service コマンドの返り型 | A: Summary/Detail に分ける / B: 全て DetailDto のまま | B: コマンドは `resolve_from_instance` と `resolve_from_workflow_with_steps` の2パターンがあるが、BFF 側は全て `WorkflowInstanceDto`（detail 相当）で受けている。変更すると BFF client trait の全メソッドに影響。今回は Core Service の list API のみ Summary を返す | 確定 |

### Phase 1: Core Service DTO 分離

#### 確認事項
- 型: `WorkflowInstanceDto` の全フィールド → `workflow.rs:226-242`
- 型: `WorkflowStepDto` → `workflow.rs:181-198`
- パターン: `#[serde(flatten)]` の既存使用 → Grep で確認
- パターン: `from_instance` / `from_workflow_with_steps` の使用箇所 → 探索済み

#### 変更内容

1. `WorkflowInstanceBaseDto` を新設（共通フィールド、Serialize 実装）
2. `WorkflowInstanceSummaryDto` = base のみ（`#[serde(flatten)]`）
3. `WorkflowInstanceDetailDto` = base + `steps: Vec<WorkflowStepDto>`（`#[serde(flatten)]`）
4. `from_instance` → `WorkflowInstanceSummaryDto` を返す（list API 用）
5. `from_workflow_with_steps` → `WorkflowInstanceDetailDto` を返す
6. `resolve_from_instance` → `WorkflowInstanceSummaryDto` を返す（ただし、command.rs からは `WorkflowInstanceDetailDto` 相当が必要 → 下記注記）
7. `resolve_from_workflow_with_steps` → `WorkflowInstanceDetailDto` を返す
8. `task.rs`: `TaskDetailDto` の `workflow` フィールドを `WorkflowInstanceDetailDto` に変更

注記: command.rs の create/submit/cancel は `resolve_from_instance` を使うが、BFF は `WorkflowInstanceDto`（detail 相当、steps optional）で受ける。Core Service 側で Summary を返しても BFF の `#[serde(default)]` で問題ない。ただし設計判断 #3 により、command.rs の返り型は今回変更しない。`resolve_from_instance` を Summary 型に変えると command.rs で型不一致になるため、list API の `from_instance` のみ Summary を返し、`resolve_from_instance` は `WorkflowInstanceDetailDto` のまま（steps は空 Vec）とする。

修正: `resolve_from_instance` は list 以外の command でも使用されるため、`WorkflowInstanceDetailDto`（steps 空）を返す。一覧 API（`query.rs`）のみ `WorkflowInstanceSummaryDto::from_instance` を直接使う。

#### テストリスト

ユニットテスト: 該当なし（DTO は Serialize のみで、ロジックなし）

ハンドラテスト: 該当なし（既存テストで網羅）

API テスト（Hurl）:
- [ ] 一覧 API のレスポンスに steps フィールドが含まれないことを確認（既存テストの更新で対応）

E2E テスト: 該当なし（既存テストで網羅）

### Phase 2: BFF Client 型分離

#### 確認事項
- 型: BFF `WorkflowInstanceDto` → `types.rs:151-168`
- パターン: `list_my_workflows` の返り型 → `workflow_client.rs:73`

#### 変更内容

1. `types.rs`: `WorkflowInstanceSummaryDto` を追加（steps なし）
2. `types.rs`: 既存 `WorkflowInstanceDto` は変更なし（detail + commands 用、`#[serde(default)]` で互換）
3. `workflow_client.rs`: `list_my_workflows` の返り型を `Vec<WorkflowInstanceSummaryDto>` に変更（trait + impl 両方）
4. `client.rs` のエクスポート更新

#### テストリスト

ユニットテスト: 該当なし
ハンドラテスト: 該当なし
API テスト: 該当なし（Phase 1 で網羅）
E2E テスト: 該当なし

### Phase 3: BFF Handler 型分離 + OpenAPI 更新

#### 確認事項
- 型: BFF `WorkflowData` → `handler/workflow.rs:156-172`
- パターン: `list_my_workflows` の BFF handler → `handler/workflow/query.rs:133`
- パターン: utoipa `ToSchema` の使用 → BFF handler 内の `#[derive(ToSchema)]`

#### 変更内容

1. `handler/workflow.rs`: `WorkflowSummaryData` を追加（steps なし、`ToSchema` 付き）
2. `WorkflowSummaryData` に `From<WorkflowInstanceSummaryDto>` を実装
3. `handler/workflow/query.rs`: `list_my_workflows` のレスポンスを `Vec<WorkflowSummaryData>` に変更
4. `handler/workflow/query.rs`: utoipa アノテーションのレスポンス型を更新
5. `just openapi-generate` で OpenAPI 仕様を再生成
6. `WorkflowData` は変更なし（detail / command は従来通り）
7. `authz_test.rs`: `list_my_workflows` の mock 返り型を更新

#### テストリスト

ユニットテスト: 該当なし
ハンドラテスト: 該当なし
API テスト: 該当なし（Phase 1 で網羅）
E2E テスト: 該当なし

## ブラッシュアップ

### ギャップ発見の観点 進行状態

| 観点 | 状態 | メモ |
|------|------|------|
| 未定義 | 完了 | 全型・全変換パスを探索済み |
| 曖昧 | 完了 | `resolve_from_instance` の扱いを明確化（command 用は DetailDto のまま） |
| 競合・エッジケース | 完了 | BFF `#[serde(default)]` で summary レスポンスの deserialize は互換 |
| 不完全なパス | 完了 | list → summary, detail/commands → detail で全パス網羅 |
| アーキテクチャ不整合 | 完了 | Core Service → BFF のレイヤー間データフロー確認済み |
| 責務の蓄積 | 完了 | 分離により DTO の責務が明確化 |
| 既存手段の見落とし | 完了 | `#[serde(flatten)]` で重複最小化 |
| 文脈依存のダミー値 | 完了 | これが Issue の本質。分離で解消 |
| テスト層網羅漏れ | 完了 | 既存テストで網羅。一覧 API の steps 不在は API テストで確認 |
| 操作パス網羅漏れ | 完了 | 操作パスは一覧/詳細の2つのみ |
| セキュリティ境界の欠落 | 完了 | DTO の分離のみ、認証・認可パスに変更なし |

### ループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `resolve_from_instance` が command.rs でも使用されている。Summary に変えると command の返り型に影響 | 不完全なパス | 設計判断 #3: command の返り型は変更しない。list API のみ Summary を返す |
| 2回目 | BFF `WorkflowData` の一覧レスポンスにも steps が含まれる | アーキテクチャ不整合 | Phase 3 で BFF handler にも `WorkflowSummaryData` を追加 |

### 未解決の問い
- なし

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Core Service DTO, BFF Client, BFF Handler, OpenAPI の4レイヤー全て網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 「必要に応じて」等の記述なし |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 共通フィールド重複回避、BFF 対応範囲、command 返り型の3判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外にフロントエンド変更、command 返り型変更を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `#[serde(flatten)]` の JSON 互換性、`#[serde(default)]` の後方互換を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 改善記録 #1050 の対策と一致 |
