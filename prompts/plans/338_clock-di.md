# #338 Clock DI 導入計画

## Context

#334（全体比較テスト移行）において、ユースケース層テスト（`task.rs`, `workflow.rs`）は `Utc::now()` で Mock データを構築しており、テスト実行中の時刻が厳密に制御されていないため対象外とした。Clock DI を導入してテストの決定性を確保し、全体比較移行を完了する。

## 設計判断

### Clock trait の配置場所: domain クレート

- Repository trait は `infra` クレートに定義されているが、Clock はインフラストラクチャではない
- 時刻はドメイン概念（状態遷移のタイムスタンプ）であり、domain が適切
- `domain` は既に `chrono` に依存しており、新たな依存は不要
- 将来、ドメインサービスが時刻を必要とする場合（期限切れ判定等）にも対応可能

### DI パターン: `Arc<dyn Clock>`

既存の Repository DI パターン（`Arc<dyn XxxRepository>`）と一致させる。

### 対象ユースケース: WorkflowUseCaseImpl のみ

- `WorkflowUseCaseImpl` — 5箇所の `Utc::now()` 呼び出し（プロダクションコード）
- `TaskUseCaseImpl` — プロダクションコードに `Utc::now()` 呼び出しなし → Clock 不要
- `DashboardUseCaseImpl` — `now` は既にパラメータで受け取っている → Clock 不要

### submit_workflow の Utc::now() 二重呼び出し修正

現状、`submit_workflow()` は lines 242, 263 で `Utc::now()` を2回呼んでいる。単一の論理操作には単一のタイムスタンプが適切。`self.clock.now()` の1回呼び出しに統合する。

## スコープ

**対象:**
- Clock trait + SystemClock + FixedClock の定義（domain クレート）
- WorkflowUseCaseImpl への Clock 注入と `Utc::now()` 置換
- main.rs の初期化更新
- workflow.rs ユースケーステスト: FixedClock 導入 + 正常系テストの全体比較移行（4テスト）
- task.rs ユースケーステスト: 正常系テストの全体比較移行（4テスト）
- ユースケース出力型（`WorkflowWithSteps`, `TaskItem`, `TaskDetail`）への `PartialEq, Eq` 追加

**対象外:**
- infra 層（`session.rs`）の `Utc::now()` 置換
- handler 層（`dashboard.rs` handler）の `Utc::now()` 置換
- dashboard.rs の残り4テストの全体比較移行（既に1テスト移行済み、残りは今回のスコープ外）

## 変更対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/src/clock.rs` | **新規**: Clock trait + SystemClock + FixedClock |
| `backend/crates/domain/src/lib.rs` | `pub mod clock;` 追加 |
| `backend/apps/core-service/src/usecase/workflow.rs` | Clock 注入、`Utc::now()` 置換、テスト移行 |
| `backend/apps/core-service/src/usecase/task.rs` | テスト全体比較移行、出力型に PartialEq 追加 |
| `backend/apps/core-service/src/main.rs` | SystemClock インスタンス生成 + 注入 |

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: Clock trait + 実装（domain クレート）

#### 確認事項
- 型: `DateTime<Utc>` の import → `chrono::{DateTime, Utc}`（domain クレートで既使用）
- パターン: 既存 trait の `Send + Sync` バウンド → `repository/*.rs` の trait 定義を参照

#### テストリスト
- [ ] `SystemClock::now()` は `DateTime<Utc>` を返す
- [ ] `FixedClock::now()` はコンストラクタで渡した時刻を返す
- [ ] `FixedClock::now()` は複数回呼んでも同じ時刻を返す

#### 設計

```rust
// backend/crates/domain/src/clock.rs
use chrono::{DateTime, Utc};

/// 現在時刻を提供するトレイト
///
/// ユースケース層での `Utc::now()` 直接呼び出しを置き換え、
/// テストで固定時刻を注入可能にする。
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// 実際のシステム時刻を返す実装
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// 固定時刻を返すテスト用実装
pub struct FixedClock {
    now: DateTime<Utc>,
}

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}
```

### Phase 2: WorkflowUseCaseImpl への Clock 注入

#### 確認事項
- 型: `WorkflowUseCaseImpl` の構造体定義 → `workflow.rs:90-98`
- パターン: `new()` のシグネチャ → `workflow.rs:102-116`
- パターン: main.rs での初期化 → `main.rs:169-175`

#### テストリスト
- [ ] 既存の workflow テスト12件が FixedClock で全て通る

#### 変更内容

1. `WorkflowUseCaseImpl` に `clock: Arc<dyn Clock>` フィールド追加
2. `new()` に `clock` パラメータ追加
3. 5箇所の `chrono::Utc::now()` を `self.clock.now()` に置換:
   - `create_workflow()` line 162
   - `submit_workflow()` lines 242, 263 → 1回に統合
   - `approve_step()` line 342
   - `reject_step()` line 439
4. main.rs で `Arc::new(SystemClock)` を生成して注入
5. テストのモック構築で `Arc::new(FixedClock::new(now))` を使用

### Phase 3: workflow.rs 正常系テスト全体比較移行

#### 確認事項
- 型: `WorkflowWithSteps` の定義 → `workflow.rs:43-46`（`PartialEq` なし → 追加が必要）
- パターン: 全体比較の書き方 → `domain/src/workflow.rs` のテスト（`from_db()` パターン）
- パターン: `DashboardStats` の全体比較テスト → `dashboard.rs:500-516`

#### テストリスト
- [ ] `test_create_workflow_正常系` — 全体比較に移行
- [ ] `test_approve_step_正常系` — 全体比較に移行
- [ ] `test_reject_step_正常系` — 全体比較に移行
- [ ] `test_submit_workflow_正常系` — 全体比較に移行

#### 変更内容

1. `WorkflowWithSteps` に `#[derive(Debug, PartialEq, Eq)]` 追加
2. 各正常系テストで、フィールド単位のアサーションを `assert_eq!(result, expected)` に置換
3. `expected` は `from_db()` パターンで構築（FixedClock の固定時刻を使用）

注意: エラー系テスト（403, 400, 409）は `matches!` を使用しており、全体比較の対象外。

### Phase 4: task.rs テスト全体比較移行

#### 確認事項
- 型: `TaskItem`, `TaskDetail` の定義 → `task.rs:29-39`（`PartialEq` なし → 追加が必要）
- パターン: task.rs テストのアサーションパターン → `task.rs:562-566`

#### テストリスト
- [ ] `test_list_my_tasks_activeなステップのみ返る` — 全体比較に移行
- [ ] `test_list_my_tasks_workflowタイトルがタスクに含まれる` — 全体比較に移行
- [ ] `test_get_task_正常系` — 全体比較に移行
- [ ] `test_get_task_by_display_numbers_正常系` — 全体比較に移行

#### 変更内容

1. `TaskItem`, `TaskDetail` に `#[derive(Debug, PartialEq, Eq)]` 追加
2. 各正常系テストで、フィールド単位のアサーションを `assert_eq!(result, expected)` に置換
3. task.rs はプロダクションコードに `Utc::now()` 呼び出しがないため、Clock 注入は不要。テストの `now` 変数をそのまま使用して `expected` を構築

注意: エラー系テスト（NotFound, Forbidden）および空リストテストは対象外。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `submit_workflow` で `Utc::now()` が2回呼ばれ微小な時間差が生じている | 不完全なパス | Clock DI 導入時に1回呼び出しに統合することを Phase 2 に明記 |
| 2回目 | `TaskItem`, `TaskDetail`, `WorkflowWithSteps` に `PartialEq` がない | 未定義 | Phase 3, 4 の確認事項と変更内容に derive 追加を明記 |
| 3回目 | task.rs はプロダクションコードに `Utc::now()` がないため Clock 不要 | 既存手段の見落とし | Phase 4 で Clock 注入なし、テストの `now` 変数のみで全体比較移行と明記 |
| 4回目 | dashboard.rs 残り4テストがスコープに含まれるか曖昧 | 曖昧 | スコープの「対象外」に dashboard 残りテストを明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | workflow.rs の 5箇所の `Utc::now()` を全て特定。task.rs はプロダクション呼び出しなしを確認。正常系テスト8件を移行対象として列挙 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更対象ファイル、行番号、derive 追加を具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | Clock 配置場所（domain）、対象ユースケース（Workflow のみ）、DI パターン（Arc<dyn Clock>）に理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（Clock trait, workflow DI, テスト移行）と対象外（infra, handler, dashboard 残りテスト）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | `PartialEq` derive の必要性、`Send + Sync` バウンドの必要性を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | #334 計画ファイル、Issue #338 の完了基準と照合。依存関係 `apps → domain` を確認 |

## 検証方法

```bash
# Phase ごとに実行
cd backend && cargo test --package ringiflow-domain   # Phase 1
cd backend && cargo test --package core-service       # Phase 2-4
just check-all                                        # 全体チェック
```
