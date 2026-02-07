# DevAuth（開発用認証バイパス）

## 概要

開発環境でフロントエンド開発を先行させるため、ログイン画面なしで認証済み状態を実現する仕組み。

## 背景

- 認証 API（/auth/login, /auth/logout, /auth/me, /auth/csrf）は実装済み
- フロントエンド（Elm）のログイン画面は未実装
- ログイン画面を作る前に、コア機能（ダッシュボード、ワークフロー管理等）を開発したい

## 仕組み

```mermaid
sequenceDiagram
    participant BFF
    participant Redis
    participant Frontend as Frontend (Elm)

    Note over BFF: DEV_AUTH_ENABLED=true で起動
    BFF->>Redis: 開発用セッションを作成<br/>session:DEV_TENANT_ID:dev-session
    BFF->>Redis: CSRF トークンを作成<br/>csrf:DEV_TENANT_ID:dev-session

    Note over Frontend: Cookie を設定して<br/>認証済み状態でアクセス
    Frontend->>BFF: GET /auth/me<br/>Cookie: session_id=dev-session<br/>X-Tenant-ID: DEV_TENANT_ID
    BFF->>Redis: セッション取得
    Redis-->>BFF: SessionData
    BFF-->>Frontend: 200 OK (ユーザー情報)
```

## 使い方

### 1. 環境変数を設定

`.env` ファイルに追加:

```bash
DEV_AUTH_ENABLED=true
```

### 2. BFF を起動

```bash
cargo run -p ringiflow-bff
```

起動時に以下のログが出力される:

```
WARN  ========================================
WARN  ⚠️  DevAuth が有効です！
WARN     本番環境では絶対に有効にしないでください
WARN  ========================================
INFO  DevAuth: 開発用セッションを作成しました
INFO    Tenant ID: 00000000-0000-0000-0000-000000000001
INFO    User ID: 00000000-0000-0000-0000-000000000001
INFO    Session ID: dev-session
INFO    CSRF Token: <64文字のトークン>
```

### 3. フロントエンドで Cookie を設定

ブラウザの開発者ツールで Cookie を設定:

```javascript
document.cookie = "session_id=dev-session; path=/";
```

または、フロントエンドの開発用コードで設定:

```elm
-- TODO: フロントエンドの DevAuth 対応を実装
```

### 4. API リクエスト時にヘッダーを設定

```
X-Tenant-ID: 00000000-0000-0000-0000-000000000001
```

## 開発用ユーザー情報

| 項目 | 値 |
|------|-----|
| Tenant ID | `00000000-0000-0000-0000-000000000001` |
| User ID | `00000000-0000-0000-0000-000000000001` |
| Email | `admin@example.com` |
| Name | `管理者` |
| Roles | `["tenant_admin"]` |
| Session ID | `dev-session` |

## 安全策

1. **Cargo feature flag によるコンパイル時除外**: `dev-auth` feature が無効の場合、DevAuth のコードは本番バイナリに一切含まれない（[ADR-034](../../05_ADR/034_DevAuthのFeatureFlag導入.md)）
2. **環境変数による実行時制御**: feature 有効時でも `DEV_AUTH_ENABLED=true` が設定されていない場合は無効
3. **警告ログ**: 起動時に目立つ警告を出力
4. **固定の値**: セッション ID やユーザー情報が固定のため、本番データと混同しにくい
5. **CI での検証**: 本番ビルド（`--no-default-features`）がコンパイルできることを CI で継続的に検証

### ビルドモードと DevAuth の関係

| ビルドモード | コマンド | DevAuth |
|-------------|---------|---------|
| 開発（デフォルト） | `cargo run` / `cargo build` | 含まれる（default feature） |
| 本番 | `cargo build --release --no-default-features` | 除外される |
| テスト | `cargo test --all-features` | 含まれる（全 feature テスト） |

## 本番環境での注意

Dockerfile では `--no-default-features` で `dev-auth` feature を除外しており、本番バイナリに DevAuth コードは含まれない。

開発環境では `DEV_AUTH_ENABLED` を本番環境で有効にしないこと。環境変数を設定しないか、明示的に `DEV_AUTH_ENABLED=false` を設定する。

## 関連

- 実装: `backend/apps/bff/src/dev_auth.rs`
- Feature flag 設計: [ADR-034: DevAuth の Feature Flag 導入](../../05_ADR/034_DevAuthのFeatureFlag導入.md)
- Issue: [#79 開発用認証バイパス（DevAuth）を実装](https://github.com/ka2kama/ringiflow/issues/79)
- Issue: [#288 本番ビルドで DevAuth が有効化されることを防止する](https://github.com/ka2kama/ringiflow/issues/288)
- 認証機能設計: [07_認証機能設計.md](../03_詳細設計書/07_認証機能設計.md)
