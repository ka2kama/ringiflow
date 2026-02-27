# 状態遷移表: WorkflowInstance

## 概要

WorkflowInstance の全状態 × 全操作のマトリクス。各セルに遷移先（成功時）またはエラー（Error）を記載する。

型安全ステートマシン（[ADR-054](../05_ADR/054_型安全ステートマシンパターンの標準化.md)）により、無効な遷移は `Result::Err(DomainError::Validation)` を返す。型レベルでの防止ではなくランタイムのガードで制御している。

ソースコード: `backend/crates/domain/src/workflow/instance.rs`

## 状態一覧（7 状態）

| 状態 | 説明 | 型 |
|------|------|-----|
| Draft | 下書き | `WorkflowInstanceState::Draft` |
| Pending | 承認待ち（申請済み） | `Pending(PendingState)` |
| InProgress | 処理中 | `InProgress(InProgressState)` |
| Approved | 承認完了 | `Approved(CompletedState)` |
| Rejected | 却下 | `Rejected(CompletedState)` |
| Cancelled | 取り消し | `Cancelled(CancelledState)` |
| ChangesRequested | 要修正（差し戻し） | `ChangesRequested(ChangesRequestedState)` |

## 操作一覧（8 操作）

| # | メソッド | 説明 |
|---|---------|------|
| 1 | `submitted()` | 申請 |
| 2 | `with_current_step()` | 初回ステップ設定 |
| 3 | `advance_to_next_step()` | 次ステップ遷移 |
| 4 | `complete_with_approval()` | 承認完了 |
| 5 | `complete_with_rejection()` | 却下完了 |
| 6 | `complete_with_request_changes()` | 差し戻し |
| 7 | `resubmitted()` | 再申請 |
| 8 | `cancelled()` | 取り消し |

## 状態遷移マトリクス

凡例:
- `→ State`: 成功。遷移先の状態
- `Error`: `DomainError::Validation` を返す

| 現在の状態 ↓ \ 操作 → | submitted | with_current_step | advance_to_next_step | complete_with_approval | complete_with_rejection | complete_with_request_changes | resubmitted | cancelled |
|---|---|---|---|---|---|---|---|---|
| **Draft** | → Pending | Error | Error | Error | Error | Error | Error | → Cancelled(FromDraft) |
| **Pending** | Error | → InProgress | Error | Error | Error | Error | Error | → Cancelled(FromPending) |
| **InProgress** | Error | Error | → InProgress(next) | → Approved | → Rejected | → ChangesRequested | Error | → Cancelled(FromActive) |
| **Approved** | Error | Error | Error | Error | Error | Error | Error | Error |
| **Rejected** | Error | Error | Error | Error | Error | Error | Error | Error |
| **Cancelled** | Error | Error | Error | Error | Error | Error | Error | Error |
| **ChangesRequested** | Error | Error | Error | Error | Error | Error | → InProgress | → Cancelled(FromActive) |

## ユニットテストカバレッジ

### 正常遷移（成功セル: 10 セル）

| 遷移 | テスト名 | カバー |
|------|---------|--------|
| Draft → Pending | `test_申請後の状態` | ✅ |
| Pending → InProgress | `test_申請後の状態` 内で検証 | ✅ |
| InProgress → InProgress(next) | `test_次ステップ遷移_処理中で成功` | ✅ |
| InProgress → Approved | `test_承認完了後の状態` | ✅ |
| InProgress → Rejected | `test_却下完了後の状態` | ✅ |
| InProgress → ChangesRequested | `test_差し戻し完了後の状態` | ✅ |
| ChangesRequested → InProgress | `test_再申請後の状態` | ✅ |
| Draft → Cancelled(FromDraft) | `test_下書きからの取消後の状態` | ✅ |
| Pending → Cancelled(FromPending) | `test_申請済みからの取消後の状態` | ✅ |
| InProgress → Cancelled(FromActive) | `test_処理中からの取消後の状態` | ✅ |
| ChangesRequested → Cancelled(FromActive) | `test_要修正状態からの取消後の状態` | ✅ |

### 異常遷移（Error セル: 46 セル）

テスト済みの Error セルを以下に示す。テスト名のない行は未テスト。

| 現在の状態 | 操作 | テスト名 | カバー |
|-----------|------|---------|--------|
| Pending | `submitted()` | — | ❌ |
| InProgress | `submitted()` | `test_処理中からの申請はエラー` | ✅ |
| Approved | `submitted()` | — | ❌ |
| Rejected | `submitted()` | — | ❌ |
| Cancelled | `submitted()` | — | ❌ |
| ChangesRequested | `submitted()` | — | ❌ |
| Draft | `with_current_step()` | `test_下書きからのステップ設定はエラー` | ✅ |
| InProgress | `with_current_step()` | — | ❌ |
| Approved | `with_current_step()` | — | ❌ |
| Rejected | `with_current_step()` | — | ❌ |
| Cancelled | `with_current_step()` | — | ❌ |
| ChangesRequested | `with_current_step()` | — | ❌ |
| Draft | `advance_to_next_step()` | `test_次ステップ遷移_処理中以外ではエラー` (rstest) | ✅ |
| Pending | `advance_to_next_step()` | 同上 | ✅ |
| Approved | `advance_to_next_step()` | 同上 | ✅ |
| Rejected | `advance_to_next_step()` | 同上 | ✅ |
| Cancelled | `advance_to_next_step()` | 同上 | ✅ |
| ChangesRequested | `advance_to_next_step()` | 同上 | ✅ |
| Draft | `complete_with_approval()` | `test_処理中以外で承認完了するとエラー` (rstest) | ✅ |
| Pending | `complete_with_approval()` | 同上 | ✅ |
| Approved | `complete_with_approval()` | 同上 | ✅ |
| Rejected | `complete_with_approval()` | 同上 | ✅ |
| Cancelled | `complete_with_approval()` | 同上 | ✅ |
| ChangesRequested | `complete_with_approval()` | 同上 | ✅ |
| Draft | `complete_with_rejection()` | `test_処理中以外で却下完了するとエラー` (rstest) | ✅ |
| Pending | `complete_with_rejection()` | 同上 | ✅ |
| Approved | `complete_with_rejection()` | 同上 | ✅ |
| Rejected | `complete_with_rejection()` | 同上 | ✅ |
| Cancelled | `complete_with_rejection()` | 同上 | ✅ |
| ChangesRequested | `complete_with_rejection()` | 同上 | ✅ |
| Draft | `complete_with_request_changes()` | `test_処理中以外で差し戻しするとエラー` (rstest) | ✅ |
| Pending | `complete_with_request_changes()` | 同上 | ✅ |
| Approved | `complete_with_request_changes()` | 同上 | ✅ |
| Rejected | `complete_with_request_changes()` | 同上 | ✅ |
| Cancelled | `complete_with_request_changes()` | 同上 | ✅ |
| ChangesRequested | `complete_with_request_changes()` | 同上 | ✅ |
| Draft | `resubmitted()` | `test_要修正以外で再申請するとエラー` (rstest) | ✅ |
| Pending | `resubmitted()` | `test_申請済みからの再申請はエラー` | ✅ |
| InProgress | `resubmitted()` | `test_要修正以外で再申請するとエラー` (rstest) | ✅ |
| Approved | `resubmitted()` | 同上 | ✅ |
| Rejected | `resubmitted()` | 同上 | ✅ |
| Cancelled | `resubmitted()` | 同上 | ✅ |
| Approved | `cancelled()` | `test_承認済みからの取消はエラー` | ✅ |
| Rejected | `cancelled()` | `test_却下済みからの取消はエラー` | ✅ |
| Cancelled | `cancelled()` | `test_キャンセル済みからの取消はエラー` | ✅ |

### カバレッジサマリー

| カテゴリ | 総数 | テスト済み | カバー率 |
|---------|------|-----------|---------|
| 正常遷移 | 11 | 11 | 100% |
| 異常遷移 | 45 | 38 | 84% |
| **合計** | **56** | **49** | **88%** |

### 未テストの Error セル（7 セル）

すべて `submitted()` と `with_current_step()` の Error セル。rstest でパラメータ化されていないテストのため、一部の状態がカバーされていない。

| 操作 | 未テストの状態 |
|------|-------------|
| `submitted()` | Pending, Approved, Rejected, Cancelled, ChangesRequested |
| `with_current_step()` | InProgress, Approved, Rejected, Cancelled, ChangesRequested |

→ 後続 Issue でテスト追加を検討。rstest のパラメータ化でカバー可能。

## 不変条件

→ 詳細: [エンティティ影響マップ（WorkflowInstance）](../03_詳細設計書/エンティティ影響マップ/WorkflowInstance.md)

| ID | 不変条件 | 型で強制 |
|----|---------|---------|
| INV-I1 | Approved ⇒ completed_at IS NOT NULL | ✅ `CompletedState.completed_at` |
| INV-I2 | Rejected ⇒ completed_at IS NOT NULL | ✅ `CompletedState.completed_at` |
| INV-I3 | InProgress ⇒ current_step_id IS NOT NULL | ✅ `InProgressState.current_step_id` |
| INV-I4 | Draft ⇒ submitted_at IS NULL | ✅ `Draft` バリアントにフィールドなし |
| INV-I5 | Pending ⇒ submitted_at IS NOT NULL | ✅ `PendingState.submitted_at` |
| INV-I6 | InProgress ⇒ submitted_at IS NOT NULL | ✅ `InProgressState.submitted_at` |
| INV-I7 | Approved/Rejected ⇒ current_step_id, submitted_at IS NOT NULL | ✅ `CompletedState` |
| INV-I8 | ChangesRequested ⇒ current_step_id, submitted_at IS NOT NULL | ✅ `ChangesRequestedState` |
| INV-I9 | Cancelled ⇒ completed_at IS NOT NULL | ✅ `CancelledState.completed_at()` |

## 関連ドキュメント

- [テスト戦略: エッジケース方針](テスト戦略_エッジケース方針.md)
- [状態遷移表（WorkflowStep）](状態遷移表_WorkflowStep.md)
- [ADR-054: 型安全ステートマシンパターンの標準化](../05_ADR/054_型安全ステートマシンパターンの標準化.md)
- [エンティティ影響マップ（WorkflowInstance）](../03_詳細設計書/エンティティ影響マップ/WorkflowInstance.md)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-27 | 初版作成（#939） |
