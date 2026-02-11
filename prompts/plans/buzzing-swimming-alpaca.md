# Story #431: ユーザー管理画面・監査ログ閲覧画面（Elm フロントエンド）

## Context

Phase 2-2 の Story #427〜#430 でバックエンド API（ユーザー CRUD、ロール管理、監査ログ）が完成した。
Story #431 では、これらの API を消費する Elm フロントエンド画面を実装する。

**意図**: テナント管理者がブラウザからユーザー管理・ロール管理・監査ログ閲覧を行えるようにする。

## スコープ

**対象:**
- ユーザー一覧・詳細・作成・編集・ステータス変更画面
- ロール一覧・作成・編集（詳細統合）・削除画面
- 監査ログ一覧画面（フィルタ・カーソルページネーション・インライン展開）
- サイドバーへの管理メニュー追加（権限制御付き）
- 必要な基盤（Route, Data型, Api モジュール, Shared 拡張）

**対象外:**
- 監査ログの「前のページ」ボタン（カーソル履歴管理。将来対応）
- 日付ピッカーライブラリ（`<input type="date">` で対応）
- ユーザー一覧のソート機能（初期実装では不要）
- E2E テスト（別途対応）

## 設計判断

### DJ-1: 作成/編集は別ページ（モーダルではない）
既存パターン（`Workflow.New` は別ページ）に準拠。TEA パターンとの親和性が高い。

### DJ-2: ユーザー作成は単一フォーム
入力項目が3つ（email, name, role）のみ。3ステップウィザードは過剰。

### DJ-3: ロール詳細と編集を統合
システムロールは読み取り専用表示、カスタムロールは編集可能フォーム表示。ページ数を削減。

### DJ-4: カーソルページネーションは「次のページ」のみ
API が `next_cursor` のみ返す。「前のページ」はカーソル履歴管理が必要で初期スコープ外。

### DJ-5: 権限マトリクスは独立コンポーネント
`Component/PermissionMatrix.elm` として分離。リソース×アクションのチェックボックスグリッド + 行ごとの「すべて選択」。

## Phase 分割

| Phase | 内容 | 新規ファイル数 |
|-------|------|-------------|
| 1 | 基盤（Route, Data型, Api, Shared, サイドバー） | ~8 |
| 2 | ユーザー一覧・詳細（ステータス変更含む） | 2 |
| 3 | ユーザー作成・編集 | 2 |
| 4 | ロール一覧・作成・編集・削除 + PermissionMatrix | 4 |
| 5 | 監査ログ一覧（フィルタ・ページネーション・インライン展開） | 1 |

各 Phase で Main.elm への統合も行う（最後にまとめない）。

---

## Phase 1: 基盤

Route, Data 型, Api モジュール, Shared 拡張, サイドバー管理セクション追加。

### 確認事項
- 型: `Shared.User.roles` → `frontend/src/Shared.elm:47`（確認済み: `roles : List String`）
- パターン: `Api.get/post/put/delete` → `frontend/src/Api.elm`（確認済み: `patch` なし）
- パターン: Data デコーダーパイプライン → `frontend/src/Data/UserItem.elm`（確認済み）
- パターン: ルート定義 → `frontend/src/Route.elm`（確認済み: oneOf + パーサーコンビネータ）
- パターン: サイドバー → `frontend/src/Main.elm:607`（確認済み: `viewNavItem` パターン）

### 変更ファイル

**`frontend/src/Route.elm`** — ルート追加:
```elm
type Route
    = ...既存...
    | Users
    | UserDetail Int          -- display_number
    | UserNew
    | UserEdit Int            -- display_number
    | Roles
    | RoleNew
    | RoleEdit String         -- role_id (UUID)
    | AuditLogs
    | NotFound
```

パーサー（順序重要: 具体的なパスを先に）:
```elm
, Parser.map UserNew (s "users" </> s "new")
, Parser.map UserEdit (s "users" </> int </> s "edit")
, Parser.map UserDetail (s "users" </> int)
, Parser.map Users (s "users")
, Parser.map RoleNew (s "roles" </> s "new")
, Parser.map RoleEdit (s "roles" </> Parser.string </> s "edit")
, Parser.map (always Roles) (s "roles")  -- string パーサーとの衝突回避のため注意
, Parser.map AuditLogs (s "audit-logs")
```

注意: `Roles` と `RoleEdit String` のパーサー順序。`s "roles" </> string </> s "edit"` を先に置く。`Roles` は `s "roles"` のみで、`/roles/some-uuid` は RoleEdit にマッチしない（`/edit` が続く必要がある）ため問題なし。ただし `RoleDetail` ルートがないので `/roles/{id}` は NotFound になる。これは DJ-3（詳細と編集の統合）と整合。

`toString`, `isRouteActive`, `pageTitle` も対応更新。
- `isRouteActive`: `Users` は `UserDetail`, `UserNew`, `UserEdit` の親。`Roles` は `RoleNew`, `RoleEdit` の親。

**`frontend/src/Api.elm`** — `patch` と `deleteNoContent` 追加:

```elm
-- patch: put と同じシグネチャ、method = "PATCH"
patch :
    { config : RequestConfig, url : String, body : Http.Body
    , decoder : Decoder a, toMsg : Result ApiError a -> msg }
    -> Cmd msg

-- deleteNoContent: 204 No Content 用。decoder 不要。
deleteNoContent :
    { config : RequestConfig, url : String
    , toMsg : Result ApiError () -> msg }
    -> Cmd msg
```

`deleteNoContent` は `Http.expectStringResponse` で 204 を `Ok ()` に変換する。
`handleResponse` と同様のエラーハンドリングだが、成功時はボディを無視。

**`frontend/src/Shared.elm`** — `isAdmin` 追加:
```elm
isAdmin : Shared -> Bool
isAdmin shared =
    case shared.user of
        Just user ->
            List.member "admin" user.roles
        Nothing ->
            False
```

**`frontend/src/Data/AdminUser.elm`** — 新規:

ユーザー管理専用の型。既存 `Data.UserItem` は承認者選択用として温存。

```elm
type alias AdminUserItem =
    { id : String, displayId : String, displayNumber : Int
    , name : String, email : String, status : String, roles : List String }

type alias UserDetail =
    { id : String, displayId : String, displayNumber : Int
    , name : String, email : String, status : String
    , roles : List String, permissions : List String, tenantName : String }

type alias CreateUserResponse =
    { id : String, displayId : String, displayNumber : Int
    , name : String, email : String, role : String, initialPassword : String }

type alias UserResponse =
    { id : String, name : String, email : String, status : String }
```

各型に `decoder`, `listDecoder`（AdminUserItem のみ）を定義。
API レスポンスの `{ "data": ... }` ラッパーに対応。

**`frontend/src/Data/Role.elm`** — 新規:

```elm
type alias RoleItem =
    { id : String, name : String, description : Maybe String
    , permissions : List String, isSystem : Bool, userCount : Int }

type alias RoleDetail =
    { id : String, name : String, description : Maybe String
    , permissions : List String, isSystem : Bool
    , createdAt : String, updatedAt : String }
```

**`frontend/src/Data/AuditLog.elm`** — 新規:

```elm
type alias AuditLogItem =
    { id : String, actorId : String, actorName : String
    , action : String, result : String
    , resourceType : String, resourceId : String
    , detail : Maybe Decode.Value, sourceIp : Maybe String
    , createdAt : String }

type alias AuditLogList =
    { data : List AuditLogItem, nextCursor : Maybe String }

-- actionToJapanese, resultToJapanese, resultToCssClass ヘルパー
```

`AuditLogList` のデコーダーは `next_cursor` が nullable であることに注意（`optional` を使用）。

**`frontend/src/Api/AdminUser.elm`** — 新規:

```elm
listAdminUsers : { config, statusFilter : Maybe String, toMsg } -> Cmd msg
getUserDetail : { config, displayNumber : Int, toMsg } -> Cmd msg
createUser : { config, body : Encode.Value, toMsg } -> Cmd msg
updateUser : { config, displayNumber : Int, body : Encode.Value, toMsg } -> Cmd msg
updateUserStatus : { config, displayNumber : Int, body : Encode.Value, toMsg } -> Cmd msg
```

**`frontend/src/Api/Role.elm`** — 新規:

```elm
listRoles : { config, toMsg } -> Cmd msg
getRole : { config, roleId : String, toMsg } -> Cmd msg
createRole : { config, body : Encode.Value, toMsg } -> Cmd msg
updateRole : { config, roleId : String, body : Encode.Value, toMsg } -> Cmd msg
deleteRole : { config, roleId : String, toMsg } -> Cmd msg
```

**`frontend/src/Api/AuditLog.elm`** — 新規:

```elm
type alias AuditLogFilter =
    { cursor : Maybe String, limit : Int
    , from : Maybe String, to : Maybe String
    , actorId : Maybe String, action : Maybe String
    , result : Maybe String }

listAuditLogs : { config, filter : AuditLogFilter, toMsg } -> Cmd msg
```

クエリパラメータはフィルタの各フィールドから構築（`Nothing` のフィールドは除外）。

**`frontend/src/Main.elm`** — サイドバーに管理セクション追加:

```elm
-- viewSidebar の nav 内に追加:
, if Shared.isAdmin shared then
    div []
        [ div [ class "mt-6 px-3 text-xs font-semibold uppercase tracking-wider text-secondary-500" ]
            [ text "管理" ]
        , viewNavItem currentRoute Route.Users "ユーザー管理" iconUsers
        , viewNavItem currentRoute Route.Roles "ロール管理" iconRoles
        , viewNavItem currentRoute Route.AuditLogs "監査ログ" iconAuditLog
        ]
  else
    text ""
```

SVG アイコン追加: `iconUsers`（People）, `iconRoles`（Shield）, `iconAuditLog`（ClipboardList）。

Page 型, Msg 型, initPage, updatePageShared, update, viewPage に新ページの骨格を追加（Phase 2〜5 で各ページを実装）。

### テストリスト

- [ ] Route.fromUrl: `/users` → `Users`
- [ ] Route.fromUrl: `/users/5` → `UserDetail 5`
- [ ] Route.fromUrl: `/users/new` → `UserNew`
- [ ] Route.fromUrl: `/users/5/edit` → `UserEdit 5`
- [ ] Route.fromUrl: `/roles` → `Roles`
- [ ] Route.fromUrl: `/roles/new` → `RoleNew`
- [ ] Route.fromUrl: `/roles/{uuid}/edit` → `RoleEdit uuid`
- [ ] Route.fromUrl: `/audit-logs` → `AuditLogs`
- [ ] Route.toString: 各ルートの往復テスト
- [ ] Route.isRouteActive: Users は UserDetail/UserNew/UserEdit の親
- [ ] Route.isRouteActive: Roles は RoleNew/RoleEdit の親
- [ ] Data.AdminUser.decoder: 全フィールドのデコード
- [ ] Data.AdminUser.listDecoder: `{ data: [...] }` ラッパー
- [ ] Data.Role.decoder: `is_system` の boolean デコード
- [ ] Data.AuditLog.decoder: `detail` の nullable JSON デコード
- [ ] Data.AuditLog.listDecoder: `next_cursor` の nullable デコード
- [ ] Shared.isAdmin: admin ロールを持つユーザー → True
- [ ] Shared.isAdmin: admin ロールを持たないユーザー → False
- [ ] Shared.isAdmin: 未ログイン → False

---

## Phase 2: ユーザー一覧・詳細画面

### 確認事項
- パターン: `Page.Task.List` の RemoteData パターン → `frontend/src/Page/Task/List.elm`
- パターン: `Page.Workflow.Detail` の詳細表示 + ConfirmDialog → `frontend/src/Page/Workflow/Detail.elm`
- 型: Phase 1 の `Data.AdminUser` 型

### 新規ファイル

**`frontend/src/Page/User/List.elm`**:
- Model: `shared`, `users : RemoteData ApiError (List AdminUserItem)`, `statusFilter : Maybe String`
- ステータスフィルタ（すべて / アクティブ / 非アクティブ）
- 「ユーザーを追加」ボタン → `/users/new` へリンク
- テーブル: 表示番号、名前、メール、ロール、ステータスバッジ
- 行クリック → `/users/{display_number}` へ遷移
- ステータスバッジ: Active = 緑、Inactive = グレー（`Badge` コンポーネント利用）
- API 呼び出し: `Api.AdminUser.listAdminUsers`

**`frontend/src/Page/User/Detail.elm`**:
- Model: `shared`, `user : RemoteData ApiError UserDetail`, `successMessage`, `errorMessage`, `confirmAction`
- 基本情報セクション（表示番号、名前、メール、ステータス）
- ロール・権限セクション
- アクションボタン:
  - 「編集」→ `/users/{display_number}/edit`
  - 「無効化」/ 「有効化」→ ConfirmDialog → API 呼び出し
- 自己無効化防止: `Shared.getUserId shared == Just user.id` の場合、無効化ボタン非表示
- ステータス変更成功時: MessageAlert 表示 + ユーザー情報再取得
- API 呼び出し: `Api.AdminUser.getUserDetail`, `Api.AdminUser.updateUserStatus`

### Main.elm 変更
- `UsersPage UserList.Model | UserDetailPage UserDetail.Model` を Page に追加
- `UsersMsg UserList.Msg | UserDetailMsg UserDetail.Msg` を Msg に追加
- initPage, update, viewPage, updatePageShared の対応ケース追加

### テストリスト

- [ ] ユーザーステータスバッジのマッピング（status → Badge config）
- [ ] 自己無効化防止ロジック（自分の ID と一致する場合はボタン非表示）

---

## Phase 3: ユーザー作成・編集

### 確認事項
- パターン: `Page.Workflow.New` のフォーム + dirty tracking → `frontend/src/Page/Workflow/New.elm`
- パターン: `Form.Validation` → `frontend/src/Form/Validation.elm`
- ライブラリ: `Json.Encode` のエンコーダーパターン → Grep `Encode.object` in frontend/src

### 新規ファイル

**`frontend/src/Page/User/New.elm`**:
- Model: `shared`, `email`, `name`, `selectedRoleId`, `roles : RemoteData`, `validationErrors`, `submitting`, `createdUser : Maybe CreateUserResponse`
- init 時にロール一覧取得（`Api.Role.listRoles`）
- フォーム: email（text）, name（text）, role（select、ロール一覧から取得）
- フロントエンドバリデーション: email 形式、name 必須（1-100文字）、role 必須
- 作成成功後: 初期パスワード表示画面（`createdUser` が `Just` の時）
  - パスワードをコピー可能なテキストで表示
  - 「ユーザー一覧に戻る」ボタン
- サーバーエラー: メール重複 (409) → MessageAlert
- dirty tracking: `Workflow.New` と同様のパターン + `Ports.setBeforeUnloadEnabled`
- API: `Api.AdminUser.createUser`

**`frontend/src/Page/User/Edit.elm`**:
- Model: `shared`, `displayNumber`, `user : RemoteData`, `name`, `selectedRoleId`, `roles : RemoteData`, `validationErrors`, `submitting`, `successMessage`, `isDirty`
- init 時にユーザー詳細 + ロール一覧を並行取得
- 編集可能: name, role（email は表示のみ）
- dirty tracking + beforeUnload
- 保存成功後: ユーザー詳細画面に遷移（`Nav.pushUrl`）
- API: `Api.AdminUser.getUserDetail`, `Api.AdminUser.updateUser`, `Api.Role.listRoles`

### Main.elm 変更
- `UserNewPage UserNew.Model | UserEditPage UserEdit.Model` を Page に追加
- 対応する Msg, initPage, update, viewPage, updatePageShared, isCurrentPageDirty 追加

### テストリスト

- [ ] メールバリデーション: 形式チェック（@ 含む、基本的な形式）
- [ ] 名前バリデーション: 空 → エラー、101文字 → エラー
- [ ] ロール未選択バリデーション
- [ ] CreateUserRequest JSON エンコード
- [ ] UpdateUserRequest JSON エンコード（Optional フィールド処理）

---

## Phase 4: ロール管理

### 確認事項
- 型: `Data.Role.RoleItem`, `Data.Role.RoleDetail` → Phase 1 で定義
- パターン: `ConfirmDialog` の使用 → `frontend/src/Page/Workflow/Detail.elm`
- API: `Api.deleteNoContent` → Phase 1 で追加

### 新規ファイル

**`frontend/src/Component/PermissionMatrix.elm`**:

リソース×アクションのチェックボックスグリッド。

```elm
type alias Config msg =
    { selectedPermissions : Set String
    , onToggle : String -> msg      -- "workflow:read" 等
    , onToggleAll : String -> msg   -- "workflow" (リソース名)
    , disabled : Bool
    }

view : Config msg -> Html msg
```

権限定義:
```elm
resources = [ ("workflow", "ワークフロー"), ("task", "タスク") ]
actions = [ ("read", "閲覧"), ("create", "作成"), ("update", "更新"), ("delete", "削除") ]
```

「すべて選択」チェックボックス: リソースの全アクションが選択済みなら checked。
トグル時: `resource:*` 権限がある場合はそのリソースの全個別権限に展開。

**`frontend/src/Page/Role/List.elm`**:
- Model: `shared`, `roles : RemoteData ApiError (List RoleItem)`, `deleteState`
- セクション分け: システムロール（`is_system = true`）/ カスタムロール（`is_system = false`）
- テーブル: ロール名、説明、種別バッジ、ユーザー数
- 「ロールを追加」ボタン → `/roles/new`
- カスタムロール行クリック → `/roles/{id}/edit`
- 削除: 行内の削除ボタン → ConfirmDialog → API
  - ユーザー割り当て中 (409) → エラーメッセージ表示
- API: `Api.Role.listRoles`, `Api.Role.deleteRole`

**`frontend/src/Page/Role/New.elm`**:
- Model: `shared`, `name`, `description`, `selectedPermissions : Set String`, `validationErrors`, `submitting`
- フォーム: name（text）, description（textarea、任意）, permissions（PermissionMatrix）
- バリデーション: name 必須、permissions 1つ以上
- 作成成功後: ロール一覧に遷移
- API: `Api.Role.createRole`

**`frontend/src/Page/Role/Edit.elm`**:
- DJ-3: 詳細と編集を統合。
- Model: `shared`, `roleId`, `role : RemoteData`, `name`, `description`, `selectedPermissions`, `validationErrors`, `submitting`, `isDirty`, `isReadOnly` (system role)
- init: `Api.Role.getRole` でロール取得。`isSystem` なら `isReadOnly = True`
- システムロール: フォーム全体が disabled、保存ボタンなし
- カスタムロール: 編集可能、dirty tracking
- API: `Api.Role.getRole`, `Api.Role.updateRole`

### Main.elm 変更
- `RolesPage`, `RoleNewPage`, `RoleEditPage` を Page に追加
- 対応する Msg, initPage, update, viewPage, updatePageShared, isCurrentPageDirty 追加

### テストリスト

- [ ] PermissionMatrix: 個別権限トグル
- [ ] PermissionMatrix: 「すべて選択」トグル（全選択 → 全解除）
- [ ] PermissionMatrix: 一部選択状態から「すべて選択」→ 全選択
- [ ] ロール一覧のシステム/カスタム分割ロジック
- [ ] CreateRoleRequest JSON エンコード
- [ ] UpdateRoleRequest JSON エンコード

---

## Phase 5: 監査ログ一覧

### 確認事項
- 型: `Data.AuditLog` → Phase 1 で定義
- パターン: `Util.DateFormat` → `frontend/src/Util/DateFormat.elm`
- API: クエリパラメータ構築パターン → Phase 1 の `Api.AuditLog`

### 新規ファイル

**`frontend/src/Page/AuditLog/List.elm`**:
- Model:
  - `shared`
  - `auditLogs : RemoteData ApiError AuditLogList`
  - `filter`: from, to, actorId, action, result
  - `expandedId : Maybe String` — インライン展開中のログ ID
  - `users : RemoteData ApiError (List AdminUserItem)` — アクターフィルタ用ユーザー一覧
- フィルタセクション:
  - 期間: `<input type="date">` × 2（from, to）
  - ユーザー: `<select>`（ユーザー一覧から取得）
  - アクション: `<select>`（固定の選択肢リスト）
  - 結果: `<select>`（すべて / 成功 / 失敗）
  - 「検索」ボタン → フィルタ適用して API 再取得
- テーブル:
  - 日時（`Util.DateFormat` でフォーマット）
  - ユーザー（`actorName`）
  - アクション（`actionToJapanese` で日本語表示）
  - 対象（`resourceType` + `resourceId`）
  - 結果バッジ（成功=緑, 失敗=赤）
- 行クリック → インライン展開（アコーディオン）:
  - 操作詳細（`detail` JSON を整形表示）
  - リソース ID
  - リクエスト元 IP（`sourceIp`、なければ「未取得」）
- ページネーション:
  - 「次のページ」ボタン（`nextCursor` が `Just` の時のみ表示）
  - ボタン押下 → `cursor` をセットして API 再取得
- init: 監査ログ取得 + ユーザー一覧取得（フィルタ用）を並行実行
- API: `Api.AuditLog.listAuditLogs`, `Api.AdminUser.listAdminUsers`

### Main.elm 変更
- `AuditLogsPage AuditLogList.Model` を Page に追加
- 対応する Msg, initPage, update, viewPage, updatePageShared 追加

### テストリスト

- [ ] actionToJapanese: 全アクション名の日本語変換
- [ ] resultToJapanese / resultToCssClass: 結果マッピング
- [ ] AuditLogList デコーダー: next_cursor が null のケース
- [ ] AuditLogList デコーダー: next_cursor が string のケース
- [ ] フィルタクエリ文字列構築ロジック（None フィールドは除外）

---

## 検証方法

各 Phase 完了時:
1. `just check` でコンパイル + テスト通過を確認
2. `just dev-all` で開発サーバー起動、ブラウザで画面確認

全 Phase 完了後:
1. `just check-all` で全体チェック
2. ブラウザで E2E 手動確認:
   - テナント管理者でログイン → サイドバーに管理メニューが表示される
   - ユーザー一覧 → 作成 → 詳細 → 編集 → 無効化 の一連フロー
   - ロール一覧 → 作成 → 編集 → 削除 の一連フロー
   - 監査ログ一覧 → フィルタ適用 → インライン展開 → ページネーション
   - 一般ユーザーでログイン → サイドバーに管理メニューが表示されない
3. Issue #431 の完了基準すべてをチェック

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `Api.elm` に `patch` メソッドがない。PATCH エンドポイントが3つある | 既存手段の見落とし | Phase 1 で `Api.patch` を追加 |
| 1回目 | `Api.delete` が decoder を要求するが、ロール削除は 204 No Content | 不完全なパス | Phase 1 で `Api.deleteNoContent` を追加 |
| 2回目 | 既存 `Data.UserItem` に `status`/`roles` がない | 未定義 | `Data.AdminUser` として管理用の拡張型を新設。既存は承認者選択用に温存 |
| 2回目 | `Roles` ルートと `RoleEdit String` のパーサー衝突の可能性 | 競合・エッジケース | パーサー順序を確認: `RoleEdit`（`/roles/{string}/edit`）は `/edit` サフィックスが必要なので `Roles`（`/roles`）とは衝突しない |
| 3回目 | Phase 6 を最後にまとめると各 Phase がテスト不能 | アーキテクチャ不整合 | Main.elm 変更を各 Phase に分散。最後にまとめない |
| 3回目 | `detail` フィールドの JSON 整形表示方法が未定義 | 曖昧 | `Json.Decode.Value` として保持し、`Json.Encode.encode 2` で整形文字列に変換して表示 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue #431 の完了基準が全て計画に含まれている | OK | ユーザー CRUD + ロール CRUD + 監査ログ閲覧 + 権限制御。4つの完了基準すべてに対応する Phase がある |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 5つの設計判断（DJ-1〜DJ-5）を明示的に記載。スコープ外を明記 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | フォーム形式、ページ構成、ページネーション方式、コンポーネント分離の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | 対象外セクションに4項目を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | `Api.patch` 未実装、`deleteNoContent` 必要性、`<input type="date">` の Elm 対応、パーサー順序を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 機能仕様書（02, 03）、OpenAPI 仕様書、既存コードパターンと照合済み |
