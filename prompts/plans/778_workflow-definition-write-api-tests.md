# 実装計画: #778 ワークフロー定義管理（書込系）API テスト

## Context

Epic #774（API テストカバレッジギャップの解消）の Story。ワークフロー定義セクション 8 EP 中、読取系 2 EP（list, get）はカバー済みだが、書込系 6 EP にテストがない。先行 Story #776（ユーザー管理）、#777（ロール管理）と同じパターンで Hurl API テストを追加する。

ワークフロー定義は状態遷移（Draft → Published → Archived）を持つため、単純な CRUD テストに加えて状態遷移テストと楽観的ロック（version）の検証が必要。

## 対象・対象外

対象:
- 6 EP の Hurl API テスト作成（`tests/api/hurl/workflow_definition/`）
- API テスト突合表の更新（`docs/08_テスト/APIテスト突合表.md`）

対象外:
- 既存の読取系テスト（list, get）の変更
- OpenAPI 仕様の変更（既存 API のテスト追加のみ）
- 詳細設計書の変更

## 操作パス

| # | 操作パス | 分類 | テスト層 | テストファイル |
|---|---------|------|---------|-------------|
| 1 | 有効なデータでワークフロー定義を作成する | 正常系 | API | create_definition.hurl |
| 2 | 有効な定義 JSON を検証する（valid: true） | 正常系 | API | validate_definition.hurl |
| 3 | 無効な定義 JSON を検証する（valid: false） | 準正常系 | API | validate_definition.hurl |
| 4 | Draft 状態の定義を更新する | 正常系 | API | update_definition.hurl |
| 5 | Draft 状態の定義を削除する | 正常系 | API | delete_definition.hurl |
| 6 | Published 状態の定義を削除しようとする（失敗） | 準正常系 | API | delete_definition.hurl |
| 7 | Draft 状態の定義を公開する（Draft → Published） | 正常系 | API | publish_definition.hurl |
| 8 | Published 状態の定義をアーカイブする（Published → Archived） | 正常系 | API | archive_definition.hurl |

操作パス #8（archive）のテストは Draft の作成 → 公開 → アーカイブの全フローを含み、完了基準の「状態遷移テスト（Draft → Published → Archived）」をカバーする。

## Phase 1: API テスト作成 + 突合表更新

単一 Phase。全 6 テストファイルと突合表更新を一括で実施する。

### 確認事項

- パターン: 既存 API テストの構造 → `tests/api/hurl/role/create_role.hurl`（Login → CSRF → 正常系 → 異常系）
- パターン: 既存 WF 定義テストの構造 → `tests/api/hurl/workflow_definition/get_workflow_definition.hurl`（アサーション形式）
- 型: WorkflowDefinitionData の required フィールド → OpenAPI spec（id, name, version, definition, status, created_by, created_at, updated_at）
- 型: ValidationResultData の required フィールド → OpenAPI spec（valid, errors）+ ValidationErrorData（code, message）
- 型: ErrorResponse の形式 → `backend/crates/shared/src/error_response.rs`（type, title, status, detail）
- テスト変数: `tests/api/hurl/vars.env`（bff_url, tenant_id, admin_email, password, workflow_definition_id）

### テストリスト

API テスト:
- [ ] create_definition.hurl: 正常系 — 有効なデータで定義作成（201、全 required フィールド検証、status=="Draft"、version==1）
- [ ] validate_definition.hurl: 正常系 — 有効な定義 JSON の検証（200、valid==true、errors count==0）
- [ ] validate_definition.hurl: 準正常系 — 無効な定義 JSON の検証（200、valid==false、errors count>=1、code/message 検証）
- [ ] update_definition.hurl: 正常系 — Draft の名前と定義を更新（200、name 変更確認、version==2）
- [ ] delete_definition.hurl: 準正常系 — Published 定義（seed）の削除失敗（400、validation-error）
- [ ] delete_definition.hurl: 正常系 — Draft 定義の削除成功（204）
- [ ] publish_definition.hurl: 正常系 — Draft を公開（200、status=="Published"、version==2）
- [ ] archive_definition.hurl: 正常系 — Create → Publish → Archive の全フロー（200、status=="Archived"、version==3）

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
E2E テスト（該当なし）

ドキュメント:
- [ ] API テスト突合表: ワークフロー定義セクションの 6 EP を「カバー済み」に更新、カバー率更新

### テストファイル設計

共通前提（全ファイル共通）:
```hurl
# Login → session_cookie
POST {{bff_url}}/api/v1/auth/login
# CSRF token 取得（書込 API に必要）
GET {{bff_url}}/api/v1/auth/csrf
```

#### 1. create_definition.hurl

```
正常系: POST /api/v1/workflow-definitions → 201
  リクエスト: name="テスト申請", definition=valid_json
  アサーション:
    - id: exists（自動生成 UUID）
    - name == "テスト申請"
    - description: null（省略時）
    - version == 1（新規作成は必ず 1）
    - status == "Draft"（新規作成は必ず Draft）
    - definition: exists
    - created_by: matches UUID
    - created_at, updated_at: matches ISO8601
```

#### 2. validate_definition.hurl

```
正常系: POST /api/v1/workflow-definitions/validate → 200
  リクエスト: definition=valid_json（start + approval + end_approved + end_rejected + transitions）
  アサーション:
    - valid == true
    - errors count == 0

準正常系: POST /api/v1/workflow-definitions/validate → 200
  リクエスト: definition=invalid_json（approval step なし）
  アサーション:
    - valid == false
    - errors count >= 1
    - errors[0].code == "missing_approval_step"
    - errors[0].message: isString
```

#### 3. update_definition.hurl

```
セットアップ: POST create → 201（Capture: def_id, version）
正常系: PUT /api/v1/workflow-definitions/{def_id} → 200
  リクエスト: name="更新後の申請", definition=valid_json, version=1
  アサーション:
    - id == def_id（Capture した ID と同一）
    - name == "更新後の申請"
    - version == 2（1 → 2 にインクリメント）
    - status == "Draft"（更新で状態は変わらない）
```

#### 4. delete_definition.hurl

```
準正常系: DELETE /api/v1/workflow-definitions/{seed_published_id} → 400
  アサーション:
    - type == ".../validation-error"
    - status == 400

セットアップ: POST create → 201（Capture: def_id）
正常系: DELETE /api/v1/workflow-definitions/{def_id} → 204
```

#### 5. publish_definition.hurl

```
セットアップ: POST create → 201（Capture: def_id, version）
正常系: POST /api/v1/workflow-definitions/{def_id}/publish → 200
  リクエスト: version=1
  アサーション:
    - id == def_id
    - status == "Published"
    - version == 2（1 → 2）
    - name, definition, created_by, timestamps: 検証
```

#### 6. archive_definition.hurl（状態遷移テスト）

```
セットアップ: POST create → 201（Capture: def_id）
セットアップ: POST publish → 200（Capture: version, status=="Published" 確認）
正常系: POST /api/v1/workflow-definitions/{def_id}/archive → 200
  リクエスト: version=2
  アサーション:
    - id == def_id
    - status == "Archived"
    - version == 3（2 → 3）
    - name, definition, created_by, timestamps: 検証
```

### 有効な定義 JSON（全テスト共通）

Core Service テストフィクスチャ（`valid_definition_json()`）と同じ構造:

```json
{
  "steps": [
    {"id": "start", "type": "start", "name": "開始"},
    {"id": "approval_1", "type": "approval", "name": "承認"},
    {"id": "end_approved", "type": "end", "name": "承認完了", "status": "approved"},
    {"id": "end_rejected", "type": "end", "name": "却下", "status": "rejected"}
  ],
  "transitions": [
    {"from": "start", "to": "approval_1"},
    {"from": "approval_1", "to": "end_approved", "trigger": "approve"},
    {"from": "approval_1", "to": "end_rejected", "trigger": "reject"}
  ]
}
```

### 無効な定義 JSON（validate テスト用）

承認ステップなしの定義（`missing_approval_step` エラーが発生する）:

```json
{
  "steps": [
    {"id": "start", "type": "start", "name": "開始"},
    {"id": "end", "type": "end", "name": "終了", "status": "approved"}
  ],
  "transitions": [
    {"from": "start", "to": "end"}
  ]
}
```

### アサーション方針

api-test.md に準拠:

| フィールド | 値の性質 | アサーション |
|-----------|---------|-------------|
| name, status, version | 決定的（入力値・ドメインロジック） | `==` 厳密一致 |
| id, created_by | 非決定的（自動生成 UUID） | `exists` or `matches UUID` |
| created_at, updated_at | 非決定的（タイムスタンプ） | `matches ISO8601` |
| description | nullable | `exists`（null の場合も存在する） |
| definition | オブジェクト | `exists` |
| errors | 配列 | `count` |
| errors[].code | 決定的（バリデーションルール） | `==` |
| Capture した id との比較 | Capture → == パターン | `==` |

### エラータイプ URI（ProblemDetails）

| エラー種別 | type URI |
|-----------|---------|
| 400 Bad Request（Draft 以外の削除等） | `https://ringiflow.example.com/errors/validation-error` |
| 404 Not Found | `https://ringiflow.example.com/errors/workflow-definition-not-found` |
| 409 Conflict（バージョン競合） | `https://ringiflow.example.com/errors/conflict` |

### 重要ファイル

| ファイル | 役割 |
|---------|------|
| `tests/api/hurl/role/create_role.hurl` | 参照パターン（Login → CSRF → 正常系 → 異常系） |
| `tests/api/hurl/workflow_definition/get_workflow_definition.hurl` | 参照パターン（WF 定義のアサーション） |
| `tests/api/hurl/vars.env` | テスト変数 |
| `backend/apps/bff/src/handler/workflow_definition.rs` | BFF ハンドラ（リクエスト/レスポンス型） |
| `backend/crates/shared/src/error_response.rs` | エラーレスポンス形式 |
| `backend/crates/domain/src/workflow/definition_validator.rs` | バリデーションルール |
| `docs/08_テスト/APIテスト突合表.md` | 更新対象 |
| `openapi/openapi.yaml` | required フィールドの参照元 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | archive テストが状態遷移テストを兼ねることを明示していなかった | 操作パス網羅漏れ | 操作パス表に「archive が Draft → Published → Archived の全フローをカバー」と注記を追加 |
| 1回目 | delete の準正常系で seed データの Published 定義を使う設計が明示されていなかった | 未定義 | delete_definition.hurl の設計に seed データ（workflow_definition_id）の利用を明記 |
| 1回目 | エラータイプ URI が未記載だった | 未定義 | BFF error.rs から URI を調査し、エラータイプ URI テーブルを追加 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の完了基準 6 項目 × Epic テスト責任マッピング 4 行すべてに対応するテストが計画に含まれている |
| 2 | 曖昧さ排除 | OK | 各テストファイルの入力・期待値・アサーション方針が具体的に記載されている |
| 3 | 設計判断の完結性 | OK | 判断点は「テストファイル分割」（EP 単位 = 既存パターン踏襲）のみ。新規判断なし |
| 4 | スコープ境界 | OK | 対象（6 テストファイル + 突合表）と対象外（読取系テスト、OpenAPI、設計書）を明記 |
| 5 | 技術的前提 | OK | Hurl の Capture → assert パターン、CSRF 要件を確認済み |
| 6 | 既存ドキュメント整合 | OK | OpenAPI spec の required フィールド、api-test.md のアサーション方針と整合 |

## 検証

```bash
just test-api        # API テスト実行（ワークフロー定義の新テスト含む）
just check-all       # 全体チェック（lint + test + API test + E2E test）
```
