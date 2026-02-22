# 計画: API テストカバレッジギャップの解消

## Context

テストピラミッドと ATDD の見直しにより、API テスト層に大きなカバレッジギャップが発見された。

- OpenAPI 仕様: 37 エンドポイント
- Hurl テスト: 26 ファイル / 21 エンドポイントをカバー
- API テスト突合表: 計測日 2026-02-09、18 エンドポイントしか追跡していない（陳腐化）
- 16 エンドポイントに API テストなし（ユーザー管理、ロール管理、定義管理書込系、監査ログ）
- E2E-009（ユーザー管理）は機能実装済みだが E2E テストなし

直近の PR #769 でユーザー API が `role_id` ベースに変更されており、テストなしの状態はリスクが高い。

## Epic / Story 構成

### Epic: API テストカバレッジギャップの解消

| Story | スコープ | 優先度 | 見積 |
|-------|---------|--------|------|
| Story 1 | API テスト突合表を最新の OpenAPI に同期 | 最高 | 小 |
| Story 2 | ユーザー管理 API テスト追加（4 EP） | 高 | 中 |
| Story 3 | ロール管理 API テスト追加（5 EP） | 高 | 中 |
| Story 4 | ワークフロー定義管理（書込系）API テスト追加（6 EP） | 中 | 大 |
| Story 5 | 監査ログ API テスト追加（1 EP） | 低 | 小 |
| Story 6 | E2E-009 ユーザー管理 E2E テスト追加 | 中 | 中 |

実装順序: Story 1 → 2 → 3 → 4 → 5 → 6

### Epic 完了基準

- 全 37 OpenAPI エンドポイントに対応する Hurl テストが存在する
- E2E-009（ユーザー管理: 作成→編集→無効化）の E2E テストが存在する
- API テスト突合表・E2E テスト突合表が最新状態に更新されている
- `just check-all` が通る

### テスト責任マッピング

| Epic 完了基準 | 操作パス | テスト層 | 担当 Story |
|-------------|---------|---------|----------|
| ユーザー管理 EP 全カバー | POST /users 正常系 | API | Story 2 |
| ユーザー管理 EP 全カバー | POST /users バリデーション失敗 | API | Story 2 |
| ユーザー管理 EP 全カバー | GET /users/{dn} 正常系 | API | Story 2 |
| ユーザー管理 EP 全カバー | PATCH /users/{dn} 正常系 | API | Story 2 |
| ユーザー管理 EP 全カバー | PATCH /users/{dn}/status 正常系 | API | Story 2 |
| ロール管理 EP 全カバー | GET /roles 正常系 | API | Story 3 |
| ロール管理 EP 全カバー | POST /roles 正常系 + 重複 409 | API | Story 3 |
| ロール管理 EP 全カバー | GET/PATCH/DELETE /roles/{id} | API | Story 3 |
| 定義管理書込 EP 全カバー | POST 作成 + PUT 更新 | API | Story 4 |
| 定義管理書込 EP 全カバー | POST validate（有効/無効） | API | Story 4 |
| 定義管理書込 EP 全カバー | POST publish + POST archive | API | Story 4 |
| 定義管理書込 EP 全カバー | DELETE（Draft のみ許可） | API | Story 4 |
| 監査ログ EP カバー | GET /audit-logs ページネーション | API | Story 5 |
| E2E-009 カバー | 作成→編集→無効化 | E2E | Story 6 |

## 今セッションの着手: Story 1 + Story 2

### Story 1: API テスト突合表を最新の OpenAPI に同期する

対象: `docs/08_テスト/APIテスト突合表.md`

スコープ:
- 37 全エンドポイントをリストし、Hurl テストの有無を記載
- 既存の 8 ファイル（comments, request_changes_step, resubmit, multi_step 系, full flow）を反映
- ギャップ 16 エンドポイントを明示

操作パス: 該当なし（ドキュメントのみ）

テストリスト:
- ユニットテスト（該当なし）
- ハンドラテスト（該当なし）
- API テスト（該当なし）
- E2E テスト（該当なし）

確認事項: なし（既知のパターンのみ）

### Story 2: ユーザー管理 API テストを追加する

対象エンドポイント:
- `POST /api/v1/users`
- `GET /api/v1/users/{display_number}`
- `PATCH /api/v1/users/{display_number}`
- `PATCH /api/v1/users/{display_number}/status`

#### 確認事項
- 型: CreateUserRequest, UpdateUserRequest, UpdateUserStatusRequest → `openapi/openapi.yaml`
- 型: UserDetailData レスポンス構造 → `openapi/openapi.yaml`
- パターン: 書込系 Hurl テストの構造（ログイン→CSRF→操作→検証）→ `tests/api/hurl/workflow/create_workflow.hurl`
- パターン: vars.env のシードデータ変数 → `tests/api/hurl/vars.env`
- ライブラリ: Hurl Capture → 既存使用パターンで確認済み

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 管理者がユーザーを作成する | 正常系 | API |
| 2 | バリデーション不正でユーザー作成が失敗する | 準正常系 | API |
| 3 | 管理者がユーザー詳細を取得する | 正常系 | API |
| 4 | 管理者がユーザー情報を更新する | 正常系 | API |
| 5 | 管理者がユーザーを無効化する | 正常系 | API |
| 6 | 未認証でユーザー作成が拒否される | 異常系 | API |

#### テストリスト

API テスト:
- [ ] `user/create_user.hurl` — ユーザー作成（正常系: 201 + 全 required フィールド検証）
- [ ] `user/create_user.hurl` — バリデーション失敗（email 形式不正 → 400/422）
- [ ] `user/create_user.hurl` — CSRF トークンなし → 403
- [ ] `user/get_user_detail.hurl` — ユーザー詳細取得（正常系: 作成したユーザーの情報を検証）
- [ ] `user/update_user.hurl` — ユーザー更新（name, role_id 変更 → 200 + 変更反映確認）
- [ ] `user/update_user_status.hurl` — ステータス変更（active → inactive → 200 + status 検証）

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
E2E テスト（該当なし）

#### 作成ファイル

| ファイル | 内容 |
|---------|------|
| `tests/api/hurl/user/create_user.hurl` | POST /users 正常系 + 異常系 |
| `tests/api/hurl/user/get_user_detail.hurl` | GET /users/{display_number} |
| `tests/api/hurl/user/update_user.hurl` | PATCH /users/{display_number} |
| `tests/api/hurl/user/update_user_status.hurl` | PATCH /users/{display_number}/status |

#### Hurl テストの構造（既存パターン準拠）

```
# 冒頭コメント（日本語）
# エンドポイント説明

# ログイン + CSRF 取得（書込系のみ）
POST /auth/login → Capture session_cookie
GET /auth/csrf → Capture csrf_token

# 正常系（Given-When-Then コメント付き）
POST /users + Asserts（required フィールド全検証）+ Captures

# 異常系
POST /users（不正データ）→ 400/422
POST /users（CSRF なし）→ 403
```

## 重要な参照ファイル

| ファイル | 用途 |
|---------|------|
| `openapi/openapi.yaml` | エンドポイント定義、required フィールド、スキーマ |
| `tests/api/hurl/vars.env` | テスト用シードデータ変数 |
| `tests/api/hurl/workflow/create_workflow.hurl` | 書込系テストの参照パターン |
| `tests/api/hurl/user/list_users.hurl` | ユーザー系テストの参照パターン |
| `.claude/rules/api-test.md` | アサーション方針（決定性ベース） |
| `docs/08_テスト/APIテスト突合表.md` | 更新対象 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | テストリストに 404 と再有効化のケースが欠落 | 操作パス網羅漏れ | get_user_detail に 404 ケース、update_user_status に active→inactive→active の往復を追加 |
| 2回目 | vars.env にロール ID 変数がない | 未定義 | `tenant_admin_role_id`, `user_role_id` を vars.env に追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 4 エンドポイント × 正常系 + 異常系がテストリストに含まれている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各テストケースの期待値（ステータスコード、アサーション対象）が明示されている |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | Hurl テスト構造パターンを明示、アサーション方針は api-test.md に準拠 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: Story 2 の 4 EP。対象外: ロール管理以降は別 Story |
| 5 | 技術的前提 | 前提が考慮されている | OK | OpenAPI スキーマの required フィールド、シードデータ変数、Hurl の Capture パターンを確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | api-test.md のアサーション方針、既存 list_users.hurl のパターンと整合 |

## 検証方法

1. `just test-api` — 新規 Hurl テストが全て通ること
2. `just check-all` — 全テストスイートが通ること
3. 突合表の更新内容が OpenAPI と一致していること
