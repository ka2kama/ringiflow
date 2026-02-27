# 計画: Issue #940 既存テストへのエッジケース・境界値テスト追加

## Context

#939 で策定したエッジケース・境界値テスト方針に基づき、既存テストにエッジケース・境界値テストを追加する。テスト層ごとの責務分離を遵守し、各層で独自の責務のみを検証する。

## スコープ

対象:
- ユニットテスト: 値オブジェクトの特殊文字テスト
- API テスト（Hurl）: 入力値境界、ページネーション無効値、状態遷移異常系、存在しない ID
- E2E テスト（Playwright）: ConfirmDialog 表示確認、二重送信防止

対象外:
- プロダクションコードの変更（テストが既存の不具合を発見した場合は別 Issue で対応）
- 新規エンドポイントや新規 UI コンポーネントの追加
- 認証・CSRF テスト（既にカバー済み）
- 楽観的ロック・テナント分離テスト（既にカバー済み）

## 設計判断

### バリデーションエラーのステータスコード

| 原因 | ステータス | 理由 |
|------|-----------|------|
| フィールド欠落（JSON 構造不正） | 422 | axum の Json extractor が自動返却 |
| 値の不正（空文字、長さ超過） | 400 | ドメイン層 `DomainError::Validation` → BFF `validation_error_response` |

API テストの期待ステータスは Phase 2 の確認事項で実測して確定する。

### ページネーション無効値テスト

OpenAPI 仕様上、ページネーション（cursor + limit）を持つのは audit-logs のみ。他の一覧 API にはページネーションパラメータがないため、テスト対象は audit-logs に限定する。

### 存在しない ID テストの対象選定

404 テスト未実装の 30 エンドポイントのうち、ID パスパラメータを持つ更新・操作系のみを対象とする:
- update_user, update_user_status
- update_role
- update_definition, publish_definition, archive_definition, validate_definition
- resubmit_workflow

一覧系・認証系・ヘルスチェックは ID を取らないため対象外。

---

## Phase 1: ユニットテスト — 値オブジェクトの特殊文字テスト（Priority 1）

### 確認事項
- 型: `define_validated_string!` マクロの動作 → `backend/crates/domain/src/macros.rs`
- パターン: 既存の値オブジェクトテスト → `backend/crates/domain/src/value_objects.rs` テストモジュール
- パターン: rstest の使用パターン → 同上（`#[case]` + `#[rstest]`）

### 対象の値オブジェクト

| 型 | ファイル | 最大文字数 | PII |
|----|---------|----------|-----|
| UserName | `value_objects.rs` | 100 | ✓ |
| WorkflowName | `value_objects.rs` | 200 | ✗ |
| TenantName | `tenant.rs` | 255 | ✗ |
| CommentBody | `workflow/comment.rs` | 2,000 | ✗ |

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 特殊文字（改行、タブ、HTML タグ）を含む文字列で値オブジェクト生成 | 正常系 | ユニット |
| 2 | 特殊文字のみの文字列で値オブジェクト生成 | 準正常系 | ユニット |
| 3 | 特殊文字 + 最大長ちょうどの文字列で値オブジェクト生成 | 正常系（境界値） | ユニット |

### テストリスト

ユニットテスト:
- [ ] UserName: 改行・タブ・HTML タグを含む文字列が受け入れられること
- [ ] WorkflowName: 改行・タブ・HTML タグを含む文字列が受け入れられること
- [ ] TenantName: 改行・タブ・HTML タグを含む文字列が受け入れられること
- [ ] CommentBody: 改行・タブ・HTML タグを含む文字列が受け入れられること

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 2: API テスト — 入力値境界の伝播確認（Priority 1）

### 確認事項
- パターン: 既存 API テストの異常系構造 → `tests/api/hurl/user/create_user.hurl`
- パターン: セクションコメントの書式 → `api-test.md` の配置ルール
- ライブラリ: Hurl の `matches` / `contains` 構文 → Grep で既存使用確認
- 型: BFF のバリデーションエラーレスポンス → `backend/apps/bff/src/error.rs`
- 実測: 空文字送信時のステータスコード（400 vs 422）を開発サーバーで確認

### 対象エンドポイント

| エンドポイント | テストファイル | テスト対象フィールド |
|-------------|-------------|-------------------|
| POST /api/v1/users | `user/create_user.hurl` | name（UserName: 100 文字）、email |
| POST /api/v1/workflows | `workflow/create_workflow.hurl` | title（WorkflowName: 200 文字） |
| POST /api/v1/roles | `role/create_role.hurl` | name |

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 空文字フィールドでリソース作成 | 準正常系 | API |
| 2 | 特殊文字（HTML タグ）フィールドでリソース作成 | 正常系 | API |
| 3 | 最大長超過フィールドでリソース作成 | 準正常系 | API |

### テストリスト

ユニットテスト（該当なし — Phase 1 でカバー）

ハンドラテスト（該当なし）

API テスト:
- [ ] create_user: name に空文字 → 400（伝播確認）
- [ ] create_user: name に 101 文字 → 400（伝播確認）
- [ ] create_user: name に HTML タグ含有 → 201（500 にならないことを確認）
- [ ] create_workflow: title に空文字 → 400（伝播確認）
- [ ] create_workflow: title に 201 文字 → 400（伝播確認）
- [ ] create_workflow: title に改行文字含有 → 201（500 にならないことを確認）
- [ ] create_role: name に空文字 → 400（伝播確認）

E2E テスト（該当なし）

---

## Phase 3: API テスト — ページネーション無効値・状態遷移異常系（Priority 2）

### 確認事項
- パターン: audit-logs テスト → `tests/api/hurl/audit_log/list_audit_logs.hurl`
- 型: ページネーションパラメータ（limit, cursor）の型と制約 → OpenAPI + handler 実装
- パターン: 状態遷移テストの構造 → `tests/api/hurl/workflow/approve_step.hurl` の異常系
- パターン: 差し戻し後フロー → `tests/api/hurl/workflow/request_changes_step.hurl`
- 実測: limit=0 / limit=-1 / 不正 cursor のレスポンスを開発サーバーで確認

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ページネーション API に limit=0 を指定 | 準正常系 | API |
| 2 | ページネーション API に limit=-1 を指定 | 準正常系 | API |
| 3 | ページネーション API に不正な cursor を指定 | 準正常系 | API |
| 4 | 差し戻し済みワークフローのステップを承認 | 異常系 | API |
| 5 | 差し戻し済みワークフローのステップを却下 | 異常系 | API |

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

API テスト:
- [ ] list_audit_logs: limit=0 → 422 またはデフォルト値適用
- [ ] list_audit_logs: limit=-1 → 422
- [ ] list_audit_logs: 不正な cursor 文字列 → 422 またはエラー
- [ ] 差し戻し後: 完了済みステップを承認 → 409（状態遷移エラー）
- [ ] 差し戻し後: 完了済みステップを却下 → 409（状態遷移エラー）

E2E テスト（該当なし）

---

## Phase 4: E2E テスト — ConfirmDialog 表示・二重送信防止（Priority 2-3）

### 確認事項
- パターン: 既存 E2E テスト → `tests/e2e/tests/approval.spec.ts`
- パターン: ヘルパー関数（approveTask, rejectTask）→ `tests/e2e/helpers/workflow.ts`
- 型: ConfirmDialog の DOM 構造 → `frontend/src/Component/ConfirmDialog.elm`（`<dialog>` 要素、aria 属性）
- パターン: 二重送信防止の実装 → `frontend/src/Page/User/New.elm`（`submitting` フラグ）
- ライブラリ: Playwright の `toBeVisible()` / `toBeDisabled()` → Grep で既存使用確認

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 承認ボタン押下後に確認ダイアログが表示される | 正常系 | E2E |
| 2 | 却下ボタン押下後に確認ダイアログが表示される | 正常系 | E2E |
| 3 | 確認ダイアログでキャンセル → 操作されない | 正常系 | E2E |
| 4 | フォーム送信後にボタンが disabled になる | 正常系 | E2E |

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト:
- [ ] 承認操作: 確認ダイアログが表示され、タイトル・メッセージが適切であること
- [ ] 却下操作: 確認ダイアログが表示されること
- [ ] 確認ダイアログ: キャンセルボタンで閉じ、操作が実行されないこと
- [ ] ユーザー作成: 送信ボタンクリック後に disabled 状態になること

---

## Phase 5: API テスト — 存在しない ID での操作（Priority 3）

### 確認事項
- パターン: 既存の 404 テスト → `tests/api/hurl/user/get_user_detail.hurl`、`tests/api/hurl/workflow/get_workflow.hurl`
- パターン: RFC 9457 エラーレスポンスの `type` フィールド値 → Grep で既存パターン確認
- 型: 各エンドポイントのパスパラメータ型（UUID vs display_number）→ OpenAPI

### 対象エンドポイント

| エンドポイント | テストファイル | パスパラメータ |
|-------------|-------------|-------------|
| PUT /api/v1/users/:id | `user/update_user.hurl` | UUID |
| PATCH /api/v1/users/:id/status | `user/update_user_status.hurl` | UUID |
| PUT /api/v1/roles/:id | `role/update_role.hurl` | UUID |
| PUT /api/v1/workflow-definitions/:id | `workflow_definition/update_definition.hurl` | UUID |
| POST /api/v1/workflow-definitions/:id/publish | `workflow_definition/publish_definition.hurl` | UUID |
| POST /api/v1/workflow-definitions/:id/archive | `workflow_definition/archive_definition.hurl` | UUID |
| POST /api/v1/workflow-definitions/:id/validate | `workflow_definition/validate_definition.hurl` | UUID |
| POST /api/v1/workflows/:display_number/resubmit | `workflow/resubmit_workflow.hurl` | display_number |

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 存在しない UUID で更新系エンドポイントを呼び出す | 異常系 | API |
| 2 | 存在しない display_number で操作系エンドポイントを呼び出す | 異常系 | API |

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

API テスト:
- [ ] update_user: 存在しない UUID → 404
- [ ] update_user_status: 存在しない UUID → 404
- [ ] update_role: 存在しない UUID → 404
- [ ] update_definition: 存在しない UUID → 404
- [ ] publish_definition: 存在しない UUID → 404
- [ ] archive_definition: 存在しない UUID → 404
- [ ] validate_definition: 存在しない UUID → 404
- [ ] resubmit_workflow: 存在しない display_number → 404

E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ページネーションが audit-logs のみ（offset-based は存在しない） | 未定義 | 対象を audit-logs の cursor+limit に限定、Issue の「offset=-1」は cursor ベースの無効値テストに読み替え |
| 2回目 | バリデーションエラーのステータスが 400（フィールド欠落 422 とは異なる） | 曖昧 | 設計判断セクションでステータスコードの使い分けを明記、Phase 2 の確認事項に実測を追加 |
| 3回目 | 差し戻し後の異常パス — ステップは Completed 状態なので approve/reject は状態遷移エラー | 不完全なパス | Phase 3 のテストリストを「完了済みステップへの操作 → 409」として具体化 |
| 4回目 | 存在しない ID テスト — 一覧系・認証系は ID を取らないため対象外 | 既存手段の見落とし | 対象を ID パスパラメータを持つ更新・操作系 8 エンドポイントに絞り込み |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準 9 項目が全て Phase に割り当て | OK | Priority 1: Phase 1-2、Priority 2: Phase 3-4、Priority 3: Phase 4-5 |
| 2 | 曖昧さ排除 | 期待ステータスコードが明記 | OK | Phase 2 で実測確認をゲート条件に設定 |
| 3 | 設計判断の完結性 | ページネーション対象・404 対象の絞り込み根拠 | OK | 設計判断セクションに記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | スコープセクションで明記 |
| 5 | 技術的前提 | Hurl の構文、Playwright API、axum エラーハンドリング | OK | 確認事項で実測・Grep を指定 |
| 6 | 既存ドキュメント整合 | テスト戦略エッジケース方針（#939）と整合 | OK | テスト層の責務分離、カテゴリ別テスト数の目安を遵守 |
