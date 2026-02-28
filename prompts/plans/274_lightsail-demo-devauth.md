# Plan: #274 Set up authentication for Lightsail demo environment

## Context

Lightsail デモ環境（`https://demo.ka2kama.com`）でダッシュボードは表示されるが、API 呼び出しが認証エラーで失敗する。ローカル開発では DevAuth（開発用認証バイパス）を使用しているが、本番ビルドでは `#[cfg(feature = "dev-auth")]` により除外済み（#288）。

デモ環境は個人の検証・デモ用（ADR-030）であり、本番環境ではない。DevAuth をデモ環境でも有効にする最小限の対応を行う。ログインページの実装は別 Issue で扱う。

## To-Be

- デモ環境で DevAuth が有効になり、ダッシュボードの API 呼び出しが成功する
- 本番ビルド（CI / 将来の本番デプロイ）はデフォルトで DevAuth が除外されたまま
- Dockerfile にビルド引数を追加し、デモ/本番を切り替え可能にする

## 対象・対象外

**対象**: Dockerfile（Backend / Frontend）のビルド引数追加、`main.js` のデモ対応、Lightsail 設定（docker-compose, .env.example, deploy.sh）

**対象外**: ログインページの実装（別 Issue）、データベースシード（既存マイグレーションで対応済み）、CI ワークフロー（既存の `--no-default-features` チェックは変更不要）

## 設計判断

### 1. ビルド引数でのフィーチャー制御

Dockerfile にビルド引数 `CARGO_FEATURES` を追加し、デフォルトは `--no-default-features`（本番）。デモ環境ではデプロイスクリプトから空文字を渡すことで、default features（dev-auth 含む）が有効になる。

```dockerfile
ARG CARGO_FEATURES="--no-default-features"
RUN cargo chef cook --release ${CARGO_FEATURES} --recipe-path recipe.json
RUN cargo build --release ${CARGO_FEATURES} --bin ...
```

代替案: 別の Dockerfile を作る → 重複が増えメンテナンスコストが高いため不採用。

### 2. フロントエンドのデモ対応

Vite のビルド時環境変数 `VITE_DEV_AUTH` を導入。`import.meta.env.DEV`（開発モード）に加え、`VITE_DEV_AUTH === 'true'`（デモビルド）でも dev-session Cookie を設定する。

```javascript
if (import.meta.env.DEV || import.meta.env.VITE_DEV_AUTH === "true") {
  document.cookie = "session_id=dev-session; path=/";
}
```

Vite はビルド時に `import.meta.env.VITE_*` をリテラルに置換するため、ランタイムに環境変数は不要。

### 3. デプロイスクリプトの変更方針

デプロイスクリプト（`deploy.sh`）は Lightsail デモ専用。デモ向けビルド引数をハードコードする。将来 `--env` 引数で切り替えたい場合は拡張可能だが、現時点では不要（YAGNI）。

### 4. Cookie の Secure フラグとの共存

デモ環境の BFF は `ENV=production`（Secure Cookie 有効）。DevAuth の Cookie は JavaScript で `session_id=dev-session; path=/` と設定される（Secure フラグなし）。ブラウザは HTTPS（Cloudflare）経由でアクセスするため、Cookie は正常に送信される。Secure フラグなしの Cookie も HTTPS リクエストには送信される（Secure は HTTP 送信を**禁止**するフラグであり、HTTPS 送信を必須にするフラグではない）。

## 実装計画

### Phase 1: Backend Dockerfile — ビルド引数追加

#### 確認事項: なし（既知のパターンのみ）

**変更ファイル**: `backend/Dockerfile`

| 行 | 変更内容 |
|----|---------|
| 47-52 | `ARG CARGO_FEATURES="--no-default-features"` 追加、`cook` コマンドで使用 |
| 63-64 | `build` コマンドでも同じ引数を使用 |

```dockerfile
# Stage 3: Builder
FROM chef AS builder

# ビルド引数: デフォルトは本番（dev-auth 除外）
# デモ環境では --build-arg CARGO_FEATURES="" で default features を有効化
ARG CARGO_FEATURES="--no-default-features"

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release ${CARGO_FEATURES} --recipe-path recipe.json

# ...

RUN cargo build --release ${CARGO_FEATURES} --bin ringiflow-bff --bin ringiflow-core-service --bin ringiflow-auth-service
```

### Phase 2: Frontend — VITE_DEV_AUTH 対応

#### 確認事項
- パターン: Vite 環境変数の既存使用 → `frontend/src/main.js:116` の `VITE_API_BASE_URL`

**変更ファイル**:

| ファイル | 変更内容 |
|---------|---------|
| `frontend/src/main.js:51` | DevAuth 条件に `VITE_DEV_AUTH` チェックを追加 |
| `frontend/Dockerfile:33-50` | Builder ステージに `ARG VITE_DEV_AUTH` を追加 |

**main.js** (line 51):
```javascript
if (import.meta.env.DEV || import.meta.env.VITE_DEV_AUTH === "true") {
  document.cookie = "session_id=dev-session; path=/";
}
```

**Dockerfile**:
```dockerfile
FROM node:22-slim AS builder

# ...

# デモ環境では --build-arg VITE_DEV_AUTH=true で DevAuth Cookie を有効化
ARG VITE_DEV_AUTH=false
ENV VITE_DEV_AUTH=${VITE_DEV_AUTH}

RUN pnpm run build
```

### Phase 3: Lightsail 設定 — DevAuth 有効化

#### 確認事項: なし（既知のパターンのみ）

**変更ファイル**:

| ファイル | 変更内容 |
|---------|---------|
| `infra/lightsail/docker-compose.yaml:43-54` | BFF environment に `DEV_AUTH_ENABLED: "true"` 追加 |
| `infra/lightsail/.env.example` | `DEV_AUTH_ENABLED` の追加 |

**docker-compose.yaml** (BFF service):
```yaml
environment:
  RUST_LOG: ${RUST_LOG:-info}
  BFF_HOST: 0.0.0.0
  BFF_PORT: 13000
  CORE_URL: http://core-service:13001
  AUTH_URL: http://auth-service:13002
  REDIS_URL: redis://:${REDIS_PASSWORD}@redis:6379
  ENV: production
  # デモ環境では DevAuth を有効化（BFF 起動時に開発用セッションを自動作成）
  DEV_AUTH_ENABLED: "true"
```

**.env.example**:
```env
# DevAuth（開発用認証バイパス）
# デモ環境では true に設定すると、BFF 起動時に開発用セッションが自動作成される
DEV_AUTH_ENABLED=true
```

### Phase 4: デプロイスクリプト — ビルド引数追加

#### 確認事項: なし（既知のパターンのみ）

**変更ファイル**: `infra/lightsail/deploy.sh`

| 行 | 変更内容 |
|----|---------|
| 98 | Backend ビルドに `--build-arg CARGO_FEATURES=""` 追加 |
| 101 | Frontend ビルドに `--build-arg VITE_DEV_AUTH=true` 追加 |

```bash
info "Backend イメージをビルド中..."
docker build \
    --build-arg CARGO_FEATURES="" \
    -t ringiflow-backend:latest \
    -f backend/Dockerfile backend/

info "Frontend イメージをビルド中..."
docker build \
    --build-arg VITE_DEV_AUTH=true \
    -t ringiflow-frontend:latest \
    -f frontend/Dockerfile frontend/
```

### Phase 5: ドキュメント更新

#### 確認事項
- パターン: 既存ナレッジベースの構造 → `docs/80_ナレッジベース/security/DevAuth.md`

**変更ファイル**:

| ファイル | 変更内容 |
|---------|---------|
| `docs/80_ナレッジベース/security/DevAuth.md` | デモ環境での有効化手順を追記 |

## 検証

```bash
# ローカルで確認
just check-all  # lint + test + API テスト（既存テストが壊れないこと）

# Docker ビルド確認（デモ用）
docker build --build-arg CARGO_FEATURES="" -t ringiflow-backend:test -f backend/Dockerfile backend/
docker build --build-arg VITE_DEV_AUTH=true -t ringiflow-frontend:test -f frontend/Dockerfile frontend/

# Docker ビルド確認（本番用 — デフォルト引数）
docker build -t ringiflow-backend:test-prod -f backend/Dockerfile backend/
docker build -t ringiflow-frontend:test-prod -f frontend/Dockerfile frontend/
```

## E2E 動作フロー（デモ環境）

```
1. deploy.sh がビルド引数付きで Docker イメージをビルド
   - Backend: CARGO_FEATURES="" → default features → dev-auth 含む
   - Frontend: VITE_DEV_AUTH=true → Cookie 設定コードが有効

2. BFF 起動時
   - DEV_AUTH_ENABLED=true → DevAuth セッションを Redis に作成
   - セッション ID: "dev-session", テナント ID: 00...0001

3. ユーザーがブラウザでアクセス
   - Nginx が静的ファイル配信
   - main.js が session_id=dev-session Cookie を設定

4. Elm アプリ起動
   - GET /api/v1/auth/csrf → Cookie で認証 → CSRF トークン取得 ✅
   - GET /api/v1/auth/me → Cookie で認証 → ユーザー情報取得 ✅
   - ダッシュボード API 呼び出し → 認証済みセッションで成功 ✅
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Cookie の Secure フラグと DevAuth の共存 | 競合・エッジケース | Secure フラグの動作を確認。HTTPS 経由なら Secure なし Cookie も送信される。設計判断に記載 |
| 2回目 | DB にデモ用ユーザーが必要か | 不完全なパス | seed マイグレーション（`20260115000008`）で dev テナント・ユーザーが作成済み。対象外に記載 |
| 3回目 | `me` ハンドラが Core Service に `get_user(user_id)` を呼ぶ際の RLS 影響 | アーキテクチャ不整合 | 既存の動作であり本 Issue のスコープ外。RLS 対応は別途確認が必要だが、DevAuth のセッションデータに正しい tenant_id が含まれているため、CSRF ミドルウェア経由で tenant_id は正しく伝播する |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 変更対象: Dockerfile（Backend/Frontend）、main.js、docker-compose、.env.example、deploy.sh、ドキュメント。すべて計画に含む |
| 2 | 曖昧さ排除 | OK | 各ファイルの変更行と具体的なコードスニペットを記載。「必要に応じて」等の不確定表現なし |
| 3 | 設計判断の完結性 | OK | ビルド引数方式の選択理由、Cookie Secure フラグの共存、デプロイスクリプトの方針を記載 |
| 4 | スコープ境界 | OK | 対象（DevAuth 有効化）と対象外（ログインページ、DB シード、CI）を明記 |
| 5 | 技術的前提 | OK | Vite の `import.meta.env` ビルド時置換、Docker ARG/ENV の動作、Cookie Secure フラグの仕様を確認 |
| 6 | 既存ドキュメント整合 | OK | ADR-030（Lightsail 構成）、ADR-034（DevAuth Feature Flag）と矛盾なし |
