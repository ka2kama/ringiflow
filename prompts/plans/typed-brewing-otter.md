# 実装計画: #769 ユーザー作成・編集 API を role_id ベースに変更する

## Context

ユーザー作成・編集 API が `role_name`（ロール名文字列）を受け取る現在の設計を `role_id`（UUID）ベースに変更する。暫定修正 (#770) でフロントエンドを現行 API に合わせたが、変数名 `selectedRoleId` のまま `role.name` を送信するねじれた状態にある。REST API の慣例として ID ベース参照が適切であり、ロール名変更への堅牢性も向上する。

## スコープ

対象:
- 全レイヤー（リポジトリ / Core Service / BFF / フロントエンド / OpenAPI）の `role_name` → `role_id` 変更

対象外:
- `find_role_by_name` メソッドのトレイトからの削除（将来用途あり）
- レスポンスの `roles: Vec<String>` の ID 化（別 Issue）
- `count_active_users_with_role` の `role_name` パラメータ変更

## 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 管理者がユーザー作成画面でロールを選択し、ユーザーを作成する | 正常系 | 手動確認（E2E テスト未整備） |
| 2 | 管理者がユーザー編集画面でロールを変更し、保存する | 正常系 | 手動確認 |
| 3 | 存在しない role_id でユーザーを作成しようとする | 準正常系 | ユースケース内で検証 |
| 4 | 不正な UUID 形式の role_id でリクエストする | 異常系 | BFF でパースエラー / Core は axum のデシリアライズで拒否 |

## Phase 1: リポジトリ層 — `find_role_by_id` メソッドの追加

#### 確認事項
- 型: `RoleId` → `backend/crates/domain/src/role.rs` (行49-52)
- パターン: `find_role_by_name` の SQL + `Role::from_db` → `backend/crates/infra/src/repository/user_repository.rs` (行568-596)

#### 変更内容

1. `UserRepository` トレイトに `find_role_by_id(&self, id: &RoleId)` を追加 (行120付近)
2. `PostgresUserRepository` に実装（`WHERE id = $1` に変更するだけ）
3. `MockUserRepository` にスタブ追加 (`backend/crates/infra/src/mock.rs` 行466付近)
4. Core Service テストの `StubUserRepository` にスタブ追加 (`backend/apps/core-service/src/handler/auth/tests.rs`)

#### テストリスト

ユニットテスト（該当なし）

統合テスト (`backend/crates/infra/tests/user_repository_test.rs`):
- [ ] 存在するロール ID で検索し、ロール情報が取得できる
- [ ] 存在しないロール ID で検索し、None が返る

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: Core Service — ユースケースとハンドラの変更

#### 確認事項
- 型: `CreateUserInput`, `UpdateUserInput` → `backend/apps/core-service/src/usecase/user.rs` (行16-28)
- 型: `CreateUserRequest`, `UpdateUserRequest` → `backend/apps/core-service/src/handler/auth/mod.rs` (行143-155)
- パターン: `create_user` ハンドラの Input 組み立て → 同ファイル (行405-411)
- パターン: `CreateUserResponseDto.role` の値設定 → 同ファイル (行420) — `req.role_name` を直接使用

#### 設計判断

`CreateUserResponseDto.role` にロール名を返すために、`create_user` ユースケースの戻り値を `User` → `(User, Role)` に変更する。

理由: ユースケース内で既にロールを取得済み（行79-85）。ハンドラで再度 `find_role_by_id` を呼ぶのは無駄な DB アクセス。

#### 変更内容

1. ユースケース Input 型: `role_name: String` → `role_id: RoleId`、`role_name: Option<String>` → `role_id: Option<RoleId>`
2. ユースケース `create_user`: `find_role_by_name` → `find_role_by_id`、戻り値を `Result<(User, Role), CoreError>` に変更
3. ユースケース `update_user`: `find_role_by_name` → `find_role_by_id`（戻り値は `User` のまま）
4. Core Service ハンドラ: `CreateUserRequest.role_name` → `role_id: Uuid`、`UpdateUserRequest.role_name` → `role_id: Option<Uuid>`
5. ハンドラの Input 組み立て: `RoleId::from_uuid(req.role_id)` で変換
6. ハンドラのレスポンス: `let (user, role) = ...` で受け取り、`role: role.name().to_string()`

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト:
- [ ] 既存テスト（`get_user_by_email`, `get_user`）がコンパイル・パスすること（StubUserRepository の変更後）

API テスト（該当なし — Core Service は BFF 経由でテスト）
E2E テスト（該当なし）

## Phase 3: BFF — リクエスト型と変換ロジックの変更

#### 確認事項
- 型: BFF `CreateUserRequest`, `UpdateUserRequest` → `backend/apps/bff/src/handler/user.rs` (行56-69)
- 型: `CreateUserCoreRequest`, `UpdateUserCoreRequest` → `backend/apps/bff/src/client/core_service/types.rs` (行40-64)
- パターン: `create_user` ハンドラの CoreRequest 組み立て → `backend/apps/bff/src/handler/user.rs` (行233-238)
- パターン: BFF での UUID パースエラーハンドリング → 既存パターンを Grep で探す

#### 変更内容

1. BFF リクエスト型: `role_name: String` → `role_id: String`
2. Core リクエスト型: `role_name: String` → `role_id: Uuid`
3. `create_user` ハンドラ: `req.role_id.parse::<Uuid>()` でパースし、エラー時は 400 Bad Request
4. `update_user` ハンドラ: `req.role_id.map(|id| id.parse::<Uuid>()).transpose()` で Optional パース

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし — ユーザー作成・編集の API テストは未整備）
E2E テスト（該当なし）

## Phase 4: フロントエンド — JSON フィールド名と select value の変更

#### 確認事項
- パターン: `New.elm` の JSON 送信 → `frontend/src/Page/User/New.elm` (行127-131)
- パターン: `Edit.elm` の JSON 送信 → `frontend/src/Page/User/Edit.elm` (行172-176)
- パターン: select の options 生成 → `New.elm` (行332), `Edit.elm` (行308) — 現在 `value = role.name`

#### 設計判断

`Edit.elm` の `selectedRoleId` 初期値問題: ユーザー詳細 API は `roles: Vec<String>`（ロール名配列）を返す。ロール一覧 API の `RoleItem` と名前でマッチして ID を解決する `resolveRoleId` ヘルパー関数を追加する。`GotUserDetail` と `GotRoles` の到着順序に関わらず正しく動作するよう、両方のブランチで解決を試みる。

#### 変更内容

1. `New.elm`: `( "role_name", ... )` → `( "role_id", ... )` (行130)
2. `New.elm`: `options = ... { value = role.name, ...}` → `{ value = role.id, ...}` (行332)
3. `Edit.elm`: `( "role_name", ... )` → `( "role_id", ... )` (行175)
4. `Edit.elm`: `options = ... { value = role.name, ...}` → `{ value = role.id, ...}` (行308)
5. `Edit.elm`: `resolveRoleId` ヘルパー関数を追加（ロール名 → ロール ID の変換）
6. `Edit.elm`: `GotUserDetail` と `GotRoles` の両方で `resolveRoleId` を適用

#### テストリスト

ユニットテスト（該当なし — Elm テスト環境未構築）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — ユーザー管理の E2E テスト未整備）

## Phase 5: OpenAPI — 仕様書の更新

#### 確認事項
- パターン: `CreateUserRequest` スキーマ → `openapi/openapi.yaml` (行2443-2456)
- パターン: `UpdateUserRequest` スキーマ → 同ファイル (行2767-2778)

#### 変更内容

1. OpenAPI: `role_name: string` → `role_id: string (format: uuid)` — CreateUserRequest, UpdateUserRequest 両方
2. `just openapi-generate` で OpenAPI YAML を再生成
3. BFF の OpenAPI スナップショット更新（`cargo insta review`）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト:
- [ ] `just check-all` で全テスト（OpenAPI スナップショット含む）がパスすること

E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `CreateUserResponseDto.role` にロール名を返す必要があるが `create_user` が `User` のみ返す | 不完全なパス | ユースケースの戻り値を `(User, Role)` に変更 |
| 2回目 | `Edit.elm` の `selectedRoleId` 初期値がロール名で設定される | 状態網羅漏れ | `resolveRoleId` ヘルパー関数を追加 |
| 3回目 | BFF で `role_id` の UUID パース時のエラーハンドリングが必要 | 不完全なパス | BFF ハンドラにパースエラーハンドリングを追加 |
| 4回目 | Mock/Stub すべてに `find_role_by_id` メソッド追加が必要 | 未定義 | Phase 1 で明記 |
| 5回目 | `GotRoles` が `GotUserDetail` より先に到着した場合のエッジケース | 競合・エッジケース | 両ブランチで `resolveRoleId` を適用 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | リポジトリ / Core Service（ユースケース＋ハンドラ）/ BFF（ハンドラ＋クライアント型）/ フロントエンド（New.elm, Edit.elm）/ OpenAPI の全レイヤーを網羅。Mock/Stub 更新も含む |
| 2 | 曖昧さ排除 | OK | 各 Phase で変更ファイル・行番号・具体的なコード変更を明示 |
| 3 | 設計判断の完結性 | OK | create_user 戻り値変更、Edit.elm の ID 解決、BFF UUID パースの 3 判断を記載 |
| 4 | スコープ境界 | OK | 対象（全レイヤー role_name→role_id）と対象外（find_role_by_name 削除、レスポンス ID 化）を明記 |
| 5 | 技術的前提 | OK | RoleId の define_uuid_id! マクロ、axum の Uuid デシリアライズ、Elm の JSON エンコードパターンを確認済み |
| 6 | 既存ドキュメント整合 | OK | OpenAPI 更新を Phase 5 で明示。既存 ADR との矛盾なし |

## 検証方法

1. `just check-all` で全テスト（ユニット / ハンドラ / API / OpenAPI スナップショット）がパスすること
2. `just dev-all` で開発サーバーを起動し、ユーザー作成・編集が正常に動作することを手動確認
