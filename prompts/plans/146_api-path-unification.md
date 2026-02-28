# Issue #146: API パス設計の統一 (`/auth/*` → `/api/v1/auth/*`)

## Summary

すべての認証エンドポイントを `/auth/*` から `/api/v1/auth/*` に移行し、API パス設計を統一する。
Single PR で実施（機械的な文字列置換、振る舞い変更なし）。

## Design Decisions

| 判断 | 結論 | 理由 |
|------|------|------|
| Single PR or phased? | Single PR | 機械的な置換、原子的に変更すべき |
| 設計書の更新? | Yes | Living docs（要件定義書、詳細設計書）は更新する |
| ADR 作成? | No | Issue #146 が十分な根拠を記載済み |
| 新規テスト? | No | 既存テストのパス更新で検証できる |

## Implementation Steps

### Step 1: Branch & Draft PR

```
git checkout -b feature/146-unify-auth-api-paths
git commit --allow-empty -m "#146 WIP: Unify auth API paths under /api/v1"
git push -u origin HEAD
gh pr create --draft ...
```

### Step 2: Backend Production Code

**`backend/apps/bff/src/main.rs`** (lines 177-191)
- `/auth/login` → `/api/v1/auth/login`
- `/auth/logout` → `/api/v1/auth/logout`
- `/auth/me` → `/api/v1/auth/me`
- `/auth/csrf` → `/api/v1/auth/csrf`

**`backend/apps/bff/src/middleware/csrf.rs`** (line 29)
- `CSRF_SKIP_PATHS`: `"/auth/login"` → `"/api/v1/auth/login"`, `"/auth/csrf"` → `"/api/v1/auth/csrf"`
- ⚠️ **Critical**: ルート定義と完全一致が必要。不一致だとログインが壊れる

**`backend/apps/bff/src/handler/auth.rs`**
- Doc comments (lines 7-9, 154, 296, 342, 397): パス参照を更新
- Test router setup (lines 876-888): 4つのルートパスを更新
- Test URIs (lines 914, 950, 988, 1017, 1041, 1074, 1108, 1138, 1166, 1199): 全テスト URI を更新

### Step 3: Backend Integration Tests

**`backend/apps/bff/tests/auth_integration_test.rs`**
- Doc comments (lines 15-21): パス参照を更新
- Router setup (lines 332-344): 4つのルートパスを更新
- Helper functions (lines 365, 376, 387, 398, 625, 636): URI を更新
- Test comments (lines 445, 533, 612, 713): パス参照を更新

### Step 4: Frontend

**`frontend/src/Api/Auth.elm`**
- Line 5: module doc `/auth` → `/api/v1/auth`
- Line 33: doc comment `GET /auth/csrf` → `GET /api/v1/auth/csrf`
- Line 47: URL `"/auth/csrf"` → `"/api/v1/auth/csrf"`
- Line 55: doc comment `GET /auth/me` → `GET /api/v1/auth/me`
- Line 69: URL `"/auth/me"` → `"/api/v1/auth/me"`

**`frontend/vite.config.js`** (lines 25-28)
- `/auth` プロキシエントリを**削除**（`/api` プロキシが `/api/v1/auth/*` をカバー）

### Step 5: OpenAPI Specification

**`openapi/openapi.yaml`**
- Path keys: `/auth/login` → `/api/v1/auth/login` (4つすべて)
- CSRF description (line 772): `/auth/csrf` → `/api/v1/auth/csrf`

### Step 6: Hurl E2E Tests

**`tests/api/hurl/auth/`** — 全6ファイルのパス参照を更新:
- `login.hurl`, `logout.hurl`, `me.hurl`, `csrf.hurl`
- `csrf_unauthorized.hurl`, `me_unauthorized.hurl`

**`tests/api/hurl/workflow/`** — auth パス参照を更新:
- `create_workflow.hurl` (2箇所)
- `submit_workflow.hurl` (2箇所)

### Step 7: Design Documents

以下の Living docs のパス参照を更新:
- `docs/40_詳細設計書/03_API設計.md` — Mermaid 図、エンドポイント記載
- `docs/40_詳細設計書/07_認証機能設計.md` — シーケンス図、エンドポイント参照
- `docs/40_詳細設計書/08_AuthService設計.md` — BFF パス参照（⚠️ `/internal/auth/*` は変更しない）
- `docs/10_要件定義書/01_コア要件.md` — エンドポイント表

**変更しないもの:**
- `/internal/auth/*`（Auth Service 内部 API）
- `/health`（インフラ用、バージョニング不要）
- 歴史的ドキュメント（ADR、セッションログ等）

## What NOT to Change

- `/internal/auth/*` paths — Auth Service の内部 API、BFF 公開 API とは別
- `/health` — インフラ用エンドポイント、バージョニング対象外
- Historical docs (`docs/70_ADR/`, `docs/90_実装解説/`, `prompts/`) — 時点記録は改変しない

## Risk

| リスク | 影響 | 対策 |
|--------|------|------|
| CSRF_SKIP_PATHS とルート定義の不一致 | ログイン不能 | 両方を同時に更新、テストで検証 |
| Hurl テストの更新漏れ | E2E テスト失敗 | `grep -r "/auth/"` で漏れチェック |
| ドキュメント更新漏れ | 仕様と実装の乖離 | 全 docs grep で確認 |

## Verification

```bash
just check-all          # lint + 全テスト
grep -rn '"/auth/' --include='*.rs' --include='*.elm' --include='*.js' --include='*.yaml' .
                        # 更新漏れがないか確認（docs 以外）
```
