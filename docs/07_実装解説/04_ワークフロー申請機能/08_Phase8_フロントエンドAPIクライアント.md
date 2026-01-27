# Phase 8: フロントエンド API クライアント

## 概要

Elm でワークフロー申請機能の API クライアント層を実装する。

### 対応 Issue

[#115 フロントエンド ワークフロー申請フォーム](https://github.com/ka2kama/ringiflow/issues/115)

## 前提: GET API の追加

Phase 7 で実装した BFF API は POST エンドポイントのみだった。
フロントエンド実装に先立ち、以下の GET エンドポイントを追加。

| エンドポイント | 用途 |
|---------------|------|
| `GET /api/v1/workflow-definitions` | 定義一覧（申請フォームでの選択肢） |
| `GET /api/v1/workflow-definitions/{id}` | 定義詳細（フォームフィールド取得） |
| `GET /api/v1/workflows` | 自分の申請一覧 |
| `GET /api/v1/workflows/{id}` | 申請詳細 |

実装箇所:
- Core Service: `usecase/workflow.rs`, `handler/workflow.rs`
- BFF: `client/core_service.rs`, `handler/workflow.rs`
- OpenAPI: `openapi/openapi.yaml`

## 実装内容

### 1. データモジュール（`Data/`）

バックエンドの型に対応する Elm の型とデコーダーを定義。

| モジュール | 用途 |
|-----------|------|
| `Data/WorkflowDefinition.elm` | ワークフロー定義（選択可能なテンプレート） |
| `Data/WorkflowInstance.elm` | ワークフローインスタンス（申請案件） |
| `Data/FormField.elm` | 動的フォームフィールド定義 |

### 2. HTTP ヘルパー（`Api/Http.elm`）

BFF への API リクエストに必要な共通処理を提供。

機能:
- `X-Tenant-ID` ヘッダーの付与
- `X-CSRF-Token` ヘッダーの付与（状態変更リクエストのみ）
- RFC 7807 Problem Details エラーレスポンスのデコード
- HTTP エラーの型安全な分類

### 3. API クライアント（`Api/`）

| モジュール | エンドポイント |
|-----------|---------------|
| `Api/WorkflowDefinition.elm` | `GET /api/v1/workflow-definitions`, `GET /api/v1/workflow-definitions/{id}` |
| `Api/Workflow.elm` | `GET /api/v1/workflows`, `GET /api/v1/workflows/{id}`, `POST /api/v1/workflows`, `POST /api/v1/workflows/{id}/submit` |

## 設計判断

### JSON デコーダーの実装方式

`NoRedInk/elm-json-decode-pipeline` パッケージを採用。

```elm
decoder : Decoder WorkflowDefinition
decoder =
    Decode.succeed WorkflowDefinition
        |> required "id" Decode.string
        |> required "name" Decode.string
        |> optional "description" (Decode.nullable Decode.string) Nothing
```

採用理由:
- パイプライン形式で可読性が高い
- `required` / `optional` で必須・任意フィールドを明示
- NoRedInk 社（Elm を本番運用する企業）がメンテナンス

代替案:
- `Decode.map8` 等: 引数の順序ミスが発生しやすい
- 手動パイプライン: ボイラープレートが増える

### エラー型の設計

```elm
type ApiError
    = BadRequest ProblemDetails
    | Unauthorized
    | Forbidden ProblemDetails
    | NotFound ProblemDetails
    | ServerError ProblemDetails
    | NetworkError
    | Timeout
    | DecodeError String
```

設計意図:
- HTTP ステータスコードごとに適切な UI 表示を可能にする
- `Unauthorized` のみ `ProblemDetails` を持たない（ログイン画面へリダイレクトするため詳細不要）
- `DecodeError` でデコード失敗時のデバッグ情報を保持

### elm-review 除外設定

Phase 1 では API クライアント層のみ実装し、UI は Phase 2 で実装する。
そのため、作成したモジュールは一時的に「未使用」となる。

対応:
- `ReviewConfig.elm` で対象モジュールを除外設定
- TODO コメントで「Phase 2 実装後に除外設定を削除」と明記

```elm
NoUnused.Modules.rule
    -- TODO: Phase 1 で作成した API/Data モジュール。Phase 2 で UI 実装後に除外設定を削除する
    |> Rule.ignoreErrorsForFiles
        [ "src/Api/Http.elm"
        , "src/Api/Workflow.elm"
        , ...
        ]
```

## ファイル構成

```
frontend/src/
├── Api/
│   ├── Http.elm              # HTTP ヘルパー
│   ├── Workflow.elm          # ワークフロー API
│   └── WorkflowDefinition.elm # ワークフロー定義 API
└── Data/
    ├── FormField.elm          # フォームフィールド型
    ├── WorkflowDefinition.elm # ワークフロー定義型
    └── WorkflowInstance.elm   # ワークフローインスタンス型
```

## 学習ポイント

1. **Elm の JSON デコード**: `Json.Decode.Pipeline` による宣言的なデコーダー定義
2. **HTTP リクエストの抽象化**: 共通ヘッダーを一箇所で管理
3. **RFC 7807 対応**: 標準化されたエラーレスポンス形式の活用
4. **段階的実装**: elm-review の除外設定を活用した段階的開発

## 次のステップ

- Phase 9: 申請フォーム UI（`Page/Workflow/New.elm`）
- Phase 10: 申請一覧・詳細ページ
