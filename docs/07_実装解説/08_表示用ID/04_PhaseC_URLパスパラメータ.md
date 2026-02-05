# Phase C: URL パスパラメータに display_number を使用

## 概要

BFF の公開 API の URL パスパラメータを UUID から display_number（整数）に変更した。

```
Before: /api/v1/workflows/01924f3e-7a8b-7000-8000-000000000001
After:  /api/v1/workflows/42
```

### 対応 Issue

[#229 公開 API の URL パスパラメータに表示用番号を使用する](https://github.com/ka2kama/ringiflow/issues/229)

## 設計書との対応

- [表示用 ID 設計](../../../03_詳細設計書/12_表示用ID設計.md)

## 実装したコンポーネント

### Phase 1: リポジトリ層

| ファイル | 責務 |
|----------|------|
| [`workflow_instance_repository.rs`](../../../backend/crates/infra/src/repository/workflow_instance_repository.rs) | `find_by_display_number` メソッド追加 |
| [`workflow_step_repository.rs`](../../../backend/crates/infra/src/repository/workflow_step_repository.rs) | `find_by_display_number` メソッド追加 |

### Phase 2: Core Service

| ファイル | 責務 |
|----------|------|
| [`usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs) | display_number 対応のユースケース追加 |
| [`handler/workflow.rs`](../../../backend/apps/core-service/src/handler/workflow.rs) | 新規エンドポイント 4 件追加 |

新規エンドポイント:

```
GET  /internal/workflows/by-display-number/{dn}
POST /internal/workflows/by-display-number/{dn}/submit
POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/approve
POST /internal/workflows/by-display-number/{dn}/steps/by-display-number/{step_dn}/reject
```

### Phase 3: BFF

| ファイル | 責務 |
|----------|------|
| [`client/core_service.rs`](../../../backend/apps/bff/src/client/core_service.rs) | 新規クライアントメソッド追加 |
| [`handler/workflow.rs`](../../../backend/apps/bff/src/handler/workflow.rs) | パスパラメータを `Path<i64>` に変更 |

### Phase 4: フロントエンド

| ファイル | 責務 |
|----------|------|
| [`Route.elm`](../../../frontend/src/Route.elm) | `WorkflowDetail String` → `WorkflowDetail Int` |
| [`Api/Workflow.elm`](../../../frontend/src/Api/Workflow.elm) | URL 構築を display_number に変更 |
| [`Page/Workflow/Detail.elm`](../../../frontend/src/Page/Workflow/Detail.elm) | API 呼び出しを display_number に変更 |

### Phase 5: OpenAPI + API テスト

| ファイル | 責務 |
|----------|------|
| [`openapi.yaml`](../../../openapi/openapi.yaml) | パスパラメータの型を integer に変更 |
| [`create_workflow.hurl`](../../../tests/api/hurl/workflow/create_workflow.hurl) | display_number の Capture とアサーション追加 |
| [`submit_workflow.hurl`](../../../tests/api/hurl/workflow/submit_workflow.hurl) | URL を display_number に変更 |

## 実装内容

### リポジトリ: find_by_display_number

```rust
pub async fn find_by_display_number(
    &self,
    display_number: DisplayNumber,
    tenant_id: &TenantId,
) -> Result<Option<WorkflowInstance>, InfraError> {
    let row = sqlx::query_as!(
        WorkflowInstanceRow,
        r#"
        SELECT ... FROM workflow_instances
        WHERE tenant_id = $1 AND display_number = $2
        "#,
        tenant_id.as_ref(),
        display_number.as_ref()
    )
    .fetch_optional(&*self.pool)
    .await?;
    // ...
}
```

### フロントエンド: Route の型変更

```elm
-- Before
type Route
    = WorkflowDetail String
    ...

-- After
type Route
    = WorkflowDetail Int
    ...

parser : Parser (Route -> a) a
parser =
    oneOf
        [ map WorkflowDetail (s "workflows" </> int)  -- int パーサーを使用
        ...
        ]
```

## テスト

### 統合テスト

```bash
just test-rust-integration
```

| テスト | 内容 |
|--------|------|
| `find_by_display_number_returns_instance` | 存在する display_number で検索できる |
| `find_by_display_number_returns_none_for_nonexistent` | 存在しない display_number で None を返す |
| `find_by_display_number_respects_tenant_isolation` | 別テナントの display_number では見つからない |

### API テスト

```bash
just check-all
```

Hurl テストで display_number ベースの API 呼び出しを検証。

## 関連ドキュメント

- [表示用 ID 設計](../../../03_詳細設計書/12_表示用ID設計.md)
- [Phase A-1: DB スキーマ変更](./01_PhaseA1_DBスキーマ変更.md)
- [Phase A-2: 採番サービス](./02_PhaseA2_採番サービス.md)
- [Phase A-3: API + フロントエンド](./03_PhaseA3_APIとフロントエンド.md)

---

## 設計解説

### 1. ID 解決アプローチ: 方案 C（新規エンドポイント）を採用

**場所**: Core Service 全体

**なぜこの設計か**:

4つの方案を比較検討し、方案 C を採用した。

| 方案 | 概要 | 利点 | 欠点 |
|------|------|------|------|
| A: 2段階呼び出し | BFF が解決 API → 既存 API を順次呼び出し | 既存 API を変更しない | 2回の HTTP 呼び出し、パフォーマンス劣化 |
| B: 既存 API を両対応 | UUID と display_number のどちらでも受け付ける | 1回の呼び出し | 型安全性低下（実行時判定） |
| **C: 新規エンドポイント** | display_number 専用のエンドポイントを追加 | 1回の呼び出し、型安全 | エンドポイント数が増える |
| D: クエリパラメータ拡張 | `?display_number={dn}` で検索 | 既存パターンと一貫 | REST セマンティクス的に微妙 |

**採用理由**:

1. **パフォーマンス**: 1回の HTTP 呼び出しで完結
2. **型安全性**: パスパラメータは整数型として静的に検証可能
3. **後方互換性**: 既存の UUID ベース API は変更しない

### 2. API レスポンスに display_number フィールドを追加

**場所**: Core Service DTO、BFF レスポンス、OpenAPI 仕様

**なぜこの設計か**:

フロントエンドで display_number を取得する方法として、2つの選択肢があった：

| 方法 | 概要 |
|------|------|
| displayId をパース | `"WF-42"` から `42` を抽出 |
| **API が直接提供** | レスポンスに `display_number: 42` を含める |

**採用理由**:

- **Single Source of Truth**: API が正規の display_number を提供
- **フォーマット変更耐性**: displayId のフォーマットが変わってもパースロジックに影響しない
- **シンプル**: フロントエンドでのパース処理が不要

### 3. Elm の型システムによる変更箇所の自動検出

**場所**: `Route.elm`, `Page/Workflow/Detail.elm`, `Api/Workflow.elm`

**コード例**:

```elm
-- Route.elm の変更
type Route
    = WorkflowDetail Int  -- String から Int に変更
```

この変更により、Elm コンパイラが以下を自動検出：

- `Page/Workflow/Detail.elm` の `init` 関数のシグネチャ
- `Page/Workflow/List.elm` のリンク構築
- `RouteTest.elm` のテストデータ

**なぜこの設計か**:

Elm の強い型付けにより、変更箇所の漏れを構造的に防止できる。UUID（String）から display_number（Int）への変更は、型の変更として表現されるため、コンパイラが全ての関連箇所を指摘してくれる。

これは「型で表現できるものは型で表現する」というプロジェクトの設計原則に合致している。
