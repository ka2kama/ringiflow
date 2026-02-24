# workflow command usecase のコード重複解消

## 概要

Issue #902 に対応し、`backend/apps/core-service/src/usecase/workflow/command/` 配下のコード重複を解消した。ハイブリッドアプローチ（ヘルパー関数 + フロー統合）で 5 Phase のリファクタリングを実施し、jscpd クローン数を約 200 から 9 へ 95% 以上削減した。

## 実施内容

### Phase 1: Persistence ヘルパー関数の抽出

`command/helpers.rs` を新設し、全コマンドで繰り返されるトランザクション・永続化ボイラープレートを 5 つのヘルパーメソッドに抽出した。

- `begin_tx`, `commit_tx`: トランザクション開始・コミット
- `save_step`, `save_instance`: version check 付き更新
- `fetch_instance_steps`: インスタンスに紐づくステップ一覧取得

submit.rs, resubmit.rs に適用。

### Phase 2: reject/request_changes のフロー統合

`decision/common.rs` を新設。95% 同一だった reject/request_changes のフローを `StepTerminationType` enum と `terminate_step()` メソッドに統合した。

変動点（enum match で分岐）:
1. ドメインメソッド: `step.reject()` / `step.request_changes()`
2. インスタンス遷移: `complete_with_rejection()` / `complete_with_request_changes()`
3. 権限チェックアクション名: "却下" / "差し戻し"
4. イベントログ: `STEP_REJECTED` / `STEP_CHANGES_REQUESTED`

### Phase 3: submit/resubmit の共通部分抽出

`lifecycle/common.rs` を新設。submit/resubmit で重複していた承認者検証とステップ作成ループを共通関数に抽出した。

- `validate_approvers()`: 承認者と定義ステップの整合性検証（純関数）
- `create_approval_steps()`: 定義と承認者に基づくステップ作成（最初のステップのみ Active）

### Phase 5: approve.rs へのヘルパー適用

Phase 1 で作成した persistence ヘルパーを approve.rs にも適用し、`InfraError` のインポート除去とボイラープレート置換を実施。

### Phase 4: テストコードの重複解消

`command.rs` の `test_helpers` モジュールに `build_sut()` 関数を追加。5 ファイル・41 箇所の 8 引数 `WorkflowUseCaseImpl::new(...)` パターンを 1 行の `build_sut(&repo, &repo, &repo, now)` に置換した。7 つの未使用インポート（`Arc`, `FixedClock`, 4 Mock 型, `WorkflowUseCaseImpl`）も各テストモジュールから削除。

## 判断ログ

- Phase 2 の設計判断: reject/request_changes の統合に enum dispatch を選択。クロージャやトレイト委譲より Rust として自然で、変動点が 4 箇所と少ないため
- Phase 3 の設計判断: submit/resubmit は前提条件が異なるため、フロー統合ではなく共通部分のみ関数抽出。`validate_approvers()` を純関数として配置（`impl` ブロック外）
- Phase 4 の設計判断: 計画では `WorkflowCommandTestContext` 構造体を検討したが、既存の `WorkflowTestBuilder` が trait object を返す（Mock 固有メソッドに非対応）ため、シンプルな `build_sut` 自由関数を採用
- Phase の実行順序: Phase 5 を Phase 4 より先に実施。Phase 5 が独立した小変更であり、Phase 4（テストコード変更）と分離した方がレビューしやすいため

## 成果物

### コミット

```
16a1027 #902 WIP: Eliminate code duplication in workflow command usecases
ae23920 #902 Extract persistence helpers and apply to submit/resubmit
b8327ee #902 Consolidate reject/request_changes into shared terminate_step flow
4c8120b #902 Extract validate_approvers and create_approval_steps helpers
4c42f2e #902 Apply persistence helpers to approve.rs
ec08e08 #902 Extract build_sut test helper to deduplicate SUT construction
```

### 作成ファイル

- `backend/apps/core-service/src/usecase/workflow/command/helpers.rs`
- `backend/apps/core-service/src/usecase/workflow/command/decision/common.rs`
- `backend/apps/core-service/src/usecase/workflow/command/lifecycle/common.rs`

### 変更ファイル

- `command.rs`: `mod helpers;` 追加、`test_helpers` に `build_sut` 追加
- `decision.rs`: `mod common;` 追加
- `decision/approve.rs`: persistence ヘルパー適用、テスト SUT 置換
- `decision/reject.rs`: `terminate_step` 委譲、テスト SUT 置換
- `decision/request_changes.rs`: `terminate_step` 委譲、テスト SUT 置換
- `lifecycle.rs`: `mod common;` 追加
- `lifecycle/submit.rs`: 共通関数適用、テスト SUT 置換
- `lifecycle/resubmit.rs`: 共通関数適用、テスト SUT 置換

### 品質指標

| 指標 | Before | After |
|------|--------|-------|
| jscpd クローン数 | ~200 | 9 |
| 重複率 | 12.07% | 2.33% |
| テスト数 | 99 | 99（変化なし） |
