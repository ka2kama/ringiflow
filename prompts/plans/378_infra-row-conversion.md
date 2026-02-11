# Issue #378: Infra 層の Row→Entity 変換コード共通化

## Context

jscpd（#373）で検出された Row→Entity 変換コードの重複を解消する。`sqlx::query!` は匿名構造体を返すため、変換ロジックを関数として切り出せず、同一の変換コードがメソッドごとにコピーされている。

- `workflow_step_repository.rs`: 4 メソッドで同一の 26 行ブロックが重複
- `workflow_instance_repository.rs`: 5 メソッドで同一の 23 行ブロックが重複

## Issue 精査

| 観点 | 判定 |
|------|------|
| Want | 保守性の向上（品質の追求）。変換ロジックの一元化により、フィールド追加時の修正漏れリスクを排除 |
| How への偏り | Issue は `From<Row>` と変換ヘルパーの2案を提示。変換は fallible なので `TryFrom` が適切 |
| 完了基準の妥当性 | 変換ロジックが各リポジトリで1箇所に集約されていれば Want を満たす |
| スコープの適切さ | 2ファイルに限定。workflow_definition（2重複）、user（4重複）は別 Issue で対応可 |

## 設計判断: `query_as!` + Row 構造体 + `TryFrom`

### 選択肢

| # | 案 | メリット | デメリット |
|---|-----|---------|----------|
| A | **`query_as!` + Row 構造体 + `TryFrom`** | Rust イディオマティック、使用箇所が最も簡潔、コンパイル時 SQL 検証維持 | Row 構造体の追加、query マクロ変更 |
| B | `query!` + 個別引数ヘルパー関数 | query マクロ変更不要 | 16+ 引数で非現実的 |
| C | `query!` + マクロで変換生成 | query マクロ変更不要 | 可読性低下、デバッグ困難 |

### 採用: 案 A

理由:
1. `TryFrom` は fallible 型変換の標準トレイト。Row→Entity はまさにこのユースケース
2. `query_as!` はコンパイル時 SQL 検証を維持（`query!` と同等の安全性）
3. 使用箇所: `row.map(WorkflowStep::try_from).transpose()` / `rows.into_iter().map(WorkflowStep::try_from).collect()`

### 配置

- Row 構造体: 各リポジトリファイル内（private）。インフラ層の関心事
- `TryFrom` impl: 同ファイル内。`WorkflowStepRow` がローカル型のため orphan rule に抵触しない

### 副次的な正規化

リファクタリングに合わせて、2ファイル間の軽微な不一致を統一:

| 項目 | step repo（現在） | instance repo（現在） | 統一後 |
|------|------------------|---------------------|--------|
| DisplayNumber 生成 | `DisplayNumber::new()` | `DisplayNumber::try_from()` | `DisplayNumber::new()`（プロジェクト全体で多数派） |
| Status パース | `WorkflowStepStatus::from_str()` | `.parse::<Status>()` | `.parse::<T>()`（Rust イディオマティック） |

## 対象・対象外

対象:
- `workflow_step_repository.rs` の find 系 4 メソッドの変換ブロック
- `workflow_instance_repository.rs` の find 系 5 メソッドの変換ブロック

対象外:
- `insert` / `update_with_version_check` メソッド（書き込みのみ、変換なし）
- `workflow_definition_repository.rs`（2重複、別 Issue で対応）
- `user_repository.rs`（4重複、Record 構造体の導入が先に必要、別 Issue で対応）

## 実装計画

### Phase 1: WorkflowStepRepository のリファクタリング

#### 確認事項
- 型: `WorkflowStepRecord` のフィールド → `domain/src/workflow/step.rs:144-168`
- パターン: `sqlx::query_as!` の使い方 → docs.rs（プロジェクト内に既存使用なし）
- テスト: 既存統合テスト 13 件 → `infra/tests/workflow_step_repository_test.rs`

#### 実装内容

1. `WorkflowStepRow` 構造体を定義（raw DB 型: Uuid, i64, String, i32 等）
2. `TryFrom<WorkflowStepRow> for WorkflowStep` を実装（変換ロジックを一元化）
3. `find_by_id`, `find_by_instance`, `find_by_assigned_to`, `find_by_display_number` の 4 メソッドを `query_as!` + `try_from` にリファクタ

#### WorkflowStepRow のフィールド定義

```rust
struct WorkflowStepRow {
    id: Uuid,
    instance_id: Uuid,
    display_number: i64,
    step_id: String,
    step_name: String,
    step_type: String,
    status: String,
    version: i32,
    assigned_to: Option<Uuid>,
    decision: Option<String>,
    comment: Option<String>,
    due_date: Option<DateTime<Utc>>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

#### リファクタ後のメソッド例

```rust
// find_by_id（Option 返却）
async fn find_by_id(...) -> Result<Option<WorkflowStep>, InfraError> {
    let row = sqlx::query_as!(WorkflowStepRow, r#"SELECT ... WHERE id = $1 AND tenant_id = $2"#, ...)
        .fetch_optional(&self.pool).await?;
    row.map(WorkflowStep::try_from).transpose()
}

// find_by_instance（Vec 返却）
async fn find_by_instance(...) -> Result<Vec<WorkflowStep>, InfraError> {
    let rows = sqlx::query_as!(WorkflowStepRow, r#"SELECT ... WHERE instance_id = $1 AND tenant_id = $2 ORDER BY ..."#, ...)
        .fetch_all(&self.pool).await?;
    rows.into_iter().map(WorkflowStep::try_from).collect()
}
```

#### テストリスト（既存テスト、すべてパスすること）
- [ ] test_insert_で新規ステップを作成できる
- [ ] test_find_by_id_でステップを取得できる
- [ ] test_find_by_id_存在しない場合はnoneを返す
- [ ] test_find_by_instance_インスタンスのステップ一覧を取得できる
- [ ] test_find_by_instance_別テナントのステップは取得できない
- [ ] test_find_by_assigned_to_担当者のタスク一覧を取得できる
- [ ] test_update_with_version_check_バージョン一致で更新できる
- [ ] test_update_with_version_check_バージョン不一致でconflictエラーを返す
- [ ] test_update_with_version_check_別テナントのステップは更新できない
- [ ] test_find_by_display_number_存在するdisplay_numberで検索できる
- [ ] test_find_by_display_number_存在しない場合はnoneを返す
- [ ] test_find_by_display_number_別のinstance_idでは見つからない
- [ ] test_ステップを完了できる

### Phase 2: WorkflowInstanceRepository のリファクタリング

#### 確認事項: なし（Phase 1 と同一パターン）

#### 実装内容

1. `WorkflowInstanceRow` 構造体を定義
2. `TryFrom<WorkflowInstanceRow> for WorkflowInstance` を実装
3. `find_by_id`, `find_by_tenant`, `find_by_initiated_by`, `find_by_ids`, `find_by_display_number` の 5 メソッドをリファクタ

#### WorkflowInstanceRow のフィールド定義

```rust
struct WorkflowInstanceRow {
    id: Uuid,
    tenant_id: Uuid,
    definition_id: Uuid,
    definition_version: i32,
    display_number: i64,
    title: String,
    form_data: serde_json::Value,
    status: String,
    version: i32,
    current_step_id: Option<String>,
    initiated_by: Uuid,
    submitted_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}
```

#### テストリスト（既存テスト、すべてパスすること）
- [ ] test_insert_で新規インスタンスを作成できる
- [ ] test_find_by_id_でインスタンスを取得できる
- [ ] test_find_by_id_存在しない場合はnoneを返す
- [ ] test_find_by_tenant_テナント内の一覧を取得できる
- [ ] test_find_by_tenant_別テナントのインスタンスは取得できない
- [ ] test_find_by_initiated_by_申請者によるインスタンスを取得できる
- [ ] test_update_with_version_check_バージョン一致で更新できる
- [ ] test_update_with_version_check_バージョン不一致でconflictエラーを返す
- [ ] test_find_by_ids_空のvecを渡すと空のvecが返る
- [ ] test_find_by_ids_存在するidを渡すとインスタンスが返る
- [ ] test_find_by_ids_存在しないidを含んでも存在するもののみ返る
- [ ] test_find_by_ids_テナントidでフィルタされる
- [ ] test_find_by_display_number_存在するdisplay_numberで検索できる
- [ ] test_find_by_display_number_存在しない場合はnoneを返す
- [ ] test_find_by_display_number_別テナントでは見つからない

### Phase 3: sqlx-prepare と最終検証

1. `just sqlx-prepare` でキャッシュ更新（`query!` → `query_as!` でハッシュが変わる）
2. `just check-all` 通過確認

## 変更対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/src/repository/workflow_step_repository.rs` | Row 構造体 + TryFrom 追加、find 系 4 メソッドをリファクタ |
| `backend/crates/infra/src/repository/workflow_instance_repository.rs` | Row 構造体 + TryFrom 追加、find 系 5 メソッドをリファクタ |

ドメイン層・テストファイルの変更は不要（振る舞い変更なし）。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `DisplayNumber::new()` vs `try_from()` の不一致 | 既存手段の見落とし | 両者は等価（`try_from` は `new` を呼ぶ）。`new()` に統一する方針を追記 |
| 2回目 | `from_str()` vs `.parse::<T>()` の不一致 | ベストプラクティス | `.parse::<T>()` に統一する方針を追記（Rust イディオマティック） |
| 3回目 | orphan rule で `TryFrom` 実装が可能か | 技術的前提 | `WorkflowStepRow` がローカル型のため問題なし。fundamental trait の特例適用 |
| 4回目 | `query_as!` の列名マッピング方式 | 技術的前提 | 列名 = フィールド名で一致。現在の SELECT 列名と Row struct フィールド名が一致することを確認済み |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 両ファイルの全 find メソッド（4+5=9）を列挙し、既存テスト（13+15=28）との対応を確認 |
| 2 | 曖昧さ排除 | OK | Row 構造体のフィールド定義を型レベルで明示。リファクタ後のメソッド例をコード提示 |
| 3 | 設計判断の完結性 | OK | 3案比較 + 採用理由を記載。正規化の判断も根拠付き |
| 4 | スコープ境界 | OK | 対象（2ファイル・find 系のみ）と対象外（insert/update、他リポジトリ）を明記 |
| 5 | 技術的前提 | OK | `query_as!` の動作、orphan rule、`DisplayNumber::new` = `try_from` を確認済み |
| 6 | 既存ドキュメント整合 | OK | 関連 ADR なし。Issue #378 の方針（From<Row> トレイト）と整合（TryFrom = fallible 版 From） |
