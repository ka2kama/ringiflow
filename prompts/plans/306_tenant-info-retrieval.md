# Issue #306: テナント情報の取得機能を実装する

## Context

BFF とフロントエンドでテナント情報がハードコードされている。ユーザー認証（#34）実装時に TODO として残された技術的負債。

- BFF `handler/auth.rs`: `tenant_name: "Development Tenant".to_string()`
- フロントエンド `Shared.elm`: `extractTenantId = "00000000-0000-0000-0000-000000000001"`

Core API（Core Service）からテナント情報を取得し、BFF → フロントエンドまで正しいデータを流す。

## Issue 精査結果

| 観点 | 結果 |
|------|------|
| Want | テナント情報をハードコードから実データに置き換え、マルチテナントの正しいデータフローを構築する |
| How への偏り | Issue の「Core API にテナント情報取得エンドポイントを追加」は特定の実装を前提にしている。設計書は既存の `/internal/users/{user_id}` レスポンスにテナント情報を含める構造を規定しているため、既存エンドポイントの拡張で Want を満たす |
| 完了基準の妥当性 | 概ね妥当。ただし「Core API にテナント情報取得エンドポイントがある」は「Core API がテナント情報を返す」に読み替える |
| スコープ | 適切。要件定義書の `/api/v1/tenant` 公開 API は別スコープ |

## 設計判断

### テナント情報の取得方式

認証機能設計書（`docs/03_詳細設計書/07_認証機能設計.md`）が `/internal/users/{user_id}` レスポンスにテナント情報を含める構造を規定しており、これに従う。

**既存 `/internal/users/{user_id}` レスポンスを拡張**:
- 設計書と整合する
- BFF から 1 API コールで完結する
- 将来 `/api/v1/tenant` が必要になったときに専用エンドポイントを追加すればよい

### テナント名の取得方法

Core Service の `get_user` ハンドラに TenantRepository を追加し、ユーザーの `tenant_id` からテナント名を取得する。UserRepository の SQL を変更せず、責務を分離する。

### フロントエンドのスコープ

- User 型に `tenantId` のみ追加（`tenantName` は YAGNI）
- `Shared.init` の初期 tenantId は維持（ログイン前の API コールに必要）
- `withUser` で User.tenantId を使用するよう変更

## 対象・対象外

| 対象 | 対象外 |
|------|--------|
| Core Service: TenantRepository 作成 | BFF 公開 API `/api/v1/tenant` |
| Core Service: `get_user` レスポンスに tenant_name 追加 | テナント設定管理機能 |
| BFF: ハードコード除去 | テナント作成・更新 API |
| フロントエンド: User 型に tenantId 追加 | tenantName の UI 表示 |
| ReviewConfig.elm の古い抑制削除 | |

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: ドメイン層 — Tenant エンティティ

`backend/crates/domain/src/tenant.rs` に `TenantName` と `Tenant` を追加。

```rust
// TenantName: UserName と同じ Newtype パターン
pub struct TenantName(String);
impl TenantName {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> { ... }
    pub fn as_str(&self) -> &str { &self.0 }
}

// Tenant エンティティ（最小限）
pub struct Tenant {
    id: TenantId,
    name: TenantName,
}
impl Tenant {
    pub fn from_db(id: TenantId, name: TenantName) -> Self { ... }
    pub fn id(&self) -> &TenantId { &self.id }
    pub fn name(&self) -> &TenantName { &self.name }
}
```

テストリスト:
- [x] TenantName: 正常な名前を受け入れる
- [x] TenantName: 空文字列を拒否する
- [x] Tenant: from_db でエンティティを復元できる

変更ファイル:
- `backend/crates/domain/src/tenant.rs`

### Phase 2: インフラ層 — TenantRepository

テストリスト:
- [x] ID でテナントを取得できる
- [x] 存在しない ID の場合 None を返す

変更ファイル:
- `backend/crates/infra/src/repository/tenant_repository.rs` （新規）
- `backend/crates/infra/src/repository.rs` （pub mod 追加）
- `backend/crates/infra/tests/tenant_repository_test.rs` （新規）

パターン参照: `user_repository.rs` のトレイト定義 + PostgreSQL 実装

```rust
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>, InfraError>;
}
```

### Phase 3: Core Service — get_user レスポンス拡張

`UserWithPermissionsData` に `tenant_name` フィールドを追加し、`get_user` ハンドラで TenantRepository から取得する。

テストリスト:
- [x] get_user レスポンスに tenant_name が含まれる
- [x] テナントが見つからない場合に内部エラーを返す

変更ファイル:
- `backend/apps/core-service/src/handler/auth.rs` — UserState に TenantRepository 追加、UserWithPermissionsData に tenant_name 追加、get_user ハンドラ修正
- `backend/apps/core-service/src/handler.rs` — re-export 更新（必要に応じて）
- `backend/apps/core-service/src/main.rs` — TenantRepository の DI 追加

### Phase 4: BFF — ハードコード除去

テストリスト:
- [x] MeResponseData が UserWithPermissionsData の tenant_name を使用する
- [x] /auth/me レスポンスに正しい tenant_name が含まれる

変更ファイル:
- `backend/apps/bff/src/client/core_service.rs` — UserWithPermissionsData に tenant_name 追加
- `backend/apps/bff/src/handler/auth.rs` — MeResponseData::from() のハードコード除去、テスト更新

### Phase 5: フロントエンド — User 型拡張

テストリスト:
- [x] User 型に tenantId フィールドがある
- [x] userDecoder が tenant_id をデコードする
- [x] withUser が User.tenantId を Shared.tenantId に設定する

変更ファイル:
- `frontend/src/Shared.elm` — User 型に `tenantId` 追加、`withUser` 修正、`extractTenantId` 削除
- `frontend/src/Api/Auth.elm` — userDecoder を map4 → map5 に更新
- `frontend/review/src/ReviewConfig.elm` — `Session.elm` への古い抑制を削除

注: `Session.elm` は `Shared.elm` にリネーム済み。ReviewConfig の抑制は効いていない（存在しないファイルへの参照）。

## 検証方法

1. `just check-all` — lint + test + API テスト通過
2. `just dev-all` で開発サーバー起動 → ログイン → ブラウザの DevTools で `/api/v1/auth/me` レスポンスを確認:
   - `tenant_name` が `"Development Tenant"`（DB のシードデータと一致）
   - `tenant_id` が `"00000000-0000-0000-0000-000000000001"`
3. フロントエンドの Shared.tenantId がログイン後に User から取得した値になっていることを確認（API リクエストの `X-Tenant-ID` ヘッダー）

## 主要ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/src/tenant.rs` | TenantName, Tenant 追加 |
| `backend/crates/infra/src/repository/tenant_repository.rs` | 新規: TenantRepository |
| `backend/crates/infra/src/repository.rs` | pub mod tenant_repository 追加 |
| `backend/crates/infra/tests/tenant_repository_test.rs` | 新規: 統合テスト |
| `backend/apps/core-service/src/handler/auth.rs` | UserState 拡張、レスポンス拡張 |
| `backend/apps/core-service/src/main.rs` | TenantRepository DI |
| `backend/apps/bff/src/client/core_service.rs` | tenant_name フィールド追加 |
| `backend/apps/bff/src/handler/auth.rs` | ハードコード除去 |
| `frontend/src/Shared.elm` | User 型拡張、withUser 修正 |
| `frontend/src/Api/Auth.elm` | decoder 更新 |
| `frontend/review/src/ReviewConfig.elm` | 古い抑制削除 |

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → Issue の前提検証 | 設計書が規定するレスポンス構造の確認、既存エンドポイントの調査 | 新規エンドポイントではなく既存レスポンスの拡張がベストと判断 |
| 2回目 | フロントエンドのスコープ検証 | User 型、decoder、Shared の実装確認。init の tenantId 初期値が必要か検証 | ログイン前 API コールに必要なため初期値は維持。tenantName は YAGNI で見送り |
| 3回目 | ReviewConfig.elm の抑制確認 | Session.elm の存在確認 | Session.elm は存在しない（Shared.elm にリネーム済み）。抑制は効いていない → 削除 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | ドメイン→インフラ→Core Service→BFF→フロントエンドの全レイヤーを網羅。各 Phase で変更ファイルを明示 |
| 2 | 曖昧さ排除 | OK | 「テナント情報取得エンドポイント」を「既存レスポンスの拡張」に具体化。各 Phase のテストリストに正常系・異常系を明記 |
| 3 | 設計判断の完結性 | OK | 取得方式（既存拡張 vs 新規エンドポイント）、取得方法（別クエリ vs JOIN）、フロントエンドスコープ（tenantId のみ vs tenantName も）の各判断に理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外セクションで明示。`/api/v1/tenant` 公開 API は対象外 |
| 5 | 技術的前提 | OK | Elm の Decode.map5 が利用可能なこと確認。init の初期 tenantId がログイン前に必要なこと確認 |
| 6 | 既存ドキュメント整合 | OK | 認証機能設計書のレスポンス構造と整合。OpenAPI は tenant_id/tenant_name を既に定義済みで更新不要 |
