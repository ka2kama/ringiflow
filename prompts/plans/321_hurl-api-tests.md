# 計画: #321 Hurl API テスト追加

## Context

API テスト突合表（#291 で作成）で特定された 9 件のギャップを解消する。
18 エンドポイントのうち 9 件が未カバー（50%）。すべてのカバーで 100% を達成する。

## 対象

| # | エンドポイント | 新規ファイル |
|---|---------------|-------------|
| 1 | `GET /health/ready` | `health_ready.hurl` |
| 2 | `GET /api/v1/workflow-definitions` | `workflow_definition/list_workflow_definitions.hurl` |
| 3 | `GET /api/v1/workflow-definitions/{id}` | `workflow_definition/get_workflow_definition.hurl` |
| 4 | `GET /api/v1/users` | `user/list_users.hurl` |
| 5 | `GET /api/v1/workflows` | `workflow/list_my_workflows.hurl` |
| 6 | `GET /api/v1/workflows/{dn}` | `workflow/get_workflow.hurl` |
| 7 | `GET /api/v1/dashboard/stats` | `dashboard/stats.hurl` |
| 8 | `POST .../approve` | `workflow/approve_step.hurl` |
| 9 | `POST .../reject` | `workflow/reject_step.hurl` |

## 対象外

- 既存テストの修正（壊れていなければ触らない）
- 各エンドポイントの `_unauthorized.hurl` 分離ファイル（認証ミドルウェアは共通であり、既存の auth テストで十分検証済み）
- OpenAPI 仕様の変更（既存仕様に対するテスト追加のみ）

## 設計判断

### Phase 順序: 複雑度順（単純→複雑）

Issue の優先度は高→低だが、実装順は複雑度順とする。

理由:
- 単純な GET テストでパターンを確立し、複雑なテストに適用する
- approve/reject は multi-user + CSRF + state change + 複数エラーケースで最も複雑
- Issue の優先度は「部分完了時にどこまで価値があるか」の指標。全件実装するため順序は実践面で決める

### テストデータ戦略: 作成パターン vs シードデータ参照

| 種類 | 戦略 | 理由 |
|------|------|------|
| 参照系 GET（定義一覧、ユーザー一覧） | シードデータ参照 | 状態を変更しないため安全 |
| ワークフロー GET（一覧、詳細） | テスト内で作成 | 他テストが作成したデータに依存しない |
| 状態変更（approve/reject） | テスト内で作成→申請→承認/却下 | 完全な E2E フローが必要 |
| ダッシュボード統計 | シードデータ参照 | glob 順で workflow/* より前に実行。カウントは `>= 0` で検証（テスト順序への脆弱性を避ける） |

### vars.env の拡張

`user_name=一般ユーザー` を追加。approve/reject テストで `assigned_to.name` を `==` で検証するため。
既存パターン: `admin_name=管理者` が既に定義済み。

---

## Phase 1: 参照系 GET エンドポイント（低優先度）

### 確認事項

- パターン: `health.hurl` の認証不要パターン → `tests/api/hurl/health.hurl`
- パターン: `create_workflow.hurl` の認証フロー → `tests/api/hurl/workflow/create_workflow.hurl`
- 型: OpenAPI `ReadinessResponse` スキーマ → `openapi/openapi.yaml`
- 型: OpenAPI `WorkflowDefinition` required フィールド → `openapi/openapi.yaml`
- 型: OpenAPI `UserItem` required フィールド → `openapi/openapi.yaml`

### 1-1. `health_ready.hurl`

```
GET /health/ready → 200
```

認証不要。`health.hurl` と同じ配置（ルート直下）。

テストシナリオ:
- [ ] 正常系: 全依存サービス稼働中 → 200, status == "ready"

アサーション:
```
jsonpath "$.status" == "ready"
jsonpath "$.checks.database" == "ok"
jsonpath "$.checks.redis" == "ok"
jsonpath "$.checks.core_api" == "ok"
```

### 1-2. `workflow_definition/list_workflow_definitions.hurl`

```
Login → GET /api/v1/workflow-definitions → 200
```

テストシナリオ:
- [ ] 正常系: シードデータの定義一覧を取得

アサーション（OpenAPI required + シードデータ値）:
```
data isCollection, count >= 1
data[0].id == "{{workflow_definition_id}}"
data[0].name == "汎用申請"
data[0].status == "Published"
data[0].version >= 1
data[0].definition exists （object 型、内部構造は additionalProperties）
data[0].created_by matches UUID
data[0].created_at, updated_at matches timestamp
```

### 1-3. `workflow_definition/get_workflow_definition.hurl`

```
Login → GET /api/v1/workflow-definitions/{id} → 200 / 404
```

テストシナリオ:
- [ ] 正常系: シードデータの定義を取得
- [ ] 異常系: 存在しない ID → 404

アサーション（正常系）: 1-2 と同じフィールド + `description` (nullable)

404:
```
type == "https://ringiflow.example.com/errors/workflow-definition-not-found"
status == 404
```

### 1-4. `user/list_users.hurl`

```
Login → GET /api/v1/users → 200
```

テストシナリオ:
- [ ] 正常系: テナント内ユーザー一覧を取得

アサーション（OpenAPI required + シードデータ値）:
```
data isCollection, count >= 2
data[*].id matches UUID
data[*].display_id matches "^USER-\\d+$"
data[*].display_number >= 1
data[*].name isString
data[*].email isString
```

注: シードデータに admin と user の 2 名が存在。順序は保証しないため個別値は `>=` / `isString` で検証。

---

## Phase 2: ワークフロー参照 + ダッシュボード（中優先度）

### 確認事項

- パターン: `create_workflow.hurl` のテスト内データ作成 → `tests/api/hurl/workflow/create_workflow.hurl`
- 型: OpenAPI `WorkflowInstance` required フィールド → `openapi/openapi.yaml`
- 型: OpenAPI `DashboardStats` required フィールド → `openapi/openapi.yaml`

### 2-1. `workflow/list_my_workflows.hurl`

```
Login → Create → GET /api/v1/workflows → 200
```

テストシナリオ:
- [ ] 正常系: 自分の申請一覧を取得

戦略: テスト内でワークフローを 1 件作成し、一覧に含まれることを確認。

アサーション:
```
data isCollection, count >= 1
data[*] の構造検証（WorkflowInstance required フィールド）
```

注: シードデータ + 他テストで作成されたワークフローも含まれるため、件数は `>= 1`。作成したワークフローの ID を capture し、一覧に含まれることは Hurl の制約上直接検証できないため、構造の正しさで検証する。

### 2-2. `workflow/get_workflow.hurl`

```
Login → Create → GET /api/v1/workflows/{dn} → 200 / 404
```

テストシナリオ:
- [ ] 正常系: 作成したワークフローの詳細を取得
- [ ] 異常系: 存在しない display_number → 404

戦略: テスト内で作成したワークフローを display_number で取得。Capture した値と `==` で突合。

アサーション（正常系）:
```
data.id == "{{workflow_id}}" （Capture → == パターン）
data.display_id == "{{workflow_display_id}}"
data.display_number == {{workflow_display_number}}
data.title == "経費申請"（リクエスト入力値）
data.status == "Draft"
data.version == 1
data.form_data.title == "交通費"
data.initiated_by.id == "{{admin_id}}"
data.initiated_by.name == "{{admin_name}}"
data.current_step_id == null
data.steps count == 0（Draft は steps なし）
data.submitted_at == null
data.completed_at == null
data.created_at, updated_at matches timestamp
```

404:
```
type == "https://ringiflow.example.com/errors/workflow-instance-not-found"
status == 404
```

### 2-3. `dashboard/stats.hurl`

```
Login → GET /api/v1/dashboard/stats → 200
```

テストシナリオ:
- [ ] 正常系: ダッシュボード統計を取得

アサーション（OpenAPI required + 型検証）:
```
data.pending_tasks >= 0
data.my_workflows_in_progress >= 0
data.completed_today >= 0
```

注: カウント値はテスト実行順序（glob 順で `dashboard/` は `task/` や `workflow/` より前）とシードデータの組み合わせで決まる。テスト順序への依存を避けるため `>= 0` で検証。整数型であること + 非負であることが重要。

---

## Phase 3: 承認・却下（高優先度）

### 確認事項

- パターン: `get_task_by_display_numbers.hurl` の multi-user フロー → `tests/api/hurl/task/get_task_by_display_numbers.hurl`
- パターン: `submit_workflow.hurl` のステップ capture パターン → `tests/api/hurl/workflow/submit_workflow.hurl`
- 型: OpenAPI `WorkflowStep` required フィールド → `openapi/openapi.yaml`
- 型: `ApproveRejectRequest` の構造 → `openapi/openapi.yaml`（version: integer, comment: optional string）
- ライブラリ: Hurl の `cookie` capture → Grep `cookie "session_id"` in existing tests

### 3-1. `workflow/approve_step.hurl`

```
Admin login → Create → Submit(assign user) → [Error cases] → User login → [Error cases] → User approve → 200
```

テストシナリオ:
- [ ] 正常系: 承認成功 → 200
- [ ] 異常系: CSRF トークンなし → 403
- [ ] 異常系: 権限なし（admin が user のステップを承認） → 403
- [ ] 異常系: バージョン不一致 → 409
- [ ] 異常系: 存在しないステップ → 404

テストフロー:

```
1. Admin login → capture admin_session_cookie
2. Get CSRF → capture csrf_token
3. Create workflow
4. Submit workflow (assigned_to: user)
5. Get workflow details → capture step_display_number, step_version

--- Error cases (admin session) ---
6. POST approve WITHOUT CSRF → 403 (csrf-validation-failed)
7. POST approve AS ADMIN (not assigned) → 403 (not-assigned)
8. POST approve NON-EXISTENT step → 404 (step-not-found)

--- Switch to user ---
9. User login → capture user_session_cookie
10. Get CSRF for user → capture user_csrf_token

--- Error case (user session) ---
11. POST approve WITH WRONG VERSION (999) → 409 (conflict)

--- Success case ---
12. POST approve WITH CORRECT VERSION → 200
```

アサーション（正常系 step 12）:

ワークフロー:
```
data.id == "{{workflow_id}}"
data.display_number == {{workflow_display_number}}
data.status == "Approved"
data.version == 3 （create:1, submit:2, approve:3）
data.completed_at matches timestamp
data.submitted_at matches timestamp
```

ステップ:
```
data.steps count == 1
data.steps[0].step_id == "approval"
data.steps[0].step_name == "承認"
data.steps[0].status == "Completed"
data.steps[0].version == 2 （created:1, approve:2）
data.steps[0].decision == "Approved"
data.steps[0].comment == "承認します"（リクエスト入力値）
data.steps[0].assigned_to.id == "{{user_id}}"
data.steps[0].assigned_to.name == "{{user_name}}"
data.steps[0].completed_at matches timestamp
```

エラーアサーション:
```
Step 6 (CSRF): type == "csrf-validation-failed", status == 403
Step 7 (権限): type == "not-assigned", status == 403
Step 8 (Not found): type == "step-not-found", status == 404
Step 11 (Conflict): type == "conflict", status == 409
```

### 3-2. `workflow/reject_step.hurl`

`approve_step.hurl` と同じセットアップだが、結果が異なる。

テストシナリオ:
- [ ] 正常系: 却下成功 → 200
- [ ] 異常系: CSRF トークンなし → 403
- [ ] 異常系: 存在しないワークフロー → 404

注: 権限エラーと楽観的ロックは approve で十分検証済み。reject では正常系 + 基本エラーに絞る。

テストフロー:
```
1. Admin login → Create → Submit (assign user)
2. Error: POST reject WITHOUT CSRF → 403
3. Error: POST reject NON-EXISTENT workflow → 404
4. User login → CSRF
5. Success: POST reject → 200
```

アサーション（正常系）:
```
data.status == "Rejected"
data.version == 3
data.steps[0].status == "Completed"
data.steps[0].decision == "Rejected"
data.steps[0].comment == "内容に不備があります"
data.completed_at matches timestamp
```

---

## Phase 4: 仕上げ

### 確認事項: なし（既知のパターンのみ）

### 4-1. vars.env 更新

`user_name=一般ユーザー` を追加。

### 4-2. API テスト突合表 更新

`docs/50_テスト/APIテスト突合表.md` のギャップを「カバー済み」に更新。サマリーを 18/18 (100%) に更新。

### 4-3. Issue チェックボックス更新

`gh issue edit 321` で完了基準を `[x]` に更新。

### 4-4. `just check-all` 実行

全テスト通過を確認。

---

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → テストデータ戦略の検討 | シードデータの workflow instances と steps の確認、display_number の割り当てロジック、テスト実行順序（glob 順） | シードデータ参照 vs テスト内作成を endpoint 種別ごとに使い分ける方針に決定。dashboard は `>= 0` でテスト順序依存を回避 |
| 2回目 | approve/reject フローの詳細設計 | `get_task_by_display_numbers.hurl` の multi-user パターン、step version のライフサイクル（created:1, approve/reject:2）、エラーケースの実行順序 | エラーケースを成功ケースの前に配置し、状態変更前にすべてのエラーパスを検証する設計に |
| 3回目 | アサーション方針の突合 | `api-test.md` のルール vs 各フィールドの決定性を確認。user 一覧の順序不定性、dashboard カウントのテスト順序依存 | user 一覧は構造検証（`isString` + `matches`）、dashboard は `>= 0`。それ以外は可能な限り `==` を使用 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の完了基準 9 エンドポイントすべてが Phase 1-3 に含まれている。対象テーブルと Issue チェックリストを突合し差分ゼロ |
| 2 | 曖昧さ排除 | OK | 各テストのシナリオ、アサーション、エラー型を具体的に記載。「必要に応じて」等の曖昧表現なし |
| 3 | 設計判断の完結性 | OK | Phase 順序（複雑度順 vs 優先度順）、テストデータ戦略（シードデータ vs 作成）、アサーション厳密度（== vs >= 0）の判断理由を記載 |
| 4 | スコープ境界 | OK | 対象（9 テストファイル + vars.env + 突合表）と対象外（既存テスト修正、unauthorized 分離ファイル、OpenAPI 変更）を明記 |
| 5 | 技術的前提 | OK | Hurl の Cookie 管理（同一ファイル内共有）、テスト実行順序（glob 順、--jobs 1）、step version ライフサイクル（activate で不変、approve/reject で +1）を確認済み |
| 6 | 既存ドキュメント整合 | OK | api-test.md（アサーション方針）、OpenAPI required フィールド、既存テストパターンと整合 |

## 検証方法

```bash
just test-api        # 全 API テスト実行（新規 + 既存）
just check-all       # lint + test + API test
```
