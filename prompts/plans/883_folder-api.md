# #883 フォルダ管理 API 実装計画

## Context

Issue #883: フォルダの CRUD API を materialized path による階層構造管理で実装する。
親 Epic #406（ドキュメント管理）の一部。S3 基盤（#880）に依存しない独立 Story。

設計ドキュメント:
- 詳細設計: `docs/03_詳細設計書/17_ドキュメント管理設計.md`
- 機能仕様書: `docs/01_要件定義書/機能仕様書/06_ドキュメント管理.md`（4.2 フォルダ管理）

完了基準:
- フォルダの作成・名前変更・移動・削除ができる
- 階層制限（5 階層）が DB レベルで強制される
- 同一フォルダ内で名前の重複が防がれる
- 空でないフォルダは削除できない
- RLS ポリシーが適用されている

## 対象

| レイヤー | 対象 |
|---------|------|
| Domain | `Folder` エンティティ、`FolderId` 値オブジェクト、バリデーション |
| Infra | `FolderRepository` trait + PostgreSQL 実装、マイグレーション |
| Core Service | ハンドラ、ユースケース、ルート登録 |
| BFF | クライアント trait、ハンドラ、ルート登録 |

## 対象外

- フロントエンド（Elm）— #885 で実装
- documents テーブルとの連携（空フォルダチェックは子フォルダのみ。documents テーブルは #881 で作成）
- 監査ログ（フォルダ操作の監査ログは今回のスコープでは追加しない。BFF は認証のみ実装し、監査ログは後続で検討）

---

## Phase 1: ドメインモデル + ユニットテスト

### 概要

`FolderId`、`FolderName`（禁止文字バリデーション付き）、`Folder` エンティティを domain クレートに追加する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/crates/domain/src/folder.rs` | 新規 |
| `backend/crates/domain/src/lib.rs` | `pub mod folder;` 追加 |

### 設計

#### FolderId

```rust
// folder.rs
define_uuid_id! {
    /// フォルダの一意識別子
    pub struct FolderId;
}
```

#### FolderName

`define_validated_string!` は禁止文字チェックを持たないため、手動で Newtype を定義する。

```rust
/// フォルダ名（1〜255 文字、禁止文字チェック付き）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderName(String);

const FORBIDDEN_CHARS: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const MAX_FOLDER_NAME_LENGTH: usize = 255;

impl FolderName {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into().trim().to_string();
        if value.is_empty() { return Err(Validation(...)); }
        if value.chars().count() > MAX_FOLDER_NAME_LENGTH { return Err(Validation(...)); }
        if value.chars().any(|c| FORBIDDEN_CHARS.contains(&c)) { return Err(Validation(...)); }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str { &self.0 }
    pub fn into_string(self) -> String { self.0 }
}

impl Display for FolderName { ... }
```

#### Folder エンティティ

```rust
pub const MAX_FOLDER_DEPTH: i32 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    id: FolderId,
    tenant_id: TenantId,
    name: FolderName,
    parent_id: Option<FolderId>,
    path: String,       // materialized path: "/parent/child/"
    depth: i32,         // 1〜5
    created_by: Option<UserId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

メソッド:
- `new(id, tenant_id, name, parent_id, parent_path, parent_depth, created_by, now)` — 新規作成。depth 計算・パス生成・depth チェックを含む
- `from_db(...)` — DB からの復元
- `rename(new_name, now)` — 名前変更。自身の path を再計算した新インスタンスを返す
- `move_to(new_parent_id, new_parent_path, new_parent_depth, now)` — 移動。新 path/depth を計算
- `child_path(child_name)` — 子フォルダのパスを計算
- `child_depth()` — 子フォルダの depth を計算（depth + 1、MAX_FOLDER_DEPTH 超過でエラー）
- ゲッター: `id()`, `tenant_id()`, `name()`, `parent_id()`, `path()`, `depth()`, `created_by()`, `created_at()`, `updated_at()`

ルートフォルダ（parent_id = None）の場合:
- path = `"/{name}/"`
- depth = 1

子フォルダの場合:
- path = `"{parent.path}{name}/"`
- depth = parent.depth + 1

#### 確認事項
- 型: `TenantId` → `backend/crates/domain/src/tenant.rs`
- 型: `UserId` → `backend/crates/domain/src/user.rs`
- パターン: `define_uuid_id!` マクロ → `backend/crates/domain/src/macros.rs`
- パターン: エンティティ定義 → `backend/crates/domain/src/tenant.rs`（`Tenant` の構造）

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 正常なフォルダ名で FolderName を作成する | 正常系 | ユニット |
| 2 | 空文字列で FolderName 作成が拒否される | 準正常系 | ユニット |
| 3 | 255 文字超で FolderName 作成が拒否される | 準正常系 | ユニット |
| 4 | 禁止文字を含むフォルダ名が拒否される | 準正常系 | ユニット |
| 5 | ルート直下にフォルダを作成する | 正常系 | ユニット |
| 6 | 子フォルダを作成する（depth 計算） | 正常系 | ユニット |
| 7 | 5 階層を超えるフォルダ作成が拒否される | 準正常系 | ユニット |
| 8 | フォルダ名を変更する | 正常系 | ユニット |
| 9 | フォルダを別の親に移動する | 正常系 | ユニット |

#### テストリスト

ユニットテスト:
- [ ] FolderName: 正常な名前を受け入れる
- [ ] FolderName: 空文字列を拒否する
- [ ] FolderName: 空白のみを拒否する
- [ ] FolderName: 前後の空白をトリミングする
- [ ] FolderName: 255 文字以内を受け入れる
- [ ] FolderName: 255 文字超を拒否する
- [ ] FolderName: 各禁止文字を拒否する（`/`, `\`, `:`, `*`, `?`, `"`, `<`, `>`, `|`）
- [ ] Folder: ルート直下にフォルダを作成（path = "/{name}/", depth = 1）
- [ ] Folder: 子フォルダを作成（path = parent.path + name + "/", depth = parent.depth + 1）
- [ ] Folder: 5 階層を超える作成を拒否する
- [ ] Folder: rename で名前と path が更新される
- [ ] Folder: move_to で parent_id, path, depth が更新される
- [ ] Folder: from_db でフォルダを復元できる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 2: データベースマイグレーション

### 概要

`folders` テーブルの作成、RLS 有効化、ポリシー作成。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/migrations/20260225000001_create_folders.sql` | 新規 |

### 設計

設計書 `docs/03_詳細設計書/17_ドキュメント管理設計.md` の「folders テーブル」セクションに準拠。

```sql
CREATE TABLE folders (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   UUID NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    name        VARCHAR(255) NOT NULL,
    parent_id   UUID REFERENCES folders(id) ON DELETE RESTRICT,
    path        TEXT NOT NULL,
    depth       INTEGER NOT NULL CHECK (depth >= 1 AND depth <= 5),
    created_by  UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE NULLS NOT DISTINCT (tenant_id, parent_id, name)
);

-- RLS
ALTER TABLE folders ENABLE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation ON folders
    FOR ALL
    TO ringiflow_app
    USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID)
    WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::UUID);

-- インデックス
CREATE INDEX idx_folders_tenant_id ON folders (tenant_id);
CREATE INDEX idx_folders_parent_id ON folders (parent_id);
CREATE INDEX idx_folders_path ON folders (path);
```

#### 確認事項
- パターン: RLS ポリシー構文 → `backend/migrations/20260210000003_enable_rls_policies.sql`
- パターン: 既存マイグレーションの命名 → `backend/migrations/` の最新ファイル

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 3: リポジトリ

### 概要

`FolderRepository` trait を infra クレートに定義し、PostgreSQL 実装を提供する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/crates/infra/src/repository/folder_repository.rs` | 新規 |
| `backend/crates/infra/src/repository.rs` | `pub mod folder_repository;` + re-export 追加 |

### 設計

```rust
#[async_trait]
pub trait FolderRepository: Send + Sync {
    /// テナント内の全フォルダを path 順で取得する
    async fn find_all_by_tenant(&self, tenant_id: &TenantId) -> Result<Vec<Folder>, InfraError>;

    /// ID でフォルダを検索する
    async fn find_by_id(&self, id: &FolderId, tenant_id: &TenantId) -> Result<Option<Folder>, InfraError>;

    /// フォルダを挿入する
    async fn insert(&self, folder: &Folder) -> Result<(), InfraError>;

    /// フォルダを更新する（名前変更）
    async fn update(&self, folder: &Folder) -> Result<(), InfraError>;

    /// フォルダとサブツリーのパスを一括更新する（移動・名前変更時）
    async fn update_subtree_paths(
        &self,
        old_path: &str,
        new_path: &str,
        depth_delta: i32,
        tenant_id: &TenantId,
    ) -> Result<(), InfraError>;

    /// フォルダを削除する
    async fn delete(&self, id: &FolderId) -> Result<(), InfraError>;

    /// 指定フォルダの直接の子フォルダ数をカウントする
    async fn count_children(&self, parent_id: &FolderId) -> Result<i64, InfraError>;
}
```

PostgreSQL 実装: `RoleRepository` パターンに準拠。`sqlx::query!` マクロ使用。

`update_subtree_paths` の SQL:
```sql
UPDATE folders
SET path = $2 || SUBSTRING(path FROM LENGTH($1) + 1),
    depth = depth + $3,
    updated_at = NOW()
WHERE tenant_id = $4
  AND path LIKE $1 || '%'
```

#### 確認事項
- パターン: リポジトリ trait 定義 → `backend/crates/infra/src/repository/role_repository.rs`
- パターン: PostgreSQL 実装 → 同上
- パターン: re-export → `backend/crates/infra/src/repository.rs`

#### テストリスト

ユニットテスト:
- [ ] PostgresFolderRepository は Send + Sync を実装している

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 4: ユースケース

### 概要

`FolderUseCaseImpl` に CRUD のビジネスロジックを実装する。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/core-service/src/usecase/folder.rs` | 新規 |
| `backend/apps/core-service/src/usecase/mod.rs` または `usecase.rs` | `pub mod folder;` 追加 |

### 設計

```rust
pub struct CreateFolderInput {
    pub tenant_id: TenantId,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_by: Uuid,
}

pub struct UpdateFolderInput {
    pub folder_id: FolderId,
    pub tenant_id: TenantId,
    pub name: Option<String>,
    pub parent_id: Option<Option<Uuid>>,  // None = 変更なし, Some(None) = ルートに移動, Some(Some(id)) = 移動
}
```

ユースケースメソッド:

**create_folder**:
1. `FolderName::new(input.name)` でバリデーション
2. parent_id あり → `find_by_id` で親取得、`child_depth()` で depth チェック
3. parent_id なし → depth = 1, path = "/{name}/"
4. `Folder::new(...)` → `insert()`
5. UNIQUE 制約違反 → `CoreError::Conflict("同名のフォルダが既に存在します")`

**update_folder** (名前変更 / 移動):
1. `find_by_id` で既存フォルダ取得
2. 名前変更: `folder.rename(new_name, now)` → `update()` + `update_subtree_paths()`
3. 移動: 新親の depth チェック → サブツリー depth チェック → `folder.move_to(...)` → `update()` + `update_subtree_paths()`
4. UNIQUE 制約違反 → `CoreError::Conflict`

**delete_folder**:
1. `find_by_id` でフォルダ取得
2. `count_children()` で子フォルダ数チェック（0 でなければエラー）
3. `delete()`

**list_folders**:
1. `find_all_by_tenant()` で全フォルダ取得（path 順）

#### 確認事項
- パターン: ユースケース構造 → `backend/apps/core-service/src/usecase/role.rs`
- パターン: UNIQUE 制約違反の処理 → `role.rs` の `create_role` メソッド

#### テストリスト

ユニットテスト（該当なし — ロジックはドメインモデルでテスト済み。ユースケースはハンドラテストで検証）
ハンドラテスト（該当なし — Phase 5 でまとめて実施）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 5: Core Service ハンドラ + ハンドラテスト

### 概要

Core Service の `/internal/folders` エンドポイントを実装し、ハンドラテストを書く。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/core-service/src/handler/folder.rs` | 新規 |
| `backend/apps/core-service/src/handler/mod.rs` または類似のモジュール宣言 | `pub mod folder;` 追加 |
| `backend/apps/core-service/src/main.rs` | ルート登録 + State 構築 |

### 設計

エンドポイント:

| メソッド | パス | ハンドラ |
|---------|------|---------|
| GET | `/internal/folders` | `list_folders` |
| POST | `/internal/folders` | `create_folder` |
| PUT | `/internal/folders/{folder_id}` | `update_folder` |
| DELETE | `/internal/folders/{folder_id}` | `delete_folder` |

State:
```rust
pub struct FolderState {
    pub usecase: FolderUseCaseImpl,
}
```

リクエスト/レスポンス型:

```rust
#[derive(Deserialize)]
pub struct FolderTenantQuery {
    pub tenant_id: Uuid,
}

#[derive(Deserialize)]
pub struct CreateFolderRequest {
    pub tenant_id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub created_by: Uuid,
}

#[derive(Deserialize)]
pub struct UpdateFolderRequest {
    pub tenant_id: Uuid,
    pub name: Option<String>,
    pub parent_id: Option<Option<Uuid>>,
}

#[derive(Serialize)]
pub struct FolderDto {
    pub id: Uuid,
    pub name: String,
    pub parent_id: Option<Uuid>,
    pub path: String,
    pub depth: i32,
    pub created_at: String,
    pub updated_at: String,
}
```

main.rs ルート登録:
```rust
.route("/internal/folders", get(list_folders).post(create_folder))
.route("/internal/folders/{folder_id}", put(update_folder).delete(delete_folder))
.with_state(folder_state)
```

#### 確認事項
- パターン: ハンドラ定義 → `backend/apps/core-service/src/handler/role.rs`
- パターン: main.rs ルート登録 → `backend/apps/core-service/src/main.rs:262-350`
- パターン: ハンドラテスト → `backend/apps/core-service/src/handler/role.rs` の `#[cfg(test)] mod tests`

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ルート直下にフォルダを作成する | 正常系 | ハンドラ |
| 2 | 親フォルダの下にサブフォルダを作成する | 正常系 | ハンドラ |
| 3 | フォルダ名が空で作成が 400 になる | 準正常系 | ハンドラ |
| 4 | 5 階層を超えて作成が 400 になる | 準正常系 | ハンドラ |
| 5 | 同名フォルダ作成が 409 になる | 準正常系 | ハンドラ |
| 6 | フォルダ名を変更する | 正常系 | ハンドラ |
| 7 | 空フォルダを削除する | 正常系 | ハンドラ |
| 8 | 子フォルダがあるフォルダの削除が 400 になる | 準正常系 | ハンドラ |
| 9 | フォルダ一覧を取得する | 正常系 | ハンドラ |
| 10 | 存在しないフォルダの操作が 404 になる | 準正常系 | ハンドラ |

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト:
- [ ] POST: ルート直下にフォルダを作成すると 201 が返る
- [ ] POST: 親フォルダの下にサブフォルダを作成すると 201 が返る
- [ ] POST: フォルダ名が空のとき 400 が返る
- [ ] POST: 5 階層を超えると 400 が返る
- [ ] PUT: フォルダ名を変更すると 200 が返る
- [ ] DELETE: 空フォルダを削除すると 204 が返る
- [ ] DELETE: 子フォルダがあると 400 が返る
- [ ] GET: フォルダ一覧が path 順で返る
- [ ] 存在しないフォルダ ID で 404 が返る

API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 6: BFF ハンドラ + クライアント

### 概要

BFF の `/api/v1/folders` エンドポイントを実装する。BFF は認証後に Core Service にプロキシする。

### 変更ファイル

| ファイル | 操作 |
|---------|------|
| `backend/apps/bff/src/client/core_service/folder.rs` | 新規 |
| `backend/apps/bff/src/client/core_service.rs` または `mod.rs` | `pub mod folder;` + re-export 追加 |
| `backend/apps/bff/src/handler/folder.rs` | 新規 |
| `backend/apps/bff/src/handler/mod.rs` または類似 | `pub mod folder;` 追加 |
| `backend/apps/bff/src/main.rs` | ルート登録 + State 構築 |

### 設計

BFF エンドポイント（設計書準拠）:

| メソッド | パス | 説明 |
|---------|------|------|
| POST | `/api/v1/folders` | フォルダ作成 |
| PUT | `/api/v1/folders/{id}` | フォルダ更新 |
| DELETE | `/api/v1/folders/{id}` | フォルダ削除 |
| GET | `/api/v1/folders` | フォルダ一覧 |

BFF ハンドラは `authenticate()` でセッションから `tenant_id` と `user_id` を取得し、Core Service の `/internal/folders` に転送する。`utoipa::path` アノテーションを付与。

#### 確認事項
- パターン: BFF クライアント trait → `backend/apps/bff/src/client/core_service/` を確認
- パターン: BFF ハンドラ → `backend/apps/bff/src/handler/role.rs`
- パターン: utoipa アノテーション → 同上
- パターン: BFF main.rs ルート登録 → `backend/apps/bff/src/main.rs`

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし — BFF は thin proxy、Core Service のハンドラテストでカバー）
API テスト（該当なし）
E2E テスト（該当なし）

---

## 検証

```bash
# 全テスト実行
just check-all

# マイグレーション確認（開発 DB）
just dev-deps
just db-migrate

# 個別テスト実行
cargo test -p ringiflow-domain -- folder
cargo test -p ringiflow-core-service -- folder
```

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | UpdateFolderInput の parent_id の型が曖昧（移動なし / ルート移動 / 別フォルダ移動の3状態を区別） | 曖昧 | `Option<Option<Uuid>>` で3状態を区別する設計に明確化 |
| 2回目 | 空フォルダチェックで documents テーブルが未作成（#881 スコープ） | 不完全なパス | 子フォルダのみチェック。documents チェックは #881 で追加する旨を対象外に明記 |
| 3回目 | update_subtree_paths でサブツリーの depth チェックが漏れている | 不完全なパス | 移動時に最大 depth のサブフォルダが制限超過しないか事前チェックするロジックを Phase 4 に追加 |
| 4回目 | 自己参照移動（フォルダを自身の子孫に移動）の防止が未定義 | エッジケース | ユースケース移動ロジックに循環検出を追加（移動先の path が自身の path で始まるか検査） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Issue の完了基準 5 項目すべてが Phase 1-6 でカバーされている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | UpdateFolderInput の parent_id 型を明確化済み |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | materialized path 更新方式、空フォルダチェック範囲、循環検出を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外セクションにフロントエンド、documents 連携、監査ログを明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | RLS ポリシー構文、UNIQUE NULLS NOT DISTINCT、sqlx マクロ制約を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書のテーブル定義・API 仕様に準拠 |
