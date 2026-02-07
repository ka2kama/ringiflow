# Issue #203: 承認者選択のユーザー検索 UI を実装する

## 概要

現在の新規申請フォーム（Step 3: 承認者選択）では UUID を直接入力させている。これを名前検索 + 選択 UI に変更し、同姓同名ユーザーを表示用 ID（`USER-N`）で区別できるようにする。

## 完了基準

- [ ] 承認者選択で名前による検索ができる（テキスト入力 → 候補表示）
- [ ] 同姓同名のユーザーを区別できる（表示用 ID やメールアドレスの併記）
- [ ] UUID の直接入力が不要になる
- [ ] ユーザーが検索 UI から承認者を選択して申請を完了できる（E2E）

## 設計判断

### 検索方式: 全ユーザー取得 + フロントエンド側フィルタ（推奨）

| 方式 | メリット | デメリット |
|------|---------|-----------|
| A) 全ユーザー取得 + フロントエンド側フィルタ | API 呼び出し1回、即時フィルタ、実装シンプル | ユーザー数が多い場合にパフォーマンス問題 |
| B) キーワード入力時に API 呼び出し | 大規模テナントでもスケール | debounce 実装が必要、ネットワーク遅延 |

**選定理由**: MVP 段階では1テナント内のユーザー数は限定的（数百人以下）。将来の拡張時にページネーション対応で B に移行可能。

### UI パターン: ドロップダウンリスト + フィルタ入力

```
┌─────────────────────────────────────────────────┐
│ 承認者を検索                                      │
│ [                山田                           ]│
├─────────────────────────────────────────────────┤
│ ○ 山田太郎 (USER-5) - yamada.taro@example.com   │
│ ○ 山田花子 (USER-12) - yamada.hanako@example.com │
└─────────────────────────────────────────────────┘
```

## Phase 構成

### Phase 1: User への display_number 追加

**概要**: 表示用 ID パターンを User エンティティに適用

**変更ファイル**:
- `backend/crates/infra/migrations/YYYYMMDD000001_add_display_number_to_users.sql` (新規)
- `backend/crates/infra/migrations/YYYYMMDD000002_set_users_display_number_not_null.sql` (新規)
- `backend/crates/domain/src/value_objects.rs` - `DisplayIdEntityType::User` 追加
- `backend/crates/domain/src/user.rs` - `display_number` フィールド追加
- `backend/crates/infra/src/repository/user_repository.rs` - SELECT/INSERT に display_number 追加

**テストリスト**:
- [ ] `DisplayIdEntityType::User` の DB 文字列が `"user"` であること
- [ ] `DisplayIdEntityType::User` のプレフィックスが `"USER"` であること
- [ ] `User` エンティティから `display_number` を取得できること
- [ ] 新規ユーザー作成時に display_number が採番されること（統合テスト）
- [ ] 既存ユーザーに display_number が割り当てられていること（マイグレーション後）

### Phase 2: ユーザー一覧 API 実装

**概要**: テナント内ユーザー一覧取得 API を実装

**変更ファイル**:
- `backend/crates/infra/src/repository/user_repository.rs` - `find_all_active_by_tenant` 追加
- `backend/apps/core-service/src/usecase/user.rs` (新規)
- `backend/apps/core-service/src/handler/user.rs` (新規)
- `backend/apps/core-service/src/main.rs` - ルーティング追加
- `backend/apps/bff/src/handler/user.rs` (新規)
- `backend/apps/bff/src/client/core_service.rs` - `list_users` 追加
- `backend/apps/bff/src/main.rs` - ルーティング追加
- `openapi/openapi.yaml` - `/api/v1/users` エンドポイント追加

**テストリスト**:
- [ ] テナント内のアクティブユーザー一覧が取得できること
- [ ] 削除済み/非アクティブユーザーが除外されること
- [ ] 他テナントのユーザーが含まれないこと
- [ ] `GET /api/v1/users` が認証済みで 200 を返すこと（API テスト）
- [ ] 未認証で 401 を返すこと（API テスト）

### Phase 3: フロントエンド検索 UI 実装

**概要**: 承認者選択を検索 UI に変更

**変更ファイル**:
- `frontend/src/Api/User.elm` (新規) - API クライアント
- `frontend/src/Data/User.elm` (新規) - 型定義、デコーダー
- `frontend/src/Page/Workflow/New.elm` - 検索 UI 実装

**Model 変更**:
```elm
-- 現在
, approverInput : String

-- 変更後
, users : RemoteData ApiError (List User)
, userSearchQuery : String
, selectedUser : Maybe User
```

**テストリスト**:
- [ ] `User` 型のデコーダーが正しく動作すること
- [ ] 検索クエリでユーザーリストがフィルタされること
- [ ] 空文字列の検索で全ユーザーが表示されること
- [ ] 名前・メールアドレスの部分一致でフィルタされること

**E2E シナリオ（手動確認）**:
- [ ] 新規申請画面で承認者検索ができること
- [ ] 検索結果から承認者を選択できること
- [ ] 選択した承認者で申請が完了すること
- [ ] 同姓同名ユーザーが表示用 ID で区別できること

## OpenAPI 仕様

```yaml
/api/v1/users:
  get:
    tags:
      - users
    summary: テナント内ユーザー一覧取得
    operationId: listUsers
    security:
      - sessionAuth: []
    responses:
      '200':
        description: ユーザー一覧
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/UserListResponse'
      '401':
        $ref: '#/components/responses/Unauthorized'

UserListResponse:
  type: object
  required:
    - data
  properties:
    data:
      type: array
      items:
        $ref: '#/components/schemas/UserItem'

UserItem:
  type: object
  required:
    - id
    - display_id
    - display_number
    - name
    - email
  properties:
    id:
      type: string
      format: uuid
    display_id:
      type: string
      pattern: '^USER-\d+$'
      example: "USER-5"
    display_number:
      type: integer
      minimum: 1
    name:
      type: string
      example: "山田太郎"
    email:
      type: string
      format: email
```

## 再利用する既存パターン

- **表示用 ID パターン** (MEMORY.md 記載)
  - `DisplayNumber`, `DisplayId`, `DisplayIdEntityType` - `backend/crates/domain/src/value_objects.rs`
  - `DisplayIdCounterRepository` - `backend/crates/infra/src/repository/display_id_counter_repository.rs`
  - マイグレーションパターン - `backend/crates/infra/migrations/20260202000002_add_display_number_to_workflow_instances.sql`

- **API 実装パターン**
  - Core Service ハンドラ - `backend/apps/core-service/src/handler/workflow.rs`
  - BFF プロキシ - `backend/apps/bff/src/handler/workflow.rs`

## 検証方法

1. **Phase 1 完了後**: `just check-all` 通過、マイグレーション成功
2. **Phase 2 完了後**: `just check-all` 通過、API テスト（Hurl）通過
3. **Phase 3 完了後**: `just check-all` 通過、手動 E2E テスト
   - ログイン → 新規申請 → 承認者検索 → 選択 → 申請完了

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | User エンティティ、UserRepository、Core Service、BFF、OpenAPI、フロントエンドを網羅 |
| 2 | 曖昧さ排除 | OK | 各 Phase の変更対象ファイルと変更内容を具体的に記載 |
| 3 | 設計判断の完結性 | OK | 検索方式（A vs B）、UI パターンの選定理由を明記 |
| 4 | スコープ境界 | OK | 対象: ユーザー検索 UI、対象外: 複雑な検索条件、ページネーション |
| 5 | 技術的前提 | OK | 表示用 ID パターン（Phase A/B で確立済み）を参照 |
| 6 | 既存ドキュメント整合 | OK | 表示用 ID 設計書、ADR-029 と整合確認済み |
