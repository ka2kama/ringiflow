# 既存テストの ADR-032 準拠化

## Context

ADR-032（テスト設計方針）で定義した命名規則・構造化方法に対し、既存テストの一部がまだ準拠していない。ADR-032 の移行方針では Phase 2（ボーイスカウトルール）と Phase 3（一括適用）を定義しており、今回は Phase 3 として「現時点で合理的に修正できるもの」を一括修正する。

併せて、ADR-032 の `fixture_` プレフィックス規則を削除する。`#[fixture]` 属性が識別の役割を既に果たしており、rstest の引数名マッチングとの相性が悪い（プレフィックスが全呼び出し側に伝播する）ため。

## スコープ

### 対象

| ファイル | 修正内容 |
|---------|---------|
| `docs/70_ADR/032_テスト設計方針.md` | `fixture_` プレフィックス規則の削除 |
| `backend/crates/shared/src/api_response.rs` | テスト関数に `test_` プレフィックス追加 |
| `backend/crates/domain/src/workflow.rs` | rstest 導入 + non-snake-case テスト名の日本語化 |
| `backend/crates/domain/src/password.rs` | 不要な `#[allow(non_snake_case)]` 削除 |
| `backend/crates/infra/tests/workflow_definition_repository_test.rs` | 英語テスト名を日本語化 |

### 対象外

- ハンドラ・ユースケーステスト: Mock 構成が複雑で、今回のスコープでは変更リスクが高い
- Elm テスト: 別途分析が必要
- Hurl テスト: 既に ADR-032 準拠
- 統合テスト（workflow_definition_repository_test 以外）: 既に日本語命名

## 修正内容

### 1. ADR-032: `fixture_` プレフィックス規則の削除

命名規則テーブルのフィクスチャ行を変更:

```
Before: fixture_[対象の説明]（日本語） | fn fixture_システムロール() -> Role
After:  [対象の説明]（日本語） | fn システムロール() -> Role
```

理由: `#[fixture]` 属性で識別可能。プレフィックスは rstest の引数名マッチングで全呼び出し側に伝播し、可読性を下げる。

ASCII プレフィックスの理由セクションからフィクスチャに関する記述を削除し、`#[fixture]` 属性で識別する旨を追記する。

### 2. `api_response.rs`: `test_` プレフィックス追加

| Before | After |
|--------|-------|
| `serialize_を正しいjson形状にする` | `test_serializeを正しいjson形状にする` |
| `deserialize_でjsonからオブジェクトに変換する` | `test_deserializeでjsonからオブジェクトに変換する` |
| `serialize_deserialize_のラウンドトリップ` | `test_serialize_deserializeのラウンドトリップ` |
| `vec_ペイロードをシリアライズする` | `test_vecペイロードをシリアライズする` |

### 3. `workflow.rs`: rstest 導入 + テスト名日本語化

#### 3a. ヘルパー関数を rstest フィクスチャに変換

| Before | After |
|--------|-------|
| `fn test_now() -> DateTime<Utc>` | `#[fixture] fn now() -> DateTime<Utc>` |
| `fn create_test_instance() -> WorkflowInstance` | `#[fixture] fn テスト用インスタンス() -> WorkflowInstance` |
| `fn create_test_step(instance_id) -> WorkflowStep` | `#[fixture] fn テスト用ステップ() -> WorkflowStep` |

`テスト用ステップ` は `WorkflowInstanceId::new()` を内部で生成する（呼び出し側の大半がそうしているため）。

#### 3b. テスト関数を `#[rstest]` に変換

各テスト関数で:
- `#[test]` → `#[rstest]`
- `create_test_instance()` → 引数 `テスト用インスタンス: WorkflowInstance`
- `create_test_step(WorkflowInstanceId::new())` → 引数 `テスト用ステップ: WorkflowStep`
- `test_now()` → 引数 `now: DateTime<Utc>`

例外: `is_overdue` テスト（`from_db` で個別データを構築）はヘルパーもフィクスチャも使わないためそのまま `#[test]`。ただし `test_now()` の直接呼び出しは `now` フィクスチャに置き換え不可のため、テスト内でローカルに定義するか `#[rstest]` + `now` で受け取る。

→ `is_overdue` テストは `#[rstest]` にして `now` をフィクスチャで受け取る。

#### 3c. non-snake-case テスト名の日本語化

| Before | After |
|--------|-------|
| `test_承認完了でステータスがApprovedになる` | `test_承認完了でステータスが承認済みになる` |
| `test_却下完了でステータスがRejectedになる` | `test_却下完了でステータスが却下済みになる` |
| `test_InProgress以外で承認完了するとエラー` | `test_処理中以外で承認完了するとエラー` |
| `test_InProgress以外で却下完了するとエラー` | `test_処理中以外で却下完了するとエラー` |
| `test_approveでCompletedとApprovedになる` | `test_承認で完了と承認済みになる` |
| `test_approveでversionがインクリメントされる` | `test_承認でversionがインクリメントされる` |
| `test_approveでコメントが設定される` | `test_承認でコメントが設定される` |
| `test_rejectでCompletedとRejectedになる` | `test_却下で完了と却下済みになる` |
| `test_rejectでversionがインクリメントされる` | `test_却下でversionがインクリメントされる` |
| `test_Active以外でapproveするとエラー` | `test_アクティブ以外で承認するとエラー` |
| `test_Active以外でrejectするとエラー` | `test_アクティブ以外で却下するとエラー` |
| `test_submitted後のsubmitted_atは注入された値と一致する` | `test_申請後のsubmitted_atは注入された値と一致する` |
| `test_activated後のstarted_atは注入された値と一致する` | `test_アクティブ化後のstarted_atは注入された値と一致する` |

#### 3d. `#[allow(non_snake_case)]` の削除

日本語化により ASCII 大文字がなくなるため、両方の内部 mod から `#[allow(non_snake_case)]` を削除する。

### 4. `password.rs`（domain）: 不要な `#[allow(non_snake_case)]` 削除

テスト名に ASCII 大文字がないため不要。L98 の `#[allow(non_snake_case)]` を削除する。

### 5. `workflow_definition_repository_test.rs`: 英語テスト名の日本語化

| Before | After |
|--------|-------|
| `test_find_published_by_tenant_returns_published_definitions` | `test_テナントの公開済み定義一覧を取得できる` |
| `test_find_published_by_tenant_filters_by_tenant` | `test_別テナントの定義は取得できない` |
| `test_find_by_id_returns_definition_when_exists` | `test_idで定義を取得できる` |
| `test_find_by_id_returns_none_when_not_exists` | `test_存在しないidの場合noneを返す` |

## 検証

```bash
just check       # lint + unit test
just check-all   # lint + unit test + integration test + API test
```

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 全テストファイルの ADR-032 準拠状況を調査 | 22 ファイルの `#[cfg(test)]` ブロックを確認、テスト関数名・構造・rstest 使用状況を網羅的に分析 | 修正が必要な 5 ファイルを特定。ハンドラ・ユースケーステストはスコープ外と判断 |
| 2回目 | `fixture_` プレフィックスの適用可否 | rstest の引数名マッチング機構と `fixture_` プレフィックスの相互作用を検討。既存コードで `fixture_` / `#[from()]` の使用例がゼロであることを確認 | `fixture_` プレフィックスは rstest との相性が悪く（全呼び出し側に伝播）、ADR 修正で削除する方針に決定 |
| 3回目 | workflow.rs の rstest 変換パターン | `is_overdue` テストの特殊性（`from_db` で個別データ構築）を確認。フィクスチャ化不要なケースの扱いを検討 | `is_overdue` テストは `#[rstest]` + `now` フィクスチャのみ使用し、ステップは直接構築のまま |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | 22 ファイルの `#[cfg(test)]` を確認し、ADR-032 非準拠の 5 ファイルをすべて特定。ハンドラ・ユースケーステストは明示的にスコープ外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全テスト名の Before/After を明示。fixture 変換パターンも具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | `fixture_` プレフィックスの判断理由、`is_overdue` テストの例外扱い、ハンドラ・ユースケースの除外理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「スコープ」セクションで対象 5 ファイルと対象外を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | rstest の引数名マッチング機構、`#[allow(non_snake_case)]` の発動条件（ASCII 大文字のみ）を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | ADR-032 の修正を計画に含めており、ドキュメントと実装が整合する |
