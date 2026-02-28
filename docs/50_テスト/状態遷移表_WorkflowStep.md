# 状態遷移表: WorkflowStep

## 概要

WorkflowStep の全状態 × 全操作のマトリクス。各セルに遷移先（成功時）またはエラー（Error）を記載する。

`activated()` のみ `Result` ではなく `Self` を返す（Pending からのみ呼ばれることが呼び出し元で保証される設計）。それ以外の操作は `Result<Self, DomainError>` を返す。

ソースコード: `backend/crates/domain/src/workflow/step.rs`

## 状態一覧（4 状態）

| 状態 | 説明 | 型 |
|------|------|-----|
| Pending | 待機中 | `WorkflowStepState::Pending` |
| Active | アクティブ（処理中） | `Active(ActiveStepState)` |
| Completed | 完了 | `Completed(CompletedStepState)` |
| Skipped | スキップ | `WorkflowStepState::Skipped` |

## 操作一覧（6 操作）

| # | メソッド | 説明 | 戻り値 |
|---|---------|------|--------|
| 1 | `activated()` | アクティブ化 | `Self`（エラーなし） |
| 2 | `approve()` | 承認 | `Result<Self, DomainError>` |
| 3 | `reject()` | 却下 | `Result<Self, DomainError>` |
| 4 | `request_changes()` | 差し戻し | `Result<Self, DomainError>` |
| 5 | `completed()` | カスタム判断で完了 | `Result<Self, DomainError>` |
| 6 | `skipped()` | スキップ | `Result<Self, DomainError>` |

## 状態遷移マトリクス

凡例:
- `→ State`: 成功。遷移先の状態
- `Error`: `DomainError::Validation` を返す
- `(*)`: ガードなし（呼び出し元で制御）

| 現在の状態 ↓ \ 操作 → | activated | approve | reject | request_changes | completed | skipped |
|---|---|---|---|---|---|---|
| **Pending** | → Active (*) | Error | Error | Error | Error | → Skipped |
| **Active** | → Active (*) | → Completed(Approved) | → Completed(Rejected) | → Completed(RequestChanges) | → Completed(custom) | Error |
| **Completed** | → Active (*) | Error | Error | Error | Error | Error |
| **Skipped** | → Active (*) | Error | Error | Error | Error | Error |

注: `activated()` は `Result` を返さないため、どの状態から呼んでも成功する。呼び出し元のユースケース層で Pending 状態のステップのみに対して呼ぶことが保証されている。

## ユニットテストカバレッジ

### 正常遷移（成功セル: 6 セル）

| 遷移 | テスト名 | カバー |
|------|---------|--------|
| Pending → Active | `test_アクティブ化後の状態` | ✅ |
| Active → Completed(Approved) | `test_承認後の状態`、`test_コメント付き承認後の状態` | ✅ |
| Active → Completed(Rejected) | `test_却下後の状態` | ✅ |
| Active → Completed(RequestChanges) | `test_差戻し後の状態`、`test_差し戻しステップの状態`、`test_コメント付き差し戻しステップの状態` | ✅ |
| Active → Completed(custom) | `completed()` を使用するテスト | ✅ |
| Pending → Skipped | `test_スキップ_待機中から成功` | ✅ |

### 異常遷移（Error セル: 15 セル）

`activated()` はガードなしのため Error セルは approve/reject/request_changes/completed/skipped の 5 操作 × (対象外の状態) で構成される。

| 現在の状態 | 操作 | テスト名 | カバー |
|-----------|------|---------|--------|
| Pending | `approve()` | `test_アクティブ以外で承認するとエラー` (rstest) | ✅ |
| Completed | `approve()` | 同上 | ✅ |
| Skipped | `approve()` | 同上 | ✅ |
| Pending | `reject()` | `test_アクティブ以外で却下するとエラー` (rstest) | ✅ |
| Completed | `reject()` | 同上 | ✅ |
| Skipped | `reject()` | 同上 | ✅ |
| Pending | `request_changes()` | `test_アクティブ以外で差し戻しするとエラー` (rstest) | ✅ |
| Completed | `request_changes()` | 同上 | ✅ |
| Skipped | `request_changes()` | 同上 | ✅ |
| Pending | `completed()` | — | ❌ |
| Completed | `completed()` | — | ❌ |
| Skipped | `completed()` | — | ❌ |
| Active | `skipped()` | `test_スキップ_待機中以外ではエラー` (rstest) | ✅ |
| Completed | `skipped()` | 同上 | ✅ |
| Skipped | `skipped()` | 同上 | ✅ |

### カバレッジサマリー

| カテゴリ | 総数 | テスト済み | カバー率 |
|---------|------|-----------|---------|
| 正常遷移 | 6 | 6 | 100% |
| 異常遷移 | 15 | 12 | 80% |
| **合計** | **21** | **18** | **86%** |

注: `activated()` の呼び出し（4 セル）はガードなしのため Error セルに含めない。呼び出し元のユースケース層でのテストでカバーされる。`completed()` の Error セルは `approve()` 等と同一のガードロジック（Active 以外はエラー）だが、専用テストは未作成。

## 不変条件

→ 詳細: [エンティティ影響マップ（WorkflowStep）](../40_詳細設計書/エンティティ影響マップ/WorkflowStep.md)

| ID | 不変条件 | 型で強制 |
|----|---------|---------|
| INV-S1 | 同一 Instance 内で Active なステップは最大 1 つ | ❌（ユースケース層で保証） |
| INV-S2 | Completed ⇒ decision IS NOT NULL | ✅ `CompletedStepState.decision` |
| INV-S3 | Completed ⇒ completed_at IS NOT NULL | ✅ `CompletedStepState.completed_at` |
| INV-S4 | Active ⇒ started_at IS NOT NULL | ✅ `ActiveStepState.started_at` |

## 関連ドキュメント

- [テスト戦略: エッジケース方針](テスト戦略_エッジケース方針.md)
- [状態遷移表（WorkflowInstance）](状態遷移表_WorkflowInstance.md)
- [ADR-054: 型安全ステートマシンパターンの標準化](../70_ADR/054_型安全ステートマシンパターンの標準化.md)
- [エンティティ影響マップ（WorkflowStep）](../40_詳細設計書/エンティティ影響マップ/WorkflowStep.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-27 | 初版作成（#939） |
