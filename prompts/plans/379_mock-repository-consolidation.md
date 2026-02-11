# 計画: #379 UseCase テストの Mock リポジトリ共通化

## Context

jscpd（#373）で検出されたテストコード重複を解消する。core-service の 3 つのユースケーステストファイルで、同一の Mock リポジトリ構造体が繰り返し定義されている。リポジトリトレイトに変更があると 3 箇所すべてを修正する必要があり、保守性を損なっている。

## スコープ

### 対象

- `backend/apps/core-service/src/usecase/task.rs` — MockWorkflowInstanceRepository, MockWorkflowStepRepository, MockUserRepository
- `backend/apps/core-service/src/usecase/dashboard.rs` — MockWorkflowInstanceRepository, MockWorkflowStepRepository
- `backend/apps/core-service/src/usecase/workflow/command.rs` — MockWorkflowDefinitionRepository, MockWorkflowInstanceRepository, MockWorkflowStepRepository, MockUserRepository, MockDisplayIdCounterRepository

### 対象外

- BFF のサービスクライアントスタブ（`StubCoreServiceClient` 等）。BFF 内部のトレイトを実装するもので、リポジトリモックとは異なるカテゴリ。

## 設計判断

### Mock の配置先: infra クレートに `test-utils` feature で公開

| 選択肢 | 評価 |
|--------|------|
| A. infra クレートに feature gate（採用） | トレイト定義と同じクレート。`domain` の `FixedClock` と同じパターン。新クレート不要 |
| B. 新 `test-utils` クレート | 分離は良いが、モック 5 個のために新ワークスペースメンバーは過剰 |
| C. `mockall` 自動生成 | 新依存、マクロの複雑さ。現在の手書きモックはインメモリ動作を提供しており `mockall` の期待値ベースより高忠実度 |

参考パターン: `backend/crates/domain/src/clock.rs` の `#[cfg(any(test, feature = "test-support"))]`

### feature 名: `test-utils`（`test-support` とは別）

`domain` クレートの `test-support` は `FixedClock` 単体を公開する。`infra` の `test-utils` はモックリポジトリ群を公開する。異なるセマンティクスなので名前を分ける。

### カノニカル実装: `command.rs` 版を採用

`task.rs` / `dashboard.rs` の `update_with_version_check` は no-op（`Ok(())`）だが、`command.rs` 版はフルの楽観的ロック動作を持つ。`command.rs` 版はスーパーセットであり、version check を呼ばないテストには影響しない。

### Mock API: 現行パターンを維持

`new()` + `Arc<Mutex<Vec<T>>>` インメモリパターンを維持。シンプルで実証済み。

## Phase 分割

### Phase 1: infra クレートに mock モジュールを作成

#### 確認事項

- 型: `InfraError` の定義 → `backend/crates/infra/src/error.rs` ✅ 確認済み
- パターン: `FixedClock` の feature gate → `backend/crates/domain/src/clock.rs` ✅ 確認済み（`#[cfg(any(test, feature = "test-support"))]`）
- パターン: repository トレイトの use path → `ringiflow_infra::repository::*` ✅ 確認済み

#### テストリスト

- [ ] `MockWorkflowDefinitionRepository` が `WorkflowDefinitionRepository` を実装する（コンパイル確認）
- [ ] `MockWorkflowInstanceRepository` が `WorkflowInstanceRepository` を実装する（コンパイル確認）
- [ ] `MockWorkflowStepRepository` が `WorkflowStepRepository` を実装する（コンパイル確認）
- [ ] `MockUserRepository` が `UserRepository` を実装する（コンパイル確認）
- [ ] `MockDisplayIdCounterRepository` が `DisplayIdCounterRepository` を実装する（コンパイル確認）
- [ ] `just check` が通る

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/Cargo.toml` | `[features] test-utils = []` を追加 |
| `backend/crates/infra/src/lib.rs` | `#[cfg(any(test, feature = "test-utils"))] pub mod mock;` を追加 |
| `backend/crates/infra/src/mock.rs` | **新規作成**: 5 つの Mock リポジトリ実装 |

`mock.rs` に含めるモック（すべて `pub`）:

- `MockWorkflowDefinitionRepository` — `command.rs:574-622` から抽出（`add_definition()` 含む）
- `MockWorkflowInstanceRepository` — `command.rs:624-734` から抽出（version check あり）
- `MockWorkflowStepRepository` — `command.rs:736-835` から抽出（version check あり）
- `MockUserRepository` — `command.rs:837-879` から抽出（unit struct）
- `MockDisplayIdCounterRepository` — `command.rs:884-908` から抽出（カウンター）

### Phase 2: core-service テストを共有モックに移行

#### 確認事項

確認事項: なし（Phase 1 で確認済みのパターンのみ）

#### テストリスト

- [ ] `task.rs` の全 11 テストが共有モックで通る
- [ ] `dashboard.rs` の全 5 テストが共有モックで通る
- [ ] `command.rs` の全 12 テストが共有モックで通る
- [ ] `just check-all` が通る

#### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/core-service/Cargo.toml` | dev-dependencies に `features = ["test-utils"]` を追加 |
| `backend/apps/core-service/src/usecase/task.rs` | Mock 定義（lines 256-497）を削除、`use ringiflow_infra::mock::*` に置換 |
| `backend/apps/core-service/src/usecase/dashboard.rs` | Mock 定義（lines 133-321）を削除、`use ringiflow_infra::mock::*` に置換 |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | Mock 定義（lines 574-908）を削除、`use ringiflow_infra::mock::*` に置換 |

移行の注意点:
- `task.rs` の `MockUserRepository` は `ringiflow_infra::repository::UserRepository` をフルパスで impl していたが、共有モックでは use が解決済み
- 可視性を `pub` に変更するため、テスト側の `use` を調整

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `update_with_version_check` の実装差異（task.rs/dashboard.rs は no-op、command.rs はフル実装） | 不完全なパス | command.rs 版をカノニカルとして採用。no-op 利用側はバージョンチェックパスを呼ばないため影響なし |
| 2回目 | BFF のスタブを同じスコープに含めるべきか | スコープ境界 | BFF スタブはサービスクライアントトレイト（BFF 内部定義）を実装するもので、リポジトリモックとは異なるカテゴリ。対象外と明示 |
| 3回目 | `TenantRepository` / `CredentialsRepository` のモックが必要か | 網羅性 | 3 つのテストファイルいずれでも使用されていない。必要になった時点で追加すればよい（YAGNI） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 重複する全モックが計画に含まれている | OK | 3 ファイルの全モック定義を Read で確認。5 種類の Mock を特定 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各モックのカノニカル版（command.rs）と行番号を明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | 配置先（A/B/C）、feature 名、カノニカル版選択、BFF スコープの判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象: core-service の 3 ファイル。対象外: BFF スタブ |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | `#[cfg(test)]` の他クレートからの不可視性を確認。feature gate パターンを domain クレートで検証済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | structural-review.md の責務重複チェック指針に合致。関連 ADR なし |

## 検証方法

```bash
# Phase 1: コンパイル確認
cd backend && cargo check --package ringiflow-infra --features test-utils

# Phase 2: テスト実行
cd backend && cargo test --package ringiflow-core-service

# 最終確認
just check-all
```

## 推定インパクト

- 削除: ~750 行（3 ファイルの重複モック定義）
- 追加: ~320 行（`mock.rs` + Cargo.toml/lib.rs 変更）
- 純減: ~430 行
