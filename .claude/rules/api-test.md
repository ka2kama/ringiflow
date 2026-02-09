---
paths:
  - "tests/api/**/*.hurl"
---

# API テスト（Hurl）のルール

このルールは Hurl API テストファイルを追加・変更する際に適用される。

## アサーション方針: 値の決定性で手段を選ぶ

各フィールドの**値の決定性**に基づいてアサーション手段を選択する。

### 判断基準

| 優先度 | 値の性質 | アサーション | 例 |
|--------|---------|-------------|-----|
| 1 | 決定的（seed データ・リクエスト入力・ドメインロジックから確定） | `==` 厳密一致 | `status == "Draft"`, `version == 1` |
| 2 | 非決定的だが形式が既知（タイムスタンプ、自動生成 UUID、トークン） | `matches` パターン検証 | `created_at matches "^\\d{4}-\\d{2}-\\d{2}T"` |
| 3 | 配列型 | `count` 型＋件数検証 | `roles count >= 0` |
| 4 | 上記のいずれにも該当しない | `exists`（最終手段） | — |

`exists` で済ませてよいのは、値の性質が上記 1〜3 に該当しない場合のみ。

### 決定的な値の見極め方

API テストは seed データを投入したうえで実行する。以下の値は決定的であり、`==` で検証できる:

| 値の出所 | 例 |
|---------|-----|
| seed データ（マイグレーション） | `tenant_name == "Development Tenant"` |
| リクエストボディの入力値 | `title == "経費申請"` |
| テンプレート変数（`vars.env`） | `initiated_by.id == "{{admin_id}}"` |
| ドメインロジックの不変条件 | `version == 1`（新規作成時）、`status == "Draft"` |
| ハードコードされた定数 | `current_step_id == "approval"` |
| 前のリクエストで Capture した値 | `display_id == "{{workflow_display_id}}"` |

判定テスト: 「テストを何度実行しても同じ値になるか？」→ Yes なら `==` を使う。

### Capture → == パターン

複数リクエストにまたがるテストでは、前のレスポンスの値を Capture し、後のレスポンスで `==` で比較する。これにより**値の安定性**（リクエスト間で値が変わらないこと）を検証できる。

```hurl
# 作成時に Capture
HTTP 201
[Captures]
workflow_id: jsonpath "$.data.id"
workflow_display_id: jsonpath "$.data.display_id"

# 更新後に == で検証（同じ値であることを確認）
HTTP 200
[Asserts]
jsonpath "$.data.id" == "{{workflow_id}}"
jsonpath "$.data.display_id" == "{{workflow_display_id}}"
```

## 必須チェック項目

### 1. OpenAPI required フィールドの検証

OpenAPI 仕様で `required` に含まれるフィールドは、**すべて**テストでアサーションする。

確認手順: OpenAPI の該当スキーマの `required` 配列と、テストのアサーションを突合する。

### 2. Cookie セキュリティ属性の検証

認証関連の Cookie を設定・クリアするエンドポイントでは、以下のセキュリティ属性を検証する:

```hurl
header "Set-Cookie" contains "HttpOnly"
header "Set-Cookie" contains "SameSite=Lax"
header "Set-Cookie" contains "Path=/"
```

注: `Secure` 属性はテスト環境（HTTP）では付与されないため、検証対象外。

### 3. 状態遷移の検証

状態変更 API（submit、approve、reject 等）では、変更前後の状態を検証する:

- `status` の期待値を `==` で検証
- `version` のインクリメントを `==` で検証（更新回数が確定している場合）
- null → 非 null に変わるフィールドの値を検証

## AI エージェントへの指示

API テストのアサーションを追加・変更する際:

1. 各フィールドの値が決定的かどうかを実装コードで確認する
2. 判断基準に従い、最も厳密なアサーションを選択する
3. OpenAPI の required フィールドがすべてアサーションされているか突合する

**禁止事項:**

- 値が決定的であるにもかかわらず `exists` や `isString` で済ませること
- OpenAPI の required フィールドをアサーションせずにコミットすること

## 参照

- Hurl ナレッジベース: [docs/06_ナレッジベース/devtools/hurl.md](../../docs/06_ナレッジベース/devtools/hurl.md)
- API 実装ルール: [.claude/rules/api.md](api.md)
- OpenAPI 仕様: [openapi/openapi.yaml](../../openapi/openapi.yaml)
