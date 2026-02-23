# #820 WorkflowStep ADT ベースステートマシンリファクタリング

## Context

ADR-054（ADT ベースステートマシンパターンの標準化）に基づき、`WorkflowStep` のドメインモデルを型安全にリファクタリングする。

現在の `WorkflowStep` は 4 状態（Pending / Active / Completed / Skipped）をフラットな `Option` フィールドで表現しており、不正な状態（例: Pending で `decision` が `Some`、Active で `started_at` が `None`）が型レベルで許容されている。エンティティ影響マップの不変条件（INV-S2〜S4）は実行時チェックでのみ担保されている。

本リファクタリングにより、不変条件を型レベルで強制する。API 契約（JSON レスポンス構造）は変更しない。

## 対象・対象外

対象:
- `backend/crates/domain/src/workflow/step.rs` — ADT 型定義 + WorkflowStep 構造体リファクタリング
- `backend/crates/infra/src/repository/workflow_step_repository.rs` — `TryFrom` の `from_db()` 呼び出し修正
- `backend/crates/infra/tests/common/mod.rs` — 不変条件チェックヘルパー（ADT により自動担保されるため簡素化の可能性）
- ドメインユニットテスト（step.rs 内）

対象外:
- `WorkflowStepStatus` enum — 維持（`status()` getter で state から導出）
- `StepDecision` enum — 変更なし
- `NewWorkflowStep` — 変更なし
- `WorkflowStepRecord` — flat 構造を維持（DB スキーマの表現）
- UseCase / Handler / BFF / Frontend — getter API 維持により変更不要
- `assigned_to` の Active/Completed での必須化 — 別 Issue で検討（本 Issue のスコープ外）

## 設計

### ADT 構造（ADR-054 Pattern A: 外側共通 + 状態 enum）

```rust
pub struct WorkflowStep {
    // 共通フィールド（全状態で存在）
    id: WorkflowStepId,
    instance_id: WorkflowInstanceId,
    display_number: DisplayNumber,
    step_id: String,
    step_name: String,
    step_type: String,
    version: Version,
    assigned_to: Option<UserId>,
    due_date: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    // 状態固有フィールド
    state: WorkflowStepState,
}

/// ワークフローステップの状態
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowStepState {
    /// 待機中
    Pending,
    /// アクティブ（処理中）
    Active(ActiveStepState),
    /// 完了
    Completed(CompletedStepState),
    /// スキップ
    Skipped,
}

/// Active 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveStepState {
    /// 開始日時（INV-S4 を型で強制: Not Option）
    pub started_at: DateTime<Utc>,
}

/// Completed 状態の固有フィールド
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedStepState {
    /// 判断（INV-S2 を型で強制: Not Option）
    pub decision: StepDecision,
    /// コメント
    pub comment: Option<String>,
    /// 開始日時（Active から引き継ぎ）
    pub started_at: DateTime<Utc>,
    /// 完了日時（INV-S3 を型で強制: Not Option）
    pub completed_at: DateTime<Utc>,
}
```

### Getter API（後方互換維持）

既存の getter の戻り値型を維持する。呼び出し元（UseCase/Handler/Repository）は変更不要。

```rust
pub fn status(&self) -> WorkflowStepStatus {
    match &self.state {
        WorkflowStepState::Pending => WorkflowStepStatus::Pending,
        WorkflowStepState::Active(_) => WorkflowStepStatus::Active,
        WorkflowStepState::Completed(_) => WorkflowStepStatus::Completed,
        WorkflowStepState::Skipped => WorkflowStepStatus::Skipped,
    }
}

pub fn decision(&self) -> Option<StepDecision> {
    match &self.state {
        WorkflowStepState::Completed(c) => Some(c.decision),
        _ => None,
    }
}

pub fn started_at(&self) -> Option<DateTime<Utc>> {
    match &self.state {
        WorkflowStepState::Active(a) => Some(a.started_at),
        WorkflowStepState::Completed(c) => Some(c.started_at),
        _ => None,
    }
}

pub fn completed_at(&self) -> Option<DateTime<Utc>> {
    match &self.state {
        WorkflowStepState::Completed(c) => Some(c.completed_at),
        _ => None,
    }
}

pub fn comment(&self) -> Option<&str> {
    match &self.state {
        WorkflowStepState::Completed(c) => c.comment.as_deref(),
        _ => None,
    }
}

// 新規: 状態への直接アクセス（パターンマッチ用）
pub fn state(&self) -> &WorkflowStepState { &self.state }
```

### from_db() の Result 化

```rust
pub fn from_db(record: WorkflowStepRecord) -> Result<Self, DomainError> {
    let state = match record.status {
        WorkflowStepStatus::Pending => WorkflowStepState::Pending,
        WorkflowStepStatus::Active => {
            let started_at = record.started_at.ok_or_else(|| {
                DomainError::Validation("Active ステップには started_at が必要です".to_string())
            })?;
            WorkflowStepState::Active(ActiveStepState { started_at })
        }
        WorkflowStepStatus::Completed => {
            let decision = record.decision.ok_or_else(|| {
                DomainError::Validation("Completed ステップには decision が必要です".to_string())
            })?;
            let started_at = record.started_at.ok_or_else(|| {
                DomainError::Validation("Completed ステップには started_at が必要です".to_string())
            })?;
            let completed_at = record.completed_at.ok_or_else(|| {
                DomainError::Validation("Completed ステップには completed_at が必要です".to_string())
            })?;
            WorkflowStepState::Completed(CompletedStepState {
                decision,
                comment: record.comment,
                started_at,
                completed_at,
            })
        }
        WorkflowStepStatus::Skipped => WorkflowStepState::Skipped,
    };

    Ok(Self {
        id: record.id,
        instance_id: record.instance_id,
        // ... 共通フィールド ...
        state,
    })
}
```

### 状態遷移メソッド

`approve()` の例（`reject()`, `request_changes()`, `completed()` も同パターン）:

```rust
pub fn approve(self, comment: Option<String>, now: DateTime<Utc>) -> Result<Self, DomainError> {
    match self.state {
        WorkflowStepState::Active(active) => Ok(Self {
            state: WorkflowStepState::Completed(CompletedStepState {
                decision: StepDecision::Approved,
                comment,
                started_at: active.started_at,  // Active から引き継ぎ
                completed_at: now,
            }),
            version: self.version.next(),
            updated_at: now,
            ..self
        }),
        _ => Err(DomainError::Validation(format!(
            "承認はアクティブ状態でのみ可能です（現在: {}）",
            self.status()
        ))),
    }
}
```

`activated()` — 現在の無検証を維持:

```rust
pub fn activated(self, now: DateTime<Utc>) -> Self {
    Self {
        state: WorkflowStepState::Active(ActiveStepState { started_at: now }),
        updated_at: now,
        ..self
    }
}
```

`skipped()` — Pending のみ:

```rust
pub fn skipped(self, now: DateTime<Utc>) -> Result<Self, DomainError> {
    match self.state {
        WorkflowStepState::Pending => Ok(Self {
            state: WorkflowStepState::Skipped,
            updated_at: now,
            ..self
        }),
        _ => Err(DomainError::Validation(format!(
            "スキップは待機中状態でのみ可能です（現在: {}）",
            self.status()
        ))),
    }
}
```

### is_overdue() — getter 委譲

```rust
pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
    if let Some(due) = self.due_date
        && self.completed_at().is_none()
    {
        return now > due;
    }
    false
}
```

### Repository 影響

`TryFrom<WorkflowStepRow>` の変更:

```rust
// Before:
Ok(WorkflowStep::from_db(WorkflowStepRecord { ... }))

// After:
WorkflowStep::from_db(WorkflowStepRecord { ... })
    .map_err(|e| InfraError::Unexpected(e.to_string()))
```

INSERT / UPDATE の SQL クエリは getter メソッド経由でフィールドを取得しているため変更不要。

## 実装計画

### Phase 1: Domain model ADT リファクタリング

対象: `backend/crates/domain/src/workflow/step.rs`

#### 確認事項
- 型: `WorkflowStepState`, `ActiveStepState`, `CompletedStepState` → 新規定義（上記設計に従う）
- パターン: ADR-054 Pattern A → `docs/05_ADR/054_ADTベースステートマシンパターンの標準化.md`
- パターン: 既存の `WorkflowStep::new()`, `from_db()`, getter、遷移メソッド → `step.rs` 行 141-404
- パターン: テストの `record_from()` ヘルパー → `step.rs` 行 440-459

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | Pending ステップを作成 → 初期状態を確認 | 正常系 | ユニット |
| 2 | Pending → Active 遷移 | 正常系 | ユニット |
| 3 | Active → Completed（承認/却下/差戻し） | 正常系 | ユニット |
| 4 | Active → Completed（コメント付き） | 正常系 | ユニット |
| 5 | Pending → Skipped 遷移 | 正常系 | ユニット |
| 6 | Pending → 承認（不正遷移） | 準正常系 | ユニット |
| 7 | Active → スキップ（不正遷移） | 準正常系 | ユニット |
| 8 | DB 復元: Completed + decision 欠損 | 準正常系 | ユニット |
| 9 | DB 復元: Active + started_at 欠損 | 準正常系 | ユニット |
| 10 | DB 復元: Completed + completed_at 欠損 | 準正常系 | ユニット |
| 11 | 期限切れチェック | 正常系 | ユニット |

#### テストリスト

ユニットテスト:
- [ ] 新規作成の初期状態（state が Pending）— 既存テスト更新
- [ ] アクティブ化後の状態（state が Active、started_at が設定）— 既存テスト更新
- [ ] 承認後の状態（state が Completed、decision が Approved）— 既存テスト更新
- [ ] コメント付き承認後の状態 — 既存テスト更新
- [ ] 却下後の状態（decision が Rejected）— 既存テスト更新
- [ ] 差戻し後の状態（completed() メソッド経由）— 既存テスト更新
- [ ] 差し戻しステップの状態（request_changes() メソッド経由）— 既存テスト更新
- [ ] コメント付き差し戻し — 既存テスト更新
- [ ] スキップ成功（Pending → Skipped）— 既存テスト更新
- [ ] スキップ失敗（Active からはエラー）— 既存テスト更新
- [ ] アクティブ以外で承認エラー — 既存テスト更新
- [ ] アクティブ以外で却下エラー — 既存テスト更新
- [ ] アクティブ以外で差し戻しエラー — 既存テスト更新
- [ ] 期限切れ判定（true / false）— 既存テスト更新
- [ ] from_db: Completed で decision 欠損 → DomainError — 新規
- [ ] from_db: Active で started_at 欠損 → DomainError — 新規
- [ ] from_db: Completed で completed_at 欠損 → DomainError — 新規
- [ ] from_db: Completed で started_at 欠損 → DomainError — 新規

ハンドラテスト（該当なし）
API テスト（該当なし — getter API 維持により既存テストで自動カバー）
E2E テスト（該当なし — API レスポンス形式不変）

### Phase 2: Repository・依存クレートの修正

対象:
- `backend/crates/infra/src/repository/workflow_step_repository.rs`
- `backend/crates/infra/tests/common/mod.rs`
- `backend/crates/infra/tests/workflow_step_repository_test.rs`

#### 確認事項
- パターン: `TryFrom<WorkflowStepRow>` の `from_db()` 呼び出し → `workflow_step_repository.rs` 行 110-142
- パターン: 不変条件チェックヘルパー `assert_step_invariants` → `common/mod.rs` 行 294-320
- パターン: Repository テストの `from_db()` 使用有無 → `workflow_step_repository_test.rs`

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | DB からステップを読み取り → ドメインオブジェクト復元 | 正常系 | ユニット（TryFrom） |
| 2 | ステップを DB に書き込み → 読み取り確認 | 正常系 | Repository 統合テスト |

#### テストリスト

ユニットテスト（該当なし — TryFrom のロジックは既存統合テストでカバー）

ハンドラテスト（該当なし）

API テスト:
- [ ] 既存の API テスト（ステップ承認/却下/差戻し）がパスすること — 既存テスト

E2E テスト（該当なし）

Repository 統合テスト:
- [ ] 既存の Repository テスト全件がパスすること — 既存テスト

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `assigned_to` を Active/Completed で必須にすると `activated()` の署名変更が大きい | 既存手段の見落とし | スコープ外とし、共通フィールドに残す。Issue の記載（Active/Completed では必須）は理想状態であり、別 Issue で対応 |
| 2回目 | `from_db()` を Result 化すると既存テストの `from_db()` 呼び出しに `.unwrap()` が必要 | 不完全なパス | Phase 1 のテスト更新で対応。`record_from()` ヘルパーは getter 経由なので変更不要 |
| 3回目 | `is_overdue()` が `self.completed_at.is_none()` を直接参照 | 未定義 | getter `self.completed_at()` に委譲して対応 |
| 4回目 | `from_db` の Completed で `started_at` 欠損のケースがテストリストに不足 | テスト層網羅漏れ | Phase 1 テストリストに追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | step.rs、repository、テストの 3 領域を Phase で網羅。UseCase/Handler は getter 維持により対象外（探索で確認済み） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | ADT 構造、getter シグネチャ、from_db エラーハンドリングをコードスニペットで明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | assigned_to のスコープ外判断、activated() の無検証維持、from_db の Result 化を明記 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 「対象・対象外」セクションで明示 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `..self` 構造体更新構文が state フィールド変更後も動作することを確認。共通フィールドはそのまま引き継がれる |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-054 Pattern A に準拠。エンティティ影響マップの INV-S2〜S4 を型で強制 |

## 検証方法

```bash
# Phase 1 完了後: domain クレートのテスト
cargo test -p ringiflow-domain -- workflow::step

# Phase 2 完了後: 全テスト
just check-all
```
