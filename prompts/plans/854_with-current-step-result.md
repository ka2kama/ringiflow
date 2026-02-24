# #854 with_current_step を Result 返却に変更

## Context

`WorkflowInstance::with_current_step` は ADT ステートマシンリファクタリング（#819）で意図的に残された技術的負債。他の全遷移メソッド（`submitted`, `advance_to_next_step`, `complete_with_approval` 等）は `Result<Self, DomainError>` を返して不正な状態遷移を防いでいるが、`with_current_step` のみ `Self` を返し、`_ =>` ワイルドカードで全状態からの遷移を許容している。

## 対象・対象外

対象:
- `with_current_step` メソッドの戻り値変更と `_ =>` 削除
- 全呼び出し元の更新（本番コード・テストコード・テストビルダー）
- 不正遷移のエラーテスト追加

対象外:
- ドキュメント（実装解説等）の `with_current_step` 記述の更新（戻り値型の記載変更は機械的で、本 Issue のスコープに含めると作業が散漫になる。必要であれば別途対応）

## Phase 1: メソッド変更 + テスト追加（TDD）

### 確認事項

- 型: `DomainError::Validation` — `backend/crates/domain/src/lib.rs` or `error.rs`
- パターン: 他の遷移メソッドのエラーメッセージ形式 → `instance.rs` の `advance_to_next_step`, `submitted` 等
  - 形式: `"〇〇は××状態でのみ可能です（現在: {}）"` + `self.status()`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 申請 → Pending → with_current_step で InProgress に遷移 | 正常系 | ユニット（既存テストでカバー済み） |
| 2 | Draft 状態から with_current_step を呼び出す → エラー | 異常系 | ユニット（新規） |

### テストリスト

ユニットテスト:
- [ ] `test_下書きからのステップ設定はエラー` — Draft 状態で `with_current_step` を呼ぶと `DomainError` が返る
- [ ] 既存テスト全通過（`.unwrap()` 追加で対応）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装手順

#### 1. Red: エラーケーステストを追加

`instance.rs` のテストモジュールに追加:

```rust
#[rstest]
fn test_下書きからのステップ設定はエラー(
    test_instance: WorkflowInstance,
    now: DateTime<Utc>,
) {
    // Draft 状態からはステップ設定不可
    let result = test_instance.with_current_step("step_1".to_string(), now);

    assert!(result.is_err());
}
```

→ コンパイルは通るが `Self` が返るため `is_err()` が使えない → Red (compile)

#### 2. Green: メソッドを Result 返却に変更

`backend/crates/domain/src/workflow/instance.rs:587`:

```rust
pub fn with_current_step(self, step_id: String, now: DateTime<Utc>) -> Result<Self, DomainError> {
    match self.state {
        WorkflowInstanceState::Pending(pending) => Ok(Self {
            state: WorkflowInstanceState::InProgress(InProgressState {
                current_step_id: step_id,
                submitted_at:    pending.submitted_at,
            }),
            version: self.version.next(),
            updated_at: now,
            ..self
        }),
        _ => Err(DomainError::Validation(format!(
            "ステップ設定は承認待ち状態でのみ可能です（現在: {}）",
            self.status()
        ))),
    }
}
```

変更点:
- 戻り値: `Self` → `Result<Self, DomainError>`
- Pending アーム: `Self { ... }` → `Ok(Self { ... })`
- `_ =>` アーム: 暫定コードを `Err(DomainError::Validation(...))` に置換
- FIXME コメント削除

#### 3. 呼び出し元の更新

本番コード（1 箇所）:

| ファイル | 行 | 変更 |
|---------|-----|------|
| `backend/apps/core-service/src/usecase/workflow/command/lifecycle/submit.rs:134` | `submitted_instance.with_current_step(first_step_id, now)` | `.map_err(\|e\| CoreError::BadRequest(e.to_string()))?` を追加 |

テストビルダー（1 箇所）:

| ファイル | 行 | 変更 |
|---------|-----|------|
| `backend/apps/core-service/src/test_utils/workflow_test_builder.rs:143` | `.with_current_step(...)` | `.unwrap()` を追加 |

テストコード（機械的変更 — `.unwrap()` 追加）:

| ファイル | 箇所数 |
|---------|--------|
| `backend/crates/domain/src/workflow/instance.rs` | 約 15 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/lifecycle/submit.rs` | 2 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/lifecycle/resubmit.rs` | 5 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/decision/approve.rs` | 4 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/decision/reject.rs` | 4 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/decision/request_changes.rs` | 4 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/comment.rs` | 3 箇所 |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | 1 箇所 |
| `backend/apps/core-service/src/usecase/workflow/query.rs` | 1 箇所 |
| `backend/apps/core-service/src/usecase/dashboard.rs` | 5 箇所 |
| `backend/apps/core-service/src/usecase/task.rs` | 7 箇所 |
| `backend/crates/infra/tests/transaction_concurrency_test.rs` | 3 箇所 |

#### 4. Refactor

設計原則レンズ:
- 意図の明確さ: エラーメッセージが他メソッドと同じ形式か確認
- 重複の排除: 新コードが既存パターンと一致しているか確認

## 検証

```bash
just check-all
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | エラーメッセージの形式が他メソッドと統一されているか未確認だった | 既存パターン整合 | 探索で他メソッドのエラーメッセージ形式を確認し、`"ステップ設定は承認待ち状態でのみ可能です（現在: {}）"` を採用 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | メソッド本体 + 本番呼び出し1箇所 + テストビルダー1箇所 + テスト約50箇所を全て列挙 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | エラーメッセージ文言、変更パターン（`.unwrap()` / `?`）が具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | エラーメッセージ形式を既存パターンに準拠する判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | ドキュメント更新を対象外として明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `Result` の `?` 演算子と `map_err` のパターンは既存コードで確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-054（型安全ステートマシン）の方針に沿った変更 |
