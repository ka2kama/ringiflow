# DevAuth 開発環境セットアップ

開発環境でログイン画面なしに認証済み状態を実現する手順。

## 前提条件

- `just setup` が完了していること
- `just dev-deps` で PostgreSQL / Redis が起動していること

## 手順

### 1. 環境変数の確認

`.env` が生成されていることを確認:

```bash
# worktree 環境の場合は自動でオフセットが設定される
just setup-env
```

`backend/.env` に以下が含まれていることを確認:

```
DEV_AUTH_ENABLED=true
```

### 2. サーバー起動

```bash
# ターミナル 1: Core Service
just dev-core-service

# ターミナル 2: BFF
just dev-bff

# ターミナル 3: フロントエンド
just dev-web
```

### 3. 動作確認

BFF のログに以下が表示されることを確認:

```
⚠️  DevAuth が有効です！
DevAuth: 開発用セッションを作成しました
  Tenant ID: 00000000-0000-0000-0000-000000000001
  User ID: 00000000-0000-0000-0000-000000000002
  Session ID: dev-session
```

### 4. ブラウザでアクセス

http://localhost:15273/ にアクセス。

フロントエンドが自動的に `session_id=dev-session` Cookie を設定し、認証済み状態になる。

## トラブルシューティング

### 401 Unauthorized が返される

1. **BFF を再起動**: `.env` の変更は再起動で反映

```bash
# BFF を停止して再起動
pkill -f ringiflow-bff
just dev-bff
```

2. **ブラウザをハードリフレッシュ**: Ctrl+Shift+R

3. **Cookie を確認**: 開発者ツール → Application → Cookies で `session_id=dev-session` があるか確認

### X-Tenant-ID ヘッダーが必要ですエラー

フロントエンドの Session モジュールで tenantId が設定されていることを確認。

デフォルト: `00000000-0000-0000-0000-000000000001`

## 関連ドキュメント

- [DevAuth ナレッジベース](../../docs/80_ナレッジベース/security/DevAuth.md)
- [セッションログ: DevAuth設定と詳細画面バグ修正](../runs/2026-01/2026-01-28_DevAuth設定と詳細画面バグ修正.md)
