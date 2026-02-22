# #769 ユーザー作成・編集 API を role_id ベースに変更

## 概要

ユーザー作成・編集 API が `role_name`（ロール名文字列）を受け取る設計を `role_id`（UUID）ベースに変更した。リポジトリ / Core Service / BFF / フロントエンド / OpenAPI の全レイヤーを一貫して修正。

## 実施内容

### Phase 1: リポジトリ層

- `UserRepository` トレイトに `find_role_by_id` メソッドを追加
- `PostgresUserRepository` に SQL 実装（`WHERE id = $1`）
- `MockUserRepository`、`StubUserRepository` にスタブ追加
- 統合テスト 2 件追加（存在するロール ID / 存在しない ID）

### Phase 2: Core Service

- `CreateUserInput.role_name: String` → `role_id: RoleId`
- `UpdateUserInput.role_name: Option<String>` → `role_id: Option<RoleId>`
- `create_user` の戻り値を `Result<User, CoreError>` → `Result<(User, Role), CoreError>` に変更（ハンドラでロール名をレスポンスに含めるため）
- ユースケースの `find_role_by_name` → `find_role_by_id` に変更

### Phase 3: BFF

- リクエスト型の `role_name: String` → `role_id: String`
- Core リクエスト型の `role_name: String` → `role_id: Uuid`
- UUID パース + バリデーション追加（`uuid::Uuid::parse_str` で 400 Bad Request）
- `#[schema(format = "uuid")]` アノテーション追加（utoipa による OpenAPI 生成用）

### Phase 4: フロントエンド

- `New.elm`, `Edit.elm` の JSON フィールド名を `"role_name"` → `"role_id"` に変更
- select の `value` を `role.name` → `role.id` に変更
- `Edit.elm` に `resolveRoleId` ヘルパー関数を追加（ロール名 → ロール ID の変換）
- `GotUserDetail` と `GotRoles` の到着順序に関わらず正しく動作するよう両ブランチで解決

### Phase 5: OpenAPI と品質ゲート

- `just openapi-generate` で仕様を再生成
- OpenAPI スナップショット更新
- `cargo sqlx prepare --workspace` で新クエリのオフラインデータ更新
- `just check-all` パス

## 判断ログ

- `create_user` の戻り値を `(User, Role)` に変更: ユースケース内で既にロールを取得済みのため、ハンドラで再度 DB アクセスするより効率的
- `Edit.elm` の `resolveRoleId`: ユーザー詳細 API が `roles: Vec<String>`（ロール名配列）を返すため、ロール一覧とマッチして ID を解決する必要がある。両方の API レスポンス到着タイミングに対応

## 成果物

コミット:
- `3d4a4fa` #769 WIP: Change user create/edit API to role_id based
- `cd28f94` #769 Change user create/edit API from role_name to role_id based

変更ファイル（14 ファイル）:
- `backend/crates/infra/src/repository/user_repository.rs`（トレイト + 実装）
- `backend/crates/infra/tests/user_repository_test.rs`（統合テスト）
- `backend/crates/infra/src/mock.rs`（Mock スタブ）
- `backend/apps/core-service/src/handler/auth/tests.rs`（Stub スタブ）
- `backend/apps/core-service/src/usecase/user.rs`（ユースケース Input 型・ロジック）
- `backend/apps/core-service/src/handler/auth/mod.rs`（ハンドラ Request 型）
- `backend/apps/bff/src/handler/user.rs`（BFF ハンドラ + UUID パース）
- `backend/apps/bff/src/client/core_service/types.rs`（Core リクエスト型）
- `backend/apps/bff/tests/snapshots/openapi_spec__openapi_spec.snap`（スナップショット）
- `frontend/src/Page/User/New.elm`（JSON フィールド + select value）
- `frontend/src/Page/User/Edit.elm`（JSON フィールド + select value + resolveRoleId）
- `openapi/openapi.yaml`（自動生成）
- `backend/.sqlx/query-*.json`（sqlx オフラインデータ）
- `prompts/plans/typed-brewing-otter.md`（計画ファイル）

## 議論の経緯

- openapi.yaml を手動編集しようとした件について指摘を受けた。utoipa がソースオブトゥルースであり、生成物を直接編集すべきでない。対策の永続化方法についても議論が発生し、Issue #772（MEMORY.md 不使用のルール明文化）と Issue #773（生成物の直接編集防止）を作成
