---
paths:
  - "backend/apps/bff/src/handler/**/*.rs"
  - "backend/apps/bff/src/main.rs"
  - "openapi/**/*.yaml"
---

# API 実装時のルール

このルールは BFF の公開 API エンドポイントを追加・変更する際に適用される。

## 必須チェックリスト

### 1. OpenAPI 仕様書の更新

**ファイル:** [`openapi/openapi.yaml`](../../openapi/openapi.yaml)

| 変更内容 | 更新箇所 |
|---------|---------|
| エンドポイント追加 | `paths` セクションにパスとメソッドを追加 |
| エンドポイント変更 | `paths` セクションの該当箇所を修正 |
| エンドポイント削除 | `paths` セクションから該当箇所を削除 |
| リクエスト/レスポンス型の変更 | `components/schemas` の該当スキーマを修正 |
| 共通パラメータの変更 | `components/parameters` の該当パラメータを修正 |
| エラーレスポンスの変更 | `components/responses` の該当レスポンスを修正 |

### 2. API 設計書との整合性確認

[`docs/03_詳細設計書/03_API設計.md`](../../docs/03_詳細設計書/03_API設計.md) に記載された設計と一致しているか確認する。

## OpenAPI 記述ガイドライン

### エンドポイント定義の基本形

```yaml
/api/v1/resources:
  post:
    tags:
      - resources
    summary: リソース作成（短く）
    description: |
      詳細な説明。
      複数行可。
    operationId: createResource  # キャメルケース
    security:
      - sessionAuth: []
    parameters:
      - $ref: '#/components/parameters/XTenantId'
      - $ref: '#/components/parameters/XCsrfToken'
    requestBody:
      required: true
      content:
        application/json:
          schema:
            $ref: '#/components/schemas/CreateResourceRequest'
    responses:
      '201':
        description: 作成成功
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ResourceResponse'
      '400':
        description: バリデーションエラー
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/ProblemDetails'
      '401':
        $ref: '#/components/responses/Unauthorized'
      '404':
        $ref: '#/components/responses/NotFound'
```

### スキーマ定義の基本形

```yaml
CreateResourceRequest:
  type: object
  required:
    - name
  properties:
    name:
      type: string
      description: リソース名
      minLength: 1
      maxLength: 255
      example: "サンプル"
```

### 共通パターン

| パターン | 使用方法 |
|---------|---------|
| 認証必須エンドポイント | `security: [sessionAuth: []]` |
| 状態変更（POST/PUT/DELETE） | `X-CSRF-Token` パラメータ必須 |
| テナント分離 | `X-Tenant-ID` パラメータ必須 |
| エラーレスポンス | `$ref: '#/components/schemas/ProblemDetails'` |

## AI エージェントへの指示

API エンドポイントを追加・変更・削除した場合:

1. **実装完了後、OpenAPI 仕様書を必ず更新する**
2. コミット前に `just check` で確認する

**禁止事項:**

- 実装と OpenAPI 仕様書が乖離した状態でコミットすること
- エラーレスポンスの RFC 9457 形式からの逸脱

## 参照

- API 設計書: [docs/03_詳細設計書/03_API設計.md](../../docs/03_詳細設計書/03_API設計.md)
- OpenAPI 仕様: [openapi/openapi.yaml](../../openapi/openapi.yaml)
