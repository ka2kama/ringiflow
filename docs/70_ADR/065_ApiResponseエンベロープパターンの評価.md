# ADR-065: ApiResponse エンベロープパターンの評価

## ステータス

承認済み（2026-03-09）

## コンテキスト

BFF の全エンドポイントは `ApiResponse<T>` で `{"data": T}` 形式にラップしてレスポンスを返している。このエンベロープパターンは初期設計時に導入されたが、判断理由が ADR に記録されていなかった。

PR #1092 で `Api.elm` に `Decode.field "data"` の自動アンラップを導入した際に、以下の問題が顕在化した:

1. `PaginatedResponse` が `{"data": [...], "next_cursor": "..."}` という異なる形式を使っており、`ApiResponse` とレスポンス構造が統一されていない
2. `PaginatedResponse` の `data` はペイロードのフィールド名であり、エンベロープの `data` とは意味が異なる
3. フロントエンドで `getRaw` というエスケープハッチが必要になった（自動アンラップをスキップするため）

現状のレスポンス形式:

| 型 | JSON 形式 | 用途 |
|---|-----------|------|
| `ApiResponse<T>` | `{"data": T}` | 単一データの全エンドポイント（88 箇所） |
| `PaginatedResponse<T>` | `{"data": [...], "next_cursor": "..."}` | ページネーション付き一覧（監査ログのみ） |
| `ErrorResponse` | RFC 9457 Problem Details | エラーレスポンス |

### エンベロープパターンの背景

エンベロープパターン（`{"data": T}` でレスポンスを包む設計）は Google JSON Style Guide 等で推奨されていたが、その主要な動機は:

- **JSONP**: クロスドメインリクエストで HTTP ヘッダーにアクセスできない環境向け → CORS の普及で不要に
- **成功/失敗の区別**: `{"data": ..., "error": ...}` で body 内に成功/失敗を格納 → HTTP ステータスコード + RFC 9457 で代替済み
- **メタデータの格納**: ページネーション情報等をトップレベルに追加 → HTTP ヘッダー（`Link`、カスタムヘッダー）で代替可能

現代の REST API 設計では「HTTP 自体がエンベロープである」が主流の考え方であり、GitHub API、Stripe API、WordPress API 等の著名な API はエンベロープを使用していない。

## 検討した選択肢

### 選択肢 A: エンベロープを維持し、PaginatedResponse を統合する

`PaginatedResponse` を `ApiResponse` でラップする形に変更する。

```json
{
  "data": {
    "items": [...],
    "next_cursor": "..."
  }
}
```

評価:
- 利点: 全エンドポイントのレスポンス形式が `{"data": ...}` で統一される。`getRaw` が不要になる
- 欠点: エンベロープの `data` フィールドが「ラッパー」としてのみ機能し、情報量がない。二重のネストが発生する。使われていない抽象層を「将来のため」に維持する形になる

### 選択肢 B: エンベロープを廃止し、T を直接返す

`ApiResponse<T>` を廃止し、各エンドポイントが `T` を直接返す。

```json
// 単一データ
{"id": "...", "name": "..."}

// ページネーション（コレクション固有の形式）
{"items": [...], "next_cursor": "..."}
```

評価:
- 利点: 不要なネストがなくなり、レスポンスがシンプルになる。`PaginatedResponse` との不整合が解消する。業界のベストプラクティスに沿う
- 欠点: バックエンド（88 箇所）+ API テスト（627 行）+ OpenAPI アノテーション（41 箇所）の変更が必要

### 選択肢 C: 現状維持（エンベロープを残すが統合はしない）

`ApiResponse` と `PaginatedResponse` を現状のまま維持する。

評価:
- 利点: コード変更が不要。PR #1092 の auto-unwrap + `getRaw` で運用上の問題は解消済み
- 欠点: レスポンス形式の不統一が残る。`data` フィールドの意味が文脈により異なる状態が続く。エンベロープの正当性が不明なまま放置される

### 比較表

| 観点 | A: 統合 | B: 廃止 | C: 現状維持 |
|------|---------|---------|------------|
| レスポンス形式の統一 | ○ | ○ | × |
| シンプルさ | △（二重ネスト） | ○（最もシンプル） | △（`getRaw` が必要） |
| 変更規模 | 小 | 中（全て機械的置換） | なし |
| 業界のベストプラクティスとの整合 | ×（エンベロープは非推奨が主流） | ○ | × |
| `data` の意味的一貫性 | ○ | ○（`data` 自体が消滅） | × |
| YAGNI 適合 | ×（使わない拡張性を維持） | ○ | △ |

## 決定

**選択肢 B: エンベロープを廃止し、T を直接返す。**

理由:

1. **エンベロープの正当性がない**: RingiFlow ではエラーを RFC 9457 Problem Details で返し、リクエスト ID 等のメタデータは HTTP ヘッダーで伝達している。エンベロープの `data` フィールドは情報を追加しておらず、純粋にネストのオーバーヘッドになっている

2. **変更は機械的で低リスク**: 実測の結果、全 88 箇所の `ApiResponse::new()` が同一パターンであり、100% 機械的に置換可能。フロントエンドは `Api.elm` の 1 行変更のみ（PR #1092 で auto-unwrap を 1 箇所に集約済み）。Hurl テストも `$.data.xxx` → `$.xxx` の機械的置換。当初「大規模変更」と評価したが、実測で覆った

3. **業界のベストプラクティスに沿う**: GitHub API、Stripe API 等の著名な API はエンベロープを使用しない。「HTTP がエンベロープである」という原則（ステータスコード + ヘッダー + ボディ）がモダンな REST API 設計の主流

選択肢 A を却下した理由: エンベロープ自体の正当性がない状態で形式を統一しても、使われていない抽象層を維持するコスト（コードの複雑さ、新規参画者の理解コスト）が残る。「将来 `meta` が必要になるかもしれない」は YAGNI に反し、要件定義書にもそのようなニーズは存在しない。

選択肢 C を却下した理由: PR #1092 の `getRaw` で運用は回っているが、エンベロープの正当性が不明なまま放置されることは技術的負債となる。

### ページネーションの扱い

`PaginatedResponse` の `{"items": [...], "next_cursor": "..."}` はエンベロープではなく、コレクションエンドポイント固有のレスポンス形式として維持する。Stripe API の `GET /customers` が `{"data": [...], "has_more": true}` を返すのと同様、これはリソースの表現であり汎用エンベロープとは区別される。

将来ページネーションメタデータ（total_count 等）が必要になった場合は、`Link` ヘッダー（RFC 8288）または `X-Total-Count` カスタムヘッダーでの提供を検討する。

## 帰結

### 肯定的な影響

- レスポンスから不要なネストが除去され、API がシンプルになる
- `PaginatedResponse` との形式不整合が解消する（エンベロープ自体がなくなるため）
- フロントエンドの `getRaw` エスケープハッチが不要になる
- 業界のベストプラクティスに沿った API 設計になる

### 否定的な影響・トレードオフ

- バックエンド 88 箇所 + API テスト 627 行 + OpenAPI 41 箇所の変更が必要（全て機械的置換）
- 将来トップレベルメタデータが必要になった場合、HTTP ヘッダーでの提供を設計する必要がある（ただし、これは HTTP の正しい使い方）

### 今後のアクション

- `ApiResponse<T>` を廃止し、`T` を直接返す形に変更する（別 Issue で追跡）
  - バックエンド: `ApiResponse::new(data)` → `data` に置換
  - フロントエンド: `Api.elm` の `expectJson` から `Decode.field "data"` を除去
  - API テスト: Hurl の `jsonpath "$.data.xxx"` → `jsonpath "$.xxx"` に置換
  - OpenAPI: アノテーションから `ApiResponse<T>` を除去
  - `PaginatedResponse` の `data` フィールドを `items` にリネーム（エンベロープの `data` との混同を防ぐ）

### 関連ドキュメント

- 実装: `backend/crates/shared/src/api_response.rs`、`backend/crates/shared/src/paginated_response.rs`
- 実装: `frontend/src/Api.elm`（auto-unwrap）
- 関連 PR: #1092（auto-unwrap 導入、`getRaw` 追加）

### 参考資料

- [Best Practices for Designing a Pragmatic RESTful API（Vinay Sahni）](https://www.vinaysahni.com/best-practices-for-a-pragmatic-restful-api) — 「エンベロープは一般的にモダンな RESTful API では避けられている」
- [Google JSON Style Guide](https://google.github.io/styleguide/jsoncstyleguide.xml) — エンベロープ推奨の原典（JSONP 時代の文脈）
- [RFC 9457: Problem Details for HTTP APIs](https://www.rfc-editor.org/rfc/rfc9457.html) — エラーレスポンスの標準化
- [RFC 8288: Web Linking](https://datatracker.ietf.org/doc/html/rfc8288) — ページネーション等のリンク関係をヘッダーで表現

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-03-09 | 初版作成 |
