# RESTful API 設計

## 概要

HTTP API の設計における共通仕様とベストプラクティス。
リクエスト/レスポンスの標準フォーマット、ステータスコード、ペジネーションを解説する。

## リクエスト仕様

### ヘッダー

| ヘッダー | 必須 | 説明 |
|---------|------|------|
| `Content-Type` | ○（POST/PUT/PATCH） | `application/json` |
| `X-CSRF-Token` | ○（状態変更） | CSRF トークン |
| `X-Request-ID` | - | リクエスト追跡用（自動生成も可） |

### URL 設計

```
# リソースの一覧
GET /api/v1/workflows

# リソースの詳細
GET /api/v1/workflows/{id}

# リソースの作成
POST /api/v1/workflows

# リソースの更新
PUT /api/v1/workflows/{id}

# リソースの削除
DELETE /api/v1/workflows/{id}

# アクション（状態遷移）
POST /api/v1/workflows/{id}/submit
POST /api/v1/workflows/{id}/approve
```

## レスポンス仕様

### 成功レスポンス

```json
// 単一リソース
{
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "title": "経費申請",
    "status": "pending"
  }
}

// リスト（ペジネーション付き）
{
  "data": [
    { "id": "...", "title": "..." },
    { "id": "...", "title": "..." }
  ],
  "pagination": {
    "page": 1,
    "per_page": 20,
    "total_pages": 5,
    "total_count": 100
  }
}
```

### レスポンスヘッダー

| ヘッダー | 説明 |
|---------|------|
| `X-Request-ID` | リクエスト追跡用 ID |
| `X-RateLimit-Limit` | レート制限上限 |
| `X-RateLimit-Remaining` | 残りリクエスト数 |
| `X-RateLimit-Reset` | リセット時刻（Unix タイムスタンプ） |

## HTTP ステータスコード

### 成功

| コード | 説明 | 使用場面 |
|--------|------|---------|
| 200 | OK | 取得・更新成功 |
| 201 | Created | リソース作成成功 |
| 204 | No Content | 削除成功（レスポンスボディなし） |

### クライアントエラー

| コード | 説明 | 使用場面 |
|--------|------|---------|
| 400 | Bad Request | バリデーションエラー |
| 401 | Unauthorized | 未認証（ログインが必要） |
| 403 | Forbidden | 権限不足（認証済みだがアクセス不可） |
| 404 | Not Found | リソースが存在しない |
| 409 | Conflict | 競合（楽観的ロック失敗等） |
| 422 | Unprocessable Entity | ビジネスルール違反 |
| 429 | Too Many Requests | レート制限超過 |

### サーバーエラー

| コード | 説明 | 使用場面 |
|--------|------|---------|
| 500 | Internal Server Error | 予期しないエラー |
| 502 | Bad Gateway | 上流サーバーからの不正レスポンス |
| 503 | Service Unavailable | サービス一時停止 |

### 401 vs 403 の違い

```
401 Unauthorized: 「あなたは誰？」（認証が必要）
403 Forbidden:    「あなたには権限がない」（認証済みだがアクセス不可）
```

## エラーレスポンス（RFC 7807）

```json
{
  "type": "https://api.example.com/errors/validation-error",
  "title": "Validation Error",
  "status": 400,
  "detail": "リクエストの検証に失敗しました",
  "instance": "/api/v1/workflows",
  "correlation_id": "550e8400-e29b-41d4-a716-446655440000",
  "errors": [
    {
      "field": "title",
      "code": "required",
      "message": "タイトルは必須です"
    },
    {
      "field": "email",
      "code": "invalid_format",
      "message": "メールアドレスの形式が不正です"
    }
  ]
}
```

詳細は [RFC 7807 セクション](./Rustエラーハンドリング.md#rfc-7807-problem-details) を参照。

## ペジネーション

### クエリパラメータ

| パラメータ | デフォルト | 説明 |
|-----------|------------|------|
| `page` | 1 | ページ番号（1 始まり） |
| `per_page` | 20 | 1 ページあたりの件数 |

### レスポンス

```json
{
  "data": [...],
  "pagination": {
    "page": 2,
    "per_page": 20,
    "total_pages": 5,
    "total_count": 100
  }
}
```

### 計算式

```
total_pages = ceil(total_count / per_page)
offset = (page - 1) * per_page
```

## フィルタリングとソート

### フィルタリング

```
GET /api/v1/workflows?status=pending&initiated_by_me=true
```

### ソート

```
GET /api/v1/workflows?sort=created_at&order=desc
```

| パラメータ | 値 | 説明 |
|-----------|-----|------|
| `sort` | フィールド名 | ソートキー |
| `order` | `asc` / `desc` | ソート順（デフォルト: `asc`） |

## バージョニング

URL にバージョンを含める方式を採用。

```
/api/v1/workflows
/api/v2/workflows  # 将来のバージョン
```

### なぜ URL バージョニングか

| 方式 | 例 | メリット | デメリット |
|------|-----|---------|-----------|
| URL | `/api/v1/` | 明示的、キャッシュしやすい | URL が変わる |
| ヘッダー | `Accept: application/vnd.api+json;version=1` | URL が変わらない | 分かりにくい |
| クエリ | `?version=1` | シンプル | キャッシュしにくい |

URL 方式はシンプルで分かりやすく、CDN でのキャッシュも容易。

## 日時フォーマット

ISO 8601 形式を使用する。

```json
{
  "created_at": "2025-01-12T10:00:00Z",
  "updated_at": "2025-01-12T10:30:00+09:00"
}
```

- タイムゾーン情報を必ず含める
- サーバーは UTC で保存、レスポンスも UTC で返すのが推奨

## プロジェクトでの使用

API 設計の詳細は [03_API設計_MVP.md](../02_設計書/03_API設計_MVP.md) を参照。

## 関連リソース

- [RFC 7807 - Problem Details for HTTP APIs](https://datatracker.ietf.org/doc/html/rfc7807)
- [Microsoft REST API Guidelines](https://github.com/microsoft/api-guidelines)
- [JSON API Specification](https://jsonapi.org/)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-14 | 初版作成 |
