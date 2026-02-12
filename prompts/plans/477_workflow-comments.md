# 実装計画: #477 ワークフローコメント機能

## Context

ワークフローに対するコメントスレッド機能を実装する。承認プロセス中に申請者と承認者がコメントでやり取りできるようにする。

**重要な区別**: 既存の `workflow_steps.comment` はステップの判定コメント（承認/却下時に入力するコメント）。今回実装するのはワークフロー単位のコメントスレッドであり、別概念。

要件出典: 機能仕様書 セクション 4.7（`docs/01_要件定義書/機能仕様書/01_ワークフロー管理.md` L335-357）

## スコープ

### 対象

- `workflow_comments` テーブル作成（マイグレーション + RLS）
- Comment ドメインモデル（エンティティ + 値オブジェクト）
- WorkflowCommentRepository（trait + PostgreSQL 実装 + Mock）
- コメント投稿/取得ユースケース（権限チェック含む）
- Core Service ハンドラ（POST / GET）
- BFF ハンドラ（POST / GET）+ CoreServiceClient 拡張
- OpenAPI 仕様書の更新
- テナント削除レジストリへの登録
- API テスト（Hurl）

### 対象外

- コメントの編集・削除
- コメントへのリアクション・通知
- フロントエンド（Elm）の実装

## 設計判断

### 1. CommentBody 値オブジェクト

`CommentBody(String)` を導入。仕様で 1〜2,000 文字の制約があり、型レベルでバリデーションを強制する。

### 2. 権限チェックの実装場所

ユースケース層で実装。関与者 = 申請者（`instance.initiated_by == user_id`）OR いずれかのステップの承認者（`steps.any(|s| s.assigned_to() == Some(&user_id))`）。既存の承認/却下の権限チェックパターンに合致。

### 3. ユースケースの配置

`WorkflowUseCaseImpl` に `comment_repo` を追加し、`post_comment` を `command.rs`、`list_comments` を `query.rs` に追加。コメントの権限チェックには `instance_repo` + `step_repo` が必要で、独立した UseCase にすると依存が重複する。

### 4. API URL

- POST `/api/v1/workflows/{display_number}/comments`
- GET `/api/v1/workflows/{display_number}/comments`

### 5. posted_by の FK 制約

`ON DELETE RESTRICT`。ユーザー削除時にコメントが消えると監査上の問題がある。テナント退会は workflow_instances の CASCADE で対処。

## Phase 分割

### Phase 1: マイグレーション + ドメインモデル

#### 確認事項
- [ ] 型: `WorkflowInstanceId` の定義パターン → `domain/src/workflow/instance.rs`
- [ ] 型: `DomainError::Validation` → `domain/src/error.rs` L64
- [ ] パターン: マイグレーション構文（RLS, tenant_id, トリガー） → `20260115000007_create_workflow_steps.sql`, `20260210000003_enable_rls_policies.sql`
- [ ] パターン: エンティティの `new()` / `from_db()` パターン → `workflow/instance.rs`

#### テストリスト

ユニットテスト:
- [ ] `CommentBody::new` — 正常: 1 文字で成功
- [ ] `CommentBody::new` — 正常: 2000 文字で成功
- [ ] `CommentBody::new` — 異常: 空文字列でエラー
- [ ] `CommentBody::new` — 異常: 2001 文字でエラー
- [ ] `WorkflowComment::new` — 正常: 初期状態の検証

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 実装内容

**マイグレーション** `YYYYMMDDHHMMSS_create_workflow_comments.sql`:

```sql
CREATE TABLE workflow_comments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL,
    instance_id UUID NOT NULL REFERENCES workflow_instances(id) ON DELETE CASCADE,
    posted_by UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    body TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT workflow_comments_body_length CHECK (
        char_length(body) >= 1 AND char_length(body) <= 2000
    )
);
CREATE INDEX workflow_comments_instance_idx ON workflow_comments(instance_id);
CREATE INDEX workflow_comments_tenant_idx ON workflow_comments(tenant_id);
CREATE TRIGGER workflow_comments_updated_at
    BEFORE UPDATE ON workflow_comments FOR EACH ROW EXECUTE FUNCTION update_updated_at();
ALTER TABLE workflow_comments ENABLE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation ON workflow_comments FOR ALL TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);
```

`tenant_id` を直接保持（`workflow_steps` と同じ RLS 二重防御パターン）。

**ドメインモデル** `domain/src/workflow/comment.rs`:

- `WorkflowCommentId(Uuid)` — UUID v7 Newtype
- `CommentBody(String)` — 1〜2000 文字バリデーション
- `WorkflowComment` — エンティティ（`new()` + `from_db()` パターン）
- `NewWorkflowComment` / `WorkflowCommentRecord` — 生成/復元パラメータ

**変更ファイル**: `domain/src/workflow.rs` に `mod comment;` + `pub use comment::*;` 追加

### Phase 2: リポジトリ（trait + PostgreSQL 実装 + Mock）

#### 確認事項
- [ ] パターン: リポジトリ trait シグネチャ（`tenant_id` 引数） → `infra/src/repository/workflow_instance_repository.rs`
- [ ] パターン: `XxxRow` + `TryFrom` 変換 → `workflow_instance_repository.rs`
- [ ] パターン: Mock リポジトリの構造 → `infra/src/mock.rs`

#### テストリスト

ユニットテスト（該当なし）

統合テスト（`infra/tests/workflow_comment_repository_test.rs`）:
- [ ] `insert` — 正常: 新規コメントを作成できる
- [ ] `find_by_instance` — 正常: インスタンス ID でコメント一覧を取得できる（created_at ASC）
- [ ] `find_by_instance` — 正常: 存在しないインスタンスは空ベクターを返す
- [ ] `find_by_instance` — 正常: 複数コメントが時系列昇順で返る
- [ ] テナント分離: 別テナントのコメントは取得できない

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 実装内容

**新規**: `infra/src/repository/workflow_comment_repository.rs`

- `WorkflowCommentRepository` trait: `insert`, `find_by_instance`
- `PostgresWorkflowCommentRepository`: `PgPool` ベースの実装
- `WorkflowCommentRow`: 中間構造体 + `TryFrom` 変換

**変更ファイル**:
- `infra/src/repository.rs` — モジュール追加 + re-export
- `infra/src/mock.rs` — `MockWorkflowCommentRepository` 追加
- `infra/src/deletion/postgres_workflow.rs` — `workflow_comments` DELETE 追加（`workflow_steps` の前に配置）
- `infra/tests/common/mod.rs` — `create_test_comment()` ヘルパー追加

**sqlx-prepare**: マイグレーション後に `just sqlx-prepare` 実行

### Phase 3: ユースケース（権限チェック含む）

#### 確認事項
- [ ] 型: `WorkflowUseCaseImpl` の構造と `new()` → `core-service/src/usecase/workflow.rs` L92-119
- [ ] パターン: ユースケースの `tenant_id` / `user_id` 受け取り方 → `command.rs`
- [ ] パターン: `CoreError` バリアント → `core-service/src/error.rs`

#### テストリスト

ユニットテスト:
- [ ] `post_comment` — 正常: 申請者がコメントを投稿できる
- [ ] `post_comment` — 正常: 承認者がコメントを投稿できる
- [ ] `post_comment` — 異常: 関与していないユーザーは Forbidden
- [ ] `post_comment` — 異常: ワークフローが見つからない場合 NotFound
- [ ] `list_comments` — 正常: コメント一覧を取得できる
- [ ] `list_comments` — 異常: ワークフローが見つからない場合 NotFound

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

#### 実装内容

**`WorkflowUseCaseImpl` への変更**:
- `comment_repo: Arc<dyn WorkflowCommentRepository>` フィールド追加
- `new()` パラメータ追加（既存テストにも影響）

**入力型** `PostCommentInput { body: String }`:

**ユースケースメソッド**:

```rust
// command.rs
pub async fn post_comment(
   &self, input: PostCommentInput, display_number: DisplayNumber,
   tenant_id: TenantId, user_id: UserId,
) -> Result<WorkflowComment, CoreError>

// 内部ヘルパー
async fn is_participant(
   &self, instance: &WorkflowInstance, user_id: &UserId, tenant_id: &TenantId,
) -> Result<bool, CoreError>

// query.rs
pub async fn list_comments(
   &self, display_number: DisplayNumber, tenant_id: TenantId,
) -> Result<Vec<WorkflowComment>, CoreError>
```

### Phase 4: Core Service ハンドラ

#### 確認事項
- [ ] パターン: `WorkflowState` と `State(state)` → `handler/workflow.rs` L281-283
- [ ] パターン: `TenantQuery` クエリパラメータ → `handler/workflow.rs` L94-98
- [ ] パターン: `UserRefDto` + `to_user_ref()` → `handler/workflow.rs` L109-129

#### テストリスト

ユニットテスト（該当なし — ハンドラは薄い）

ハンドラテスト（該当なし）

API テスト（Phase 6 で実施）

E2E テスト（該当なし）

#### 実装内容

**DTO**: `WorkflowCommentDto { id, posted_by: UserRefDto, body, created_at }`

**リクエスト型**: `PostCommentRequest { body, tenant_id, user_id }`（Core Service 内部 API 用）

**ハンドラ**:
- `POST /internal/workflows/by-display-number/{display_number}/comments` → `post_comment`
- `GET /internal/workflows/by-display-number/{display_number}/comments?tenant_id=` → `list_comments`

**変更ファイル**:
- `handler/workflow.rs` — DTO + リクエスト型追加
- `handler/workflow/command.rs` — `post_comment` ハンドラ追加
- `handler/workflow/query.rs` — `list_comments` ハンドラ追加
- `main.rs` — ルート追加、`WorkflowUseCaseImpl::new()` に `comment_repo` 追加

### Phase 5: BFF ハンドラ + OpenAPI + CoreServiceClient

#### 確認事項
- [ ] パターン: BFF ハンドラのセッション取得 → `bff/src/handler/workflow/command.rs` L56-98
- [ ] パターン: `CoreServiceWorkflowClient` メソッド → `bff/src/client/core_service/workflow_client.rs`
- [ ] パターン: BFF DTO（`ToSchema` derive）→ `bff/src/handler/workflow.rs`
- [ ] パターン: `utoipa::path` マクロ → BFF command.rs
- [ ] パターン: Core Service types.rs のリクエスト型 → `bff/src/client/core_service/types.rs`

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（Phase 6 で実施）

E2E テスト（該当なし）

#### 実装内容

**CoreServiceWorkflowClient** に追加:
- `post_comment(display_number, req)` → `POST /internal/workflows/by-display-number/{dn}/comments`
- `list_comments(display_number, tenant_id)` → `GET /internal/workflows/by-display-number/{dn}/comments?tenant_id=`

**BFF types.rs** に追加:
- `PostCommentCoreRequest { body, tenant_id, user_id }` — Serialize
- `WorkflowCommentDto { id, posted_by: UserRefDto, body, created_at }` — Deserialize

**BFF handler DTO**:
- `PostCommentRequest { body }` — BFF 公開 API（ToSchema）
- `WorkflowCommentData { id, posted_by: UserRefData, body, created_at }` — レスポンス（ToSchema）

**BFF ハンドラ**:
- `POST /api/v1/workflows/{display_number}/comments` — `post_comment`
- `GET /api/v1/workflows/{display_number}/comments` — `list_comments`

**OpenAPI 仕様書** `openapi/openapi.yaml` に追加:
- 2 エンドポイント + `PostCommentRequest` / `WorkflowCommentData` スキーマ

### Phase 6: API テスト（Hurl）

#### 確認事項
- [ ] パターン: API テストフロー（ログイン → CSRF → 操作） → `tests/api/hurl/workflow/approve_step.hurl`
- [ ] パターン: `vars.env` 変数 → `tests/api/hurl/vars.env`

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（`tests/api/hurl/workflow/comments.hurl`）:
- [ ] 正常系: 申請者がコメントを投稿できる（201）
- [ ] 正常系: 承認者がコメントを投稿できる（201）
- [ ] 正常系: コメント一覧を時系列で取得できる（200、投稿者名付き）
- [ ] 異常系: CSRF トークンなしでは投稿できない（403）
- [ ] 異常系: 関与していないユーザーは投稿できない（403）
- [ ] 異常系: 存在しないワークフローにはコメントできない（404）
- [ ] 異常系: 空文字のコメント本文はエラー（400）

E2E テスト（該当なし — フロントエンド対象外）

#### テストフロー

1. admin でログイン → CSRF 取得
2. ワークフロー作成・申請（user を承認者に指定）
3. admin（申請者）がコメント投稿 → 201
4. CSRF なしでコメント投稿 → 403
5. user でログイン → CSRF 取得
6. user（承認者）がコメント投稿 → 201
7. コメント一覧取得 → 200、2件、時系列順
8. 存在しないワークフローにコメント → 404
9. 空文字のコメント → 400

## 変更対象ファイル一覧

### 新規

| ファイル | 内容 |
|---------|------|
| `backend/migrations/YYYYMMDDHHMMSS_create_workflow_comments.sql` | マイグレーション |
| `backend/crates/domain/src/workflow/comment.rs` | ドメインモデル |
| `backend/crates/infra/src/repository/workflow_comment_repository.rs` | リポジトリ trait + 実装 |
| `backend/crates/infra/tests/workflow_comment_repository_test.rs` | リポジトリ統合テスト |
| `tests/api/hurl/workflow/comments.hurl` | API テスト |

### 変更

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/src/workflow.rs` | `mod comment;` 追加 |
| `backend/crates/infra/src/repository.rs` | モジュール追加 + re-export |
| `backend/crates/infra/src/mock.rs` | `MockWorkflowCommentRepository` 追加 |
| `backend/crates/infra/src/deletion/postgres_workflow.rs` | `workflow_comments` DELETE 追加 |
| `backend/crates/infra/tests/common/mod.rs` | `create_test_comment()` ヘルパー追加 |
| `backend/apps/core-service/src/usecase/workflow.rs` | `comment_repo` 依存追加 |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | `post_comment` + `is_participant` 追加 |
| `backend/apps/core-service/src/usecase/workflow/query.rs` | `list_comments` 追加 |
| `backend/apps/core-service/src/handler/workflow.rs` | DTO 型追加 |
| `backend/apps/core-service/src/handler/workflow/command.rs` | `post_comment` ハンドラ追加 |
| `backend/apps/core-service/src/handler/workflow/query.rs` | `list_comments` ハンドラ追加 |
| `backend/apps/core-service/src/main.rs` | ルート + DI 変更 |
| `backend/apps/bff/src/client/core_service/workflow_client.rs` | トレイト + 実装メソッド追加 |
| `backend/apps/bff/src/client/core_service/types.rs` | リクエスト/レスポンス型追加 |
| `backend/apps/bff/src/handler/workflow.rs` | BFF DTO 型追加 |
| `backend/apps/bff/src/handler/workflow/command.rs` | `post_comment` ハンドラ追加 |
| `backend/apps/bff/src/handler/workflow/query.rs` | `list_comments` ハンドラ追加 |
| `backend/apps/bff/src/main.rs` | ルート追加 |
| `openapi/openapi.yaml` | エンドポイント + スキーマ追加 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | テナント削除レジストリの更新漏れ | 不完全なパス | Phase 2 に `postgres_workflow.rs` 更新を追加 |
| 1回目 | `posted_by` FK の ON DELETE 未定義 | 未定義 | `ON DELETE RESTRICT` 選択、理由を記載 |
| 2回目 | RLS ポリシー追加漏れ | 不完全なパス | マイグレーションに RLS ENABLE + ポリシー作成追加 |
| 2回目 | Mock リポジトリの追加漏れ | 未定義 | Phase 2 に `mock.rs` 更新を明記 |
| 3回目 | `WorkflowUseCaseImpl::new()` パラメータ変更の既存テスト影響 | 競合 | 既存テストに `comment_repo` 追加が必要（Mock デフォルト値で対応） |
| 3回目 | `tenant_id` 直接保持 vs JOIN | 既存手段の見落とし | `workflow_steps` の既存パターン（直接保持 + RLS）を踏襲 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue 完了基準 5 項目すべてに対応する Phase が存在。全レイヤー（DB → Domain → Infra → UseCase → Handler → BFF → OpenAPI → Test）をカバー |
| 2 | 曖昧さ排除 | OK | 各 Phase の実装内容にコードスニペット（型定義、SQL、メソッドシグネチャ）を含む。不確定表現なし |
| 3 | 設計判断の完結性 | OK | 5 つの設計判断に選択理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明示。編集/削除/リアクション/通知/フロントエンドは対象外 |
| 5 | 技術的前提 | OK | RLS ポリシー、FK 制約の ON DELETE、テナント退会時の削除順序、CSRF 検証を考慮 |
| 6 | 既存ドキュメント整合 | OK | 機能仕様書 4.7（1〜2000文字、関与者のみ、時系列表示）に準拠。CQRS、テナント分離、UUID v7 パターンと整合 |

## 検証方法

1. `just check` — コンパイル + lint + ユニットテスト
2. `just test-rust-integration` — リポジトリ統合テスト
3. `just check-all` — 全テスト（API テスト含む）
4. `just sqlx-prepare` — sqlx キャッシュ更新
