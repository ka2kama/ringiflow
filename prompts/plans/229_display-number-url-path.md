# Issue #229 実装計画: 公開 API の URL パスパラメータに表示用番号を使用する

## 概要

BFF の公開 API において、URL パスパラメータを UUID から display_number（整数）に変更する。

```
Before: /api/v1/workflows/01924f3e-7a8b-7000-8000-000000000001
After:  /api/v1/workflows/42
```

## 設計判断: ID 解決アプローチ

### 検討した代替案

| 方案 | 概要 | 利点 | 欠点 |
|------|------|------|------|
| A: 2段階呼び出し | BFF が「解決API」→「既存API」を順次呼び出し | 既存 API を変更しない | 2回の呼び出し、遅延、複雑化 |
| B: 既存 API を両対応 | UUID と display_number のどちらでも受け付ける | 1回の呼び出し | 型安全性低下（実行時判定） |
| **C: 新規エンドポイント** | display_number 専用のエンドポイントを追加 | 1回の呼び出し、型安全 | エンドポイント数が増える |
| D: クエリパラメータ拡張 | `?display_number={dn}` で検索 | 既存パターンと一貫 | REST セマンティクス的に微妙 |

### 採用: 方案 C（新規エンドポイント）

**理由:**
1. **1回の呼び出しで完結** - パフォーマンス最適、BFF ↔ Core Service 間の往復を削減
2. **型安全性を維持** - パスパラメータは整数型として静的に検証可能
3. **既存 API は変更しない** - 後方互換性を保持、既存の UUID ベース API はそのまま
4. **責務が明確** - 「display_number で操作」という新しいパス体系として分離

**却下理由:**
- **方案 A**: 2回の HTTP 呼び出しはパフォーマンス劣化。特に承認/却下では顕著
- **方案 B**: `Path<String>` で受けて実行時に UUID か整数かを判定するのは型安全性を損なう
- **方案 D**: リソース識別はパスパラメータの領分。`/workflows?display_number=42` は一覧検索のセマンティクス

## 対象エンドポイント

### BFF（公開 API）

| 変更前 | 変更後 |
|--------|--------|
| `GET /api/v1/workflows/{id}` | `GET /api/v1/workflows/{display_number}` |
| `POST /api/v1/workflows/{id}/submit` | `POST /api/v1/workflows/{display_number}/submit` |
| `POST /api/v1/workflows/{id}/steps/{step_id}/approve` | `POST /api/v1/workflows/{display_number}/steps/{step_display_number}/approve` |
| `POST /api/v1/workflows/{id}/steps/{step_id}/reject` | `POST /api/v1/workflows/{display_number}/steps/{step_display_number}/reject` |

### Core Service（内部 API）- 新規追加

| 新規エンドポイント | 用途 |
|-------------------|------|
| `GET /internal/workflows/by-display-number/{dn}` | ワークフロー詳細取得 |
| `POST /internal/workflows/by-display-number/{dn}/submit` | ワークフロー申請 |
| `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/approve` | ステップ承認 |
| `POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/reject` | ステップ却下 |

### 対象外（変更しない）

- `GET /api/v1/workflow-definitions/{id}` - ワークフロー定義は UUID のまま
- `GET /api/v1/tasks/{id}` - タスクは Issue の範囲外
- Core Service の既存 UUID ベース API - 後方互換性のため維持

---

## Phase 分割

```
Phase 1: リポジトリ層 (find_by_display_number)
    ↓
Phase 2: Core Service ユースケース・ハンドラ（新規エンドポイント追加）
    ↓
Phase 3: BFF クライアント・ハンドラ
    ↓
Phase 4: フロントエンド
    ↓
Phase 5: OpenAPI + API テスト
```

---

## Phase 1: リポジトリ層

### 目的

display_number で WorkflowInstance / WorkflowStep を検索するメソッドを追加。

### 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `backend/crates/infra/src/repository/workflow_instance_repository.rs` | `find_by_display_number` メソッド追加 |
| `backend/crates/infra/src/repository/workflow_step_repository.rs` | `find_by_display_number` メソッド追加 |
| `backend/crates/infra/tests/workflow_instance_repository_test.rs` | 統合テスト追加 |
| `backend/crates/infra/tests/workflow_step_repository_test.rs` | 統合テスト追加 |

### 新規メソッド仕様

```rust
// WorkflowInstanceRepository
async fn find_by_display_number(
    &self,
    display_number: DisplayNumber,
    tenant_id: &TenantId,
) -> Result<Option<WorkflowInstance>, InfraError>;

// WorkflowStepRepository
async fn find_by_display_number(
    &self,
    display_number: DisplayNumber,
    instance_id: &WorkflowInstanceId,
    tenant_id: &TenantId,
) -> Result<Option<WorkflowStep>, InfraError>;
```

### テストリスト

**WorkflowInstanceRepository::find_by_display_number**

- [ ] 存在する display_number で検索できる（正常系）
- [ ] 存在しない display_number で None を返す（異常系）
- [ ] 別テナントの display_number では見つからない（テナント分離）

**WorkflowStepRepository::find_by_display_number**

- [ ] 存在する display_number で検索できる（正常系）
- [ ] 存在しない display_number で None を返す（異常系）
- [ ] 別の instance_id では見つからない（スコープ分離）

**境界値について:** `DisplayNumber` の値オブジェクトテスト（domain 層）で 0 以下を拒否することを検証済み。リポジトリテストでは有効な値のみ使用。

---

## Phase 2: Core Service ユースケース・ハンドラ

### 目的

display_number でワークフローを操作する新規内部 API を追加。
**1回の呼び出しで処理を完結**させる（ID 解決 + 操作を一体化）。

### 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `backend/apps/core-service/src/usecase/workflow.rs` | 既存ユースケースを display_number 対応に拡張 |
| `backend/apps/core-service/src/handler/workflow.rs` | 新規エンドポイント追加（4つ） |
| `backend/apps/core-service/src/router.rs` | ルーティング追加 |

### 新規エンドポイント仕様

display_number でリソースを直接操作し、**ワークフロー詳細を返す**（UUID 解決 API ではない）。

```
GET /internal/workflows/by-display-number/{display_number}?tenant_id={tenant_id}
Response: { data: WorkflowInstanceDto }  # 既存の get_workflow と同じレスポンス
404: ワークフローインスタンスが見つからない

POST /internal/workflows/by-display-number/{display_number}/submit?tenant_id={tenant_id}
Response: { data: WorkflowInstanceDto }
404: ワークフローインスタンスが見つからない

POST /internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/approve
Response: { data: WorkflowInstanceDto }
404: ワークフローインスタンスまたはステップが見つからない

POST /internal/workflows/by-display-number/{display_number}/steps/by-display-number/{step_display_number}/reject
Response: { data: WorkflowInstanceDto }
404: ワークフローインスタンスまたはステップが見つからない
```

### 実装方針

既存ユースケースを再利用し、ID 解決部分のみ追加:

```rust
// 既存: get_workflow(id: WorkflowInstanceId, tenant_id: TenantId)
// 新規: get_workflow_by_display_number(display_number: DisplayNumber, tenant_id: TenantId)
//       → リポジトリで display_number → WorkflowInstance を取得
//       → 以降は既存ロジックを再利用
```

### テストリスト

**get_workflow_by_display_number**

- [ ] 存在する display_number でワークフロー詳細を返す（正常系）
- [ ] 存在しない display_number で 404（異常系）
- [ ] 0 や負の display_number で 400（境界値）

**submit_workflow_by_display_number**

- [ ] 存在する display_number で申請成功（正常系）
- [ ] 存在しない display_number で 404（異常系）

**approve_step_by_display_number**

- [ ] 存在する display_number でステップ承認成功（正常系）
- [ ] 存在しない workflow display_number で 404（異常系）
- [ ] 存在しない step display_number で 404（異常系）

**reject_step_by_display_number**

- [ ] 存在する display_number でステップ却下成功（正常系）
- [ ] 存在しない workflow display_number で 404（異常系）
- [ ] 存在しない step display_number で 404（異常系）

**既存テストとの関係:** ビジネスロジック（状態遷移、認可）は既存ユースケーステストで担保済み。新規テストは「display_number → エンティティ解決」の部分に集中。

---

## Phase 3: BFF クライアント・ハンドラ

### 目的

BFF の公開 API を display_number ベースに変更。
Core Service の新規エンドポイントを **1回呼び出すだけ** で処理完結。

### 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `backend/apps/bff/src/client/core_service.rs` | 新規メソッド追加（`get_workflow_by_display_number` 等） |
| `backend/apps/bff/src/handler/workflow.rs` | パスパラメータを `i64` に変更、新規クライアントメソッド呼び出し |
| `backend/apps/bff/src/router.rs` | ルーティングのパスパラメータ型を更新 |

### 変更前後の比較

```rust
// Before: UUID で Core Service を呼び出し
pub async fn get_workflow<C, S>(
    Path(workflow_id): Path<Uuid>,
    ...
) {
    state.core_service_client
        .get_workflow(workflow_id, tenant_id)
        .await
}

// After: display_number で新規エンドポイントを 1回呼び出し
pub async fn get_workflow<C, S>(
    Path(display_number): Path<i64>,
    ...
) {
    state.core_service_client
        .get_workflow_by_display_number(display_number, tenant_id)
        .await
    // ↑ Core Service が display_number → ワークフロー詳細を直接返す
}
```

### Core Service クライアント新規メソッド

```rust
// core_service.rs に追加
async fn get_workflow_by_display_number(&self, display_number: i64, tenant_id: Uuid)
    -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

async fn submit_workflow_by_display_number(&self, display_number: i64, req: SubmitWorkflowRequest)
    -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

async fn approve_step_by_display_number(&self, workflow_dn: i64, step_dn: i64, req: ApproveRejectRequest)
    -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;

async fn reject_step_by_display_number(&self, workflow_dn: i64, step_dn: i64, req: ApproveRejectRequest)
    -> Result<ApiResponse<WorkflowInstanceDto>, CoreServiceError>;
```

### エラーハンドリング

| 状況 | HTTP ステータス | エラータイプ |
|------|----------------|-------------|
| display_number が見つからない | 404 | workflow-instance-not-found |
| step_display_number が見つからない | 404 | step-not-found |

### テストリスト

- [ ] display_number で正常にワークフロー取得（正常系）
- [ ] 存在しない display_number で 404（異常系）
- [ ] 0 や負の display_number で 400（境界値 - axum が i64 を受け入れ、ハンドラで検証）
- [ ] display_number で申請成功（正常系）
- [ ] display_number + step_display_number で承認成功（正常系）
- [ ] display_number + step_display_number で却下成功（正常系）
- [ ] 存在しない step_display_number で 404（異常系）

**BFF ハンドラの境界値処理:**
- `Path<i64>` で受け取った値を `DisplayNumber::try_from()` で変換
- 0 以下の場合は `DomainError::Validation` → 400 Bad Request

---

## Phase 4: フロントエンド

### 目的

URL 構築で UUID から display_number を使用するように変更。

### 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `frontend/src/Data/WorkflowInstance.elm` | `displayNumber` フィールド追加（API レスポンスに追加） |
| `frontend/src/Data/WorkflowStep.elm` | `displayNumber` フィールド追加 |
| `frontend/src/Page/Workflow/List.elm` | リンク URL を `displayNumber` で構築 |
| `frontend/src/Page/Workflow/Detail.elm` | API 呼び出し・承認/却下で `displayNumber` を使用 |
| `frontend/src/Api/Workflow.elm` | URL 構築を `displayNumber` に変更 |
| `frontend/src/Route.elm` | `WorkflowDetail` の引数型を `Int` に変更 |

### 設計判断: display_number の取得方法

**API レスポンスに `display_number` フィールドを追加する。**

理由:
- `displayId` (例: "WF-42") からのパースは脆弱（フォーマット変更時に壊れる）
- API が Single Source of Truth として display_number を提供するのが堅牢
- Core Service は既に display_number を保持しているので追加は容易

### 追加変更（API レスポンス拡張）

| ファイル | 変更内容 |
|----------|----------|
| `backend/apps/core-service/src/handler/workflow.rs` | `WorkflowInstanceDto`, `WorkflowStepDto` に `display_number` 追加 |
| `backend/apps/bff/src/handler/workflow.rs` | `WorkflowData`, `WorkflowStepData` に `display_number` 追加 |
| `openapi/openapi.yaml` | レスポンススキーマに `display_number` フィールド追加 |

### テストリスト

- [ ] API レスポンスに display_number が含まれる
- [ ] 一覧から詳細へ遷移時に display_number が URL に使われる
- [ ] 詳細ページの API 呼び出しで display_number が使われる
- [ ] 承認/却下 API で display_number が使われる

---

## Phase 5: OpenAPI + API テスト

### 目的

仕様書とテストを更新し、変更を文書化。

### 変更対象ファイル

| ファイル | 変更内容 |
|----------|----------|
| `openapi/openapi.yaml` | パスパラメータの型を `integer` に変更 |
| `tests/api/hurl/workflow/*.hurl` | URL パスを display_number に更新 |

### OpenAPI 変更

```yaml
# Before
/api/v1/workflows/{id}:
  parameters:
    - name: id
      schema:
        type: string
        format: uuid

# After
/api/v1/workflows/{display_number}:
  parameters:
    - name: display_number
      schema:
        type: integer
        minimum: 1
```

### API テスト変更

```hurl
# Before
POST {{bff_url}}/api/v1/workflows/{{workflow_id}}/submit

# After
# 1. display_number を Capture
[Captures]
workflow_display_number: jsonpath "$.data.display_number"

# 2. display_number で API 呼び出し
POST {{bff_url}}/api/v1/workflows/{{workflow_display_number}}/submit
```

### テストリスト

- [ ] OpenAPI 仕様が更新されている
- [ ] Hurl テストが display_number で動作する
- [ ] E2E: 一覧 → 詳細 → 承認のフローが動作する

---

## 検証方法

1. **単体テスト**: `just check` で全テスト通過
2. **統合テスト**: `just test-rust-integration` でリポジトリテスト通過
3. **API テスト**: `just check-all` で Hurl テスト通過
4. **手動 E2E テスト**:
   - ワークフロー一覧から詳細画面に遷移（URL が `/workflows/42` 形式）
   - 詳細画面で承認/却下操作が成功

---

## 自己検証（設計・計画）

### Want（本質）

ユーザーが本当に望んでいること:
- **URL が人間可読であること** - UUID は長く、口頭伝達やチャット共有に不向き
- **UI と URL の一致** - ユーザーが画面で見る番号と URL が同じであること
- **エンタープライズツールの標準パターン** - GitHub `/issues/42`、Jira `/browse/PROJ-123` と同様

### To-Be（理想状態）

#### 外部品質

| 観点 | 理想状態 | 現在の設計 | 判定 |
|------|---------|-----------|------|
| 機能性 | `/workflows/42` でワークフロー詳細が表示される | BFF → Core Service（新規 API）→ リポジトリの経路で実現 | ✅ |
| 信頼性 | 存在しない display_number で適切なエラー | 404 + `workflow-instance-not-found` を返す設計 | ✅ |
| パフォーマンス | 追加の HTTP 往復なし、インデックス活用 | 1回の呼び出しで完結、`(tenant_id, display_number)` にユニーク制約あり | ✅ |
| セキュリティ | 連番推測が攻撃ベクトルにならない | テナント分離（tenant_id 必須）、認可は既存の仕組みで担保 | ✅ |

#### 内部品質

| 観点 | 理想状態 | 現在の設計 | 判定 |
|------|---------|-----------|------|
| 可読性 | 意図が明確、命名が一貫 | `/by-display-number/` で明示的、既存パターンに準拠 | ✅ |
| 保守性 | 変更が局所化、影響最小 | 既存 UUID API は変更なし、新規 API として追加 | ✅ |
| テスタビリティ | 各レイヤーで独立テスト可能 | リポジトリ、Core Service、BFF 各層でテストリストを定義 | ✅ |
| アーキテクチャ整合性 | レイヤー違反なし、型安全 | BFF → Core Service → リポジトリの依存方向を維持、`Path<i64>` で型安全 | ✅ |

#### ギャップ分析

| 理想との差分 | 許容判断 | 理由 |
|-------------|---------|------|
| Core Service のエンドポイント数が増える（4つ追加） | 許容 | 型安全性とパフォーマンスのトレードオフ。代替案（方案B: 両対応）は型安全性を損なう |
| フロントエンドで displayId → display_number への変換が必要 | 許容 | API レスポンスに display_number を追加することで解決。パースより堅牢 |

#### ギャップ: Issue 記載との差異

Issue #229 の記載:
> Core Service（内部 API）変更なし。引き続き UUID を使用。
> BFF ハンドラで display_number → UUID の解決を行い、Core Service を UUID で呼び出す。

現在の計画:
> Core Service に display_number 対応の新規 API を追加。BFF は 1 回の呼び出しで完結。

**差異の理由:**
1. Issue の「影響範囲」セクションには「Core Service | 表示用番号からエンティティを検索する API の追加」と記載されており、API 追加は想定内
2. 2 段階呼び出し（BFF で解決 → 既存 API）はパフォーマンス劣化を招く
3. 現在の計画は Issue の意図（人間可読な URL）を満たしつつ、実装を最適化

**対応:**
- 実装着手時に Issue の「変更内容」セクションを更新し、Core Service への API 追加を明記

#### E2E 完了基準（Issue に追記予定）

Issue #229 には E2E 視点の完了基準が欠如。以下を追加する:

- [ ] ワークフロー一覧から詳細画面に遷移できる（URL が `/workflows/42` 形式）
- [ ] 詳細画面で承認操作を完了できる
- [ ] 詳細画面で却下操作を完了できる
- [ ] 存在しない display_number でアクセス時、適切なエラー画面が表示される

#### 見落としリスクの検討

| リスク | 対策 |
|--------|------|
| Phase 間でデータフロー不整合 | Phase 4 で API レスポンスに display_number を追加、横断検証で確認 |
| OpenAPI と実装の乖離 | Phase 5 で仕様書更新、Hurl テストで検証 |
| 既存機能への退行 | 既存 UUID API は変更なし、テストで確認 |
| Issue 記載との差異による混乱 | 実装着手時に Issue を更新 |

---

### チェックリスト検証

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 下記詳細参照 |
| 2 | 曖昧さ排除 | OK | 下記詳細参照 |
| 3 | 設計判断の完結性 | OK | 下記詳細参照 |
| 4 | スコープ境界 | OK | 下記詳細参照 |
| 5 | 技術的前提 | OK | 下記詳細参照 |
| 6 | 既存ドキュメント整合 | OK | 下記詳細参照 |

### 1. 網羅性

**確認した内容:**
- BFF 公開 API: Explore エージェントで全エンドポイントを調査 → 対象 4 件を特定
- Core Service: 既存 API パターン（クエリパラメータ、パスパラメータ）を調査
- フロントエンド: Route.elm、Page/Workflow/\*.elm、Api/Workflow.elm を調査
- 各 Phase の変更対象ファイルを網羅的に列挙

**対象外の明示:**
- `workflow-definitions/{id}` - ワークフロー定義は内部管理用、display_id 不要
- `tasks/{id}` - Issue #229 のスコープ外として Issue に明記されている
- Core Service の既存 UUID ベース API - 後方互換性のため維持

### 2. 曖昧さ排除

**確定した設計判断:**
- ID 解決アプローチ: 方案 C（新規エンドポイント）を採用、4つの代替案を比較検討済み
- Core Service 新規 API: `/by-display-number/{dn}` 形式、レスポンスは既存 DTO と同一
- フロントエンド display_number 取得: API レスポンスに追加（displayId パースは却下）
- エラーハンドリング: 404 のみ（display_number は整数なので形式エラーは axum が処理）

**不確定要素なし:** 「あれば」「必要に応じて」等の記述を排除

### 3. 設計判断の完結性

**検討した設計判断と選択理由:**

| 判断ポイント | 選択肢 | 採用 | 理由 |
|-------------|--------|------|------|
| ID 解決アプローチ | A:2段階呼び出し, B:両対応, C:新規EP, D:クエリパラメータ | C | パフォーマンス（1回呼び出し）+ 型安全性 |
| display_number 取得 | displayId パース vs API 追加 | API 追加 | Single Source of Truth、フォーマット変更耐性 |
| Core Service EP 命名 | `/dn/`, `/by-dn/`, `/by-display-number/` | `/by-display-number/` | 明示的、既存パターンとの区別 |

**却下理由も明記:** 各代替案の欠点を具体的に記載

### 4. スコープ境界

**対象:**
- BFF 公開 API: 4 エンドポイント
- Core Service 内部 API: 4 エンドポイント新規追加
- フロントエンド: Route、Page、Api モジュール
- OpenAPI: パス定義、レスポンススキーマ
- API テスト: Hurl ファイル

**対象外:**
- `GET /api/v1/workflow-definitions/{id}` - 理由: 管理者向け、display_id の必要性なし
- `GET /api/v1/tasks/{id}` - 理由: Issue #229 のスコープ外
- Core Service 既存 UUID API - 理由: 後方互換性、内部ツール連携の可能性

### 5. 技術的前提

**確認した技術仕様:**
- axum `Path<i64>`: 整数パラメータの自動パース、形式エラーは 400 を返す
- sqlx `query!`: `display_number = $1` でインデックスを活用可能（ユニーク制約あり）
- Elm `int` パーサー: Route.elm で `Parser.int` を使用可能
- Hurl regex capture: `jsonpath "$.data.display_number"` で整数値を直接取得可能

**既存パターンとの整合:**
- Core Service API: クエリパラメータパターン（`?tenant_id=`）は検索条件用、パスパラメータはリソース識別用
- リポジトリ: `find_by_id`, `find_by_tenant` パターンに `find_by_display_number` を追加

### 6. 既存ドキュメント整合

**確認したドキュメント:**
- Issue #229: URL 変更の要件、対象エンドポイント、対象外の明記
- ADR-029: display_id 導入の意思決定、採番メカニズム
- ADR-023: BFF と Core Service の責務分離（BFF はリポジトリに直接アクセスしない）
- OpenAPI 仕様: 現在の UUID パスパラメータ定義

**矛盾なし:** 既存の設計方針（レイヤー分離、型安全性）に準拠
