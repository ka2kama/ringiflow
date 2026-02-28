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

## エッジケーステスト方針

API テストでは、ユニットテストで守ったルールが HTTP 層まで伝播していることを確認する。全境界値の網羅はユニットテストの責務であり、API テストでは各カテゴリ 1-2 ケースの代表値で伝播を確認する。

### エッジケースカテゴリ

| カテゴリ | テスト対象 | 期待レスポンス | テストケース数の目安 |
|---------|-----------|-------------|-------------------|
| 入力値境界（伝播確認） | 必須フィールドへの空文字・最大長超過 | 422 | エンドポイントごとに 1-2 件 |
| ページネーション無効値 | limit=0、offset=-1、極大値 | 422 または安全なデフォルト値 | 一覧 API 共通で 2-3 件 |
| 権限境界 | 他テナント/他ユーザーのリソースアクセス | 403 / 404 | 認可パターンごとに 1 件 |
| 状態遷移異常系 | 無効な状態からの操作 | 409 | 代表的な不正遷移 2-3 件 |
| 楽観的ロック競合 | 古い version での更新 | 409 | 更新 API ごとに 1 件 |
| 重複データ | 一意制約違反 | 409 | 一意制約を持つ作成 API ごとに 1 件 |

### テストファイル内の配置

エッジケーステストは既存のテストファイル内にセクションコメントで区切って追加する:

```hurl
# ── 異常系: 入力値境界 ──

# 空文字のタイトルで作成
POST {{base_url}}/api/v1/workflows
# ...
HTTP 422

# ── 異常系: 状態遷移 ──

# 完了済みワークフローへの承認
POST {{base_url}}/api/v1/workflows/{{workflow_id}}/steps/{{step_id}}/approve
# ...
HTTP 409
```

→ 全体方針: [テスト戦略: エッジケース方針](../../docs/50_テスト/テスト戦略_エッジケース方針.md)

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

- Hurl ナレッジベース: [docs/80_ナレッジベース/devtools/hurl.md](../../docs/80_ナレッジベース/devtools/hurl.md)
- API 実装ルール: [.claude/rules/api.md](api.md)
- OpenAPI 仕様: [openapi/openapi.yaml](../../openapi/openapi.yaml)
