# #290 Split core_service.rs using sub-trait pattern

## Context

Issue #290「Refactor oversized files (500+ lines)」の残り対応。

### 500 行閾値の再評価

分割済みの 2 ファイル（domain/workflow.rs、usecase/workflow.rs）は責務混在が明確で、分割は正当だった。しかし残り 5 ファイルを分析した結果、行数超過の原因はファイルごとに異なり、一律分割は不適切と判断:

| ファイル | 行数 | 本体/テスト | 本当の問題 | 対応 |
|---------|------|-----------|-----------|------|
| `core_service.rs` | 1212 | 1208/4 | ISP 違反 | **この PR で分割** |
| `auth.rs` | 1135 | 450/685 | ISP の症状（スタブ肥大化） | core_service 分割で自然に改善 |
| `task.rs` | 1042 | 225/817 | 問題なし（実装は凝集度高い） | 例外記録 |
| `New.elm` | 1115 | 1115/0 | TEA パターンの典型的ページ | 例外記録（要検討） |
| `Main.elm` | 832 | 832/0 | SPA エントリポイント | 例外記録 |

structural-review.md の閾値基準を「行数超過 → 分割」から「行数超過 → 責務分析 → 判断」に改善する。

### core_service.rs の問題

`CoreServiceClient` トレイトが 20 メソッドを持つ巨大インターフェース（ISP 違反）。これにより:

1. ファイル分割の制約: Rust では `impl Trait for Type` ブロックを分割不可能（614 行が 1 ブロック）
2. テストスタブの肥大化: auth テストで 17 個の `unimplemented!()` メソッド（約 350 行のボイラープレート）

### 解決策

`CoreServiceClient` を User/Workflow/Task の 3 つのサブトレイトに分割し、スーパートレイト + ブランケット impl でまとめる。行数削減は副次効果であり、主目的は ISP 違反の解消。

## 対象と対象外

対象:
- `core_service.rs` のサブトレイト分割
- `AuthState` の型を `Arc<dyn CoreServiceUserClient>` に変更
- auth テスト/統合テストのスタブ簡素化
- ADR-040 の作成

対象外:
- `WorkflowState` の型変更（全メソッドを使用するため不要）
- dashboard/task/user ハンドラの変更（WorkflowState 経由で影響なし）
- 他の 500+ 行ファイルの分割（例外記録で対応）

## 分割後のファイル構造

```
backend/apps/bff/src/client/
├── core_service.rs              # 親モジュール (~50行)
│                                # mod 宣言 + pub use re-export
├── core_service/
│   ├── error.rs                 # CoreServiceError + From impl (~50行)
│   ├── types.rs                 # 全 DTO/Request/Response 型 (~170行)
│   ├── client_impl.rs           # CoreServiceClientImpl 構造体 + new()
│   │                            # CoreServiceClient スーパートレイト + ブランケット impl (~40行)
│   ├── user_client.rs           # CoreServiceUserClient: trait 定義 + impl (~120行)
│   ├── workflow_client.rs       # CoreServiceWorkflowClient: trait 定義 + impl (~420行)
│   └── task_client.rs           # CoreServiceTaskClient: trait 定義 + impl (~180行)
└── auth_service.rs              # 変更なし
```

全ファイルが 500 行以下。

### メソッドの分類

| サブトレイト | メソッド数 | メソッド |
|------------|----------|---------|
| CoreServiceUserClient | 3 | list_users, get_user_by_email, get_user |
| CoreServiceWorkflowClient | 12 | create/submit_workflow, list/get_definitions, list/get_workflows, approve/reject_step, + display_number 版 4 つ |
| CoreServiceTaskClient | 4 | list_my_tasks, get_task, get_dashboard_stats, get_task_by_display_numbers |

### スーパートレイト設計

```rust
// client_impl.rs
pub trait CoreServiceClient:
    CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient
{}

impl<T> CoreServiceClient for T
where
    T: CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient,
{}
```

`dyn CoreServiceClient` は引き続き使用可能。スーパートレイトの全メソッドが vtable に含まれる。

### 消費者への影響

| ファイル | 変更内容 |
|---------|---------|
| `handler/auth.rs` | `AuthState.core_service_client: Arc<dyn CoreServiceClient>` → `Arc<dyn CoreServiceUserClient>` |
| `handler/workflow.rs` | 変更なし（`Arc<dyn CoreServiceClient>` のまま） |
| `handler/dashboard.rs`, `task.rs`, `user.rs` | 変更なし（WorkflowState 経由） |
| `main.rs` | `Arc<CoreServiceClientImpl>` を具象型で保持し、各 State 注入時に coerce |
| `client.rs` | re-export にサブトレイト名を追加 |
| `tests/auth_integration_test.rs` | `CoreServiceClient` → `CoreServiceUserClient` に変更、17 個の `unimplemented!()` 削除 |

## Phase 構成

### Phase 1: 共有型の切り出し

`error.rs` と `types.rs` を作成し、`core_service.rs` から型定義を移動。親モジュール化して re-export。

#### 確認事項
- パターン: `domain/workflow.rs` の re-export パターン → `backend/crates/domain/src/workflow.rs`
- パターン: `client.rs` の既存 re-export → `backend/apps/bff/src/client.rs`

#### テストリスト
- [ ] `just check` が通る
- [ ] 消費者のインポートパスが変わっていない

### Phase 2: サブトレイト定義 + スーパートレイト化 + impl 分割

3 つのサブトレイトファイルを作成。`CoreServiceClient` をスーパートレイトに変更。元の `impl CoreServiceClient for CoreServiceClientImpl` を 3 つのサブトレイト impl に分割。テストスタブも同様に 3 つの impl に一時的に分割。

注意: Phase 2 ではテストスタブの `unimplemented!()` は残る（Phase 3 で除去）。`CoreServiceClient` をスーパートレイトに変更した瞬間、既存スタブは全サブトレイトの実装が必要になるため。

#### 確認事項
- 型: `CoreServiceClientImpl` のフィールド visibility → `pub(super)` に変更してサブモジュールからアクセス可能にする
- ライブラリ: `async_trait` のサブトレイト適用パターン → Grep `#[async_trait]` in bff
- 技術的前提: ブランケット impl と `dyn CoreServiceClient` の object safety

#### テストリスト
- [ ] `just check` が通る
- [ ] `core_service.rs` が親モジュール（~50 行）のみになっている
- [ ] `dyn CoreServiceClient` がコンパイルで object-safe と確認

### Phase 3: 消費者の型変更 + テストスタブ簡素化

`AuthState.core_service_client` を `Arc<dyn CoreServiceUserClient>` に変更。`main.rs` で具象型保持 + coerce パターンを適用。auth テストスタブを `CoreServiceUserClient` のみ実装に簡素化。

#### 確認事項
- 型: `AuthState` の使用箇所 → `handler/auth.rs`、`main.rs`、`tests/auth_integration_test.rs`
- パターン: `Arc<T> as Arc<dyn Trait>` の unsizing coerce → Rust 標準の coercion

#### テストリスト
- [ ] `just check-all` が通る（統合テスト含む）
- [ ] auth テスト: `StubCoreServiceClient` が `CoreServiceUserClient` の 3 メソッドのみ実装
- [ ] 統合テスト: 同上
- [ ] ボイラープレート削減: auth.rs ~324 行、統合テスト ~148 行

### Phase 4: ADR-040 + structural-review.md 改善 + #290 クローズ準備

1. ADR-040 作成: サブトレイト分割パターンを記録
2. structural-review.md 更新: 閾値基準を「行数超過 → 責務分析 → 判断」に改善
3. Issue #290 のチェックボックス更新と残り対象ファイルの例外記録

#### 確認事項
- ドキュメント: `.claude/rules/structural-review.md` の現在の記述 → Read で確認

#### テストリスト
- [ ] `just check-all` が通る

## 変更対象ファイル

| ファイル | Phase | 変更内容 |
|---------|-------|---------|
| `backend/apps/bff/src/client/core_service.rs` | 1,2 | 1212行 → 親モジュール ~50行 |
| `backend/apps/bff/src/client/core_service/error.rs` | 1 | 新規 ~50行 |
| `backend/apps/bff/src/client/core_service/types.rs` | 1 | 新規 ~170行 |
| `backend/apps/bff/src/client/core_service/client_impl.rs` | 2 | 新規 ~40行 |
| `backend/apps/bff/src/client/core_service/user_client.rs` | 2 | 新規 ~120行 |
| `backend/apps/bff/src/client/core_service/workflow_client.rs` | 2 | 新規 ~420行 |
| `backend/apps/bff/src/client/core_service/task_client.rs` | 2 | 新規 ~180行 |
| `backend/apps/bff/src/client.rs` | 2 | re-export 更新 |
| `backend/apps/bff/src/handler/auth.rs` | 3 | AuthState 型変更 + スタブ簡素化 |
| `backend/apps/bff/src/main.rs` | 3 | クライアント生成パターン変更 |
| `backend/apps/bff/tests/auth_integration_test.rs` | 3 | スタブ簡素化 |
| `docs/05_ADR/040_*.md` | 4 | 新規 ADR |
| `.claude/rules/structural-review.md` | 4 | 閾値基準の改善 |

## 検証

```bash
just check       # Phase 1, 2 の各ステップ後
just check-all   # Phase 3 完了後（統合テスト含む）
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1 回目 | `Arc<dyn CoreServiceClient>` → `Arc<dyn CoreServiceUserClient>` の直接キャストは Rust で不可能 | 技術的前提 | main.rs で具象型 `Arc<CoreServiceClientImpl>` を保持し、State 注入時に coerce |
| 2 回目 | `CoreServiceClientImpl` のフィールド (`base_url`, `client`) がサブモジュールから非公開 | アーキテクチャ不整合 | `client_impl.rs` に構造体を配置し、フィールドを `pub(super)` に変更 |
| 3 回目 | Phase 2 でスーパートレイト化すると既存テストスタブが全サブトレイト実装を要求される | 競合・エッジケース | Phase 2 でスタブも 3 つの impl に一時分割。簡素化は Phase 3 で実施 |
| 4 回目 | DTO 型をサブトレイト別に分散 vs 集約の判断 | シンプルさ | 共有 DTO（WorkflowInstanceDto 等）が複数サブトレイトで使われるため types.rs に集約 |
| 5 回目 | 500 行閾値が残りファイルに対してプロキシメトリクスとして機能していない | 既存手段の見落とし | 閾値を「検討トリガー」に留め、判断基準を「責務分析」に変更。task.rs/New.elm/Main.elm は例外記録で対応 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | core_service.rs の全セクション（エラー型, DTO, トレイト, 構造体, impl, テスト）を配分済み。消費者 6 箇所の変更を計画 |
| 2 | 曖昧さ排除 | OK | 各ファイルの内容、スーパートレイト設計、coerce パターンを具体的コードで記載 |
| 3 | 設計判断の完結性 | OK | types.rs 集約、フィールド visibility、テストスタブの段階的移行を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象（core_service 分割 + AuthState 変更）と対象外（WorkflowState、他ハンドラ）を明記 |
| 5 | 技術的前提 | OK | object safety、unsizing coercion、dyn-to-dyn キャスト不可の制約を Phase 設計に反映 |
| 6 | 既存ドキュメント整合 | OK | ADR-039 のパターン、structural-review.md の閾値、rust.md の mod.rs 禁止と整合 |
