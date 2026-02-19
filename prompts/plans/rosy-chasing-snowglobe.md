# Issue #688: 判断系ユースケースのトランザクション化

## Context

判断系ユースケース（approve_step, reject_step, request_changes_step）は、#687 で `TxContext` による構造的強制が導入済みだが、各書き込み操作が**独立したトランザクション**で実行されている。これにより、中間の書き込み成功後に後続の書き込みが失敗した場合、データ不整合が発生するリスクがある（例: ステップは Approved だがインスタンスは InProgress のまま）。

本 Issue では、各ユースケース内の全書き込みを**同一トランザクション**にまとめ、原子性を保証する。

## 方針

**全読み取りをトランザクション開始前に、全書き込みをトランザクション内に集約する。**

```
[読み取り + ドメインロジック] → TX BEGIN → [全書き込み] → TX COMMIT → [結果読み取り]
```

この方針が成立する理由:
- 楽観的ロック（バージョンチェック）が読み取り〜書き込み間の競合を検出する
- 読み取りメソッドは `TxContext` を取らない設計（#687 の ADR-051）
- トランザクション内の書き込みが 1 つでも失敗すれば、`tx` が drop → 自動ロールバック

### 対象外

- 読み取りメソッドのトランザクション対応（リポジトリ trait 変更はスコープ外）
- reject/request_changes 間のコード重複解消（別 Issue で対応）

## Phase 1: approve_step のトランザクション統合

### 対象ファイル

- `backend/apps/core-service/src/usecase/workflow/command/decision/approve.rs`

### 変更内容

現在の処理フロー（3 トランザクション）を 1 トランザクションに統合する。

**Before（3 TX）:**
```
1-7: 読み取り + ドメインロジック
8:   TX1: save approved_step → commit
9:   Read all_steps → find next → TX2: save activated_step → commit
10:  TX3: save updated_instance → commit
11:  Read final steps
```

**After（1 TX）:**
```
1-7:  読み取り + ドメインロジック（変更なし）
8:    Read all_steps → find next step → activate (ドメインロジック)
9:    TX: save approved_step + [save activated_step] + save updated_instance → commit
10:   Read final steps
```

具体的な変更:
- ステップ 9 の `find_by_instance` 読み取りをトランザクション開始前に移動
- `next_step` の activate ドメインロジックもトランザクション前に実行
- 3 つの `begin/commit` ブロックを 1 つに統合
- 各書き込みの `map_err` はそのまま維持（エラーメッセージの区別を保持）

### 確認事項

- [x] 型: `TxContext` は `&mut` 借用で複数の書き込みに順次渡せるか → `workflow_step_repository.rs` L47, `tx: &mut TxContext`
- [x] パターン: 既存の単一 TX 内で複数書き込みを行うパターン → `workflow_instance_repository_test.rs` で `begin → write → commit`
- [x] ライブラリ: `activated()` は `Self` を返す直値。`skipped()` は `Result<Self, DomainError>` → `step.rs` L255, L295

### テストリスト

ユニットテスト:
- 既存テスト全パス（行動変更なし。MockTransactionManager は TX 境界に無関心）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: reject_step のトランザクション統合

### 対象ファイル

- `backend/apps/core-service/src/usecase/workflow/command/decision/reject.rs`

### 変更内容

**Before（N+2 TX）:**
```
1-4:  読み取り + ドメインロジック（reject step）
5:    TX1: save rejected_step → commit
6:    Read all_steps → for each Pending: TX: save skipped → commit
7:    Read instance → complete (domain)
8:    TX_final: save completed_instance → commit
9:    Read final steps
```

**After（1 TX）:**
```
1-4:  読み取り + ドメインロジック（reject step）（変更なし）
5:    Read all_steps → skip pending steps (ドメインロジック)
6:    Read instance → complete_with_rejection (ドメインロジック)
7:    TX: save rejected_step + save all skipped_steps + save completed_instance → commit
8:    Read final steps
```

具体的な変更:
- `find_by_instance` と `find_by_id`（instance）をトランザクション前に移動
- pending ステップの `skipped()` ドメインロジックをトランザクション前に実行し、結果をコレクションに蓄積
- ループ内の個別 TX を排除し、単一 TX 内でループ書き込み

### 確認事項

- [x] 型: `skipped()` は `Result<Self, DomainError>`。`self` を消費する → `step.rs` L295
- [x] パターン: Phase 1 で確立したパターンを踏襲（`&mut tx` を複数の write に順次渡す）

### テストリスト

ユニットテスト:
- 既存テスト全パス（行動変更なし）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 3: request_changes_step のトランザクション統合

### 対象ファイル

- `backend/apps/core-service/src/usecase/workflow/command/decision/request_changes.rs`

### 変更内容

reject_step と同一パターン。差分は状態遷移のみ:
- `reject()` → `request_changes()`
- `complete_with_rejection()` → `complete_with_request_changes()`

### 確認事項

確認事項: なし（Phase 2 と同一パターン）

### テストリスト

ユニットテスト:
- 既存テスト全パス（行動変更なし）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 4: 並行操作の統合テスト

### 対象ファイル

- `backend/crates/infra/tests/transaction_concurrency_test.rs`（新規作成）

### 設計判断

**テスト配置: infra crate のリポジトリ層テスト**

ユースケース層（core-service）でのテストは、core-service に sqlx dev-dependency の追加と、infra の `common/mod.rs` テストヘルパーへのアクセスが必要になり、依存関係が複雑化する。リポジトリ層での並行テストで、トランザクション + 楽観的ロックの正しい動作を検証できる。409 へのマッピングは既存ユニットテストで検証済み。

**テストシナリオ: 2 つの並行トランザクションが同一エンティティを更新**

```
1. セットアップ: Instance(v1), Step(v1) を DB に挿入
2. TX_A: step update (v1→v2) + instance update (v1→v2)
3. TX_B: step update (expects v1, actual v2) → Conflict
4. TX_A: commit → 成功
5. TX_B: 自動 rollback
6. 検証: invariants が保持されていること
```

### 確認事項

- [ ] `sqlx::test` での並行実行パターン → `tokio::spawn` + `Arc` 共有
- [ ] `PgTransactionManager::new` の引数 → `PgPool`
- [ ] `common/mod.rs` の `assert_workflow_invariants` シグネチャ → `(pool, instance_id, tenant_id)`
- [ ] `setup_test_data` の戻り値 → `(TenantId, UserId)`

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

統合テスト:
- [ ] 並行更新で一方が Conflict を返し、もう一方が成功する
- [ ] 並行更新後に不変条件が保持されている

API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | 読み取りをトランザクション前に移動する際、approve の `find_by_instance` が書き込み後に配置されている | 不完全なパス | 読み取りを TX 前に移動。楽観的ロックにより正確性は保証される |
| 2回目 | 統合テストの配置先（core-service vs infra）の判断が必要 | アーキテクチャ不整合 | infra crate に配置。既存テストインフラ（common/mod.rs, sqlx::test）を再利用可能 |
| 3回目 | reject/request_changes の pending ステップ skip ループで、ドメインロジックとトランザクション書き込みの分離方法が未定義 | 曖昧 | ドメインロジックをループで先行実行し `Vec<(skipped_step, version)>` に蓄積、TX 内で一括書き込み |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 3 ユースケース + 統合テスト | OK | approve/reject/request_changes の全 3 ファイルを Phase 1-3 で対応、統合テストを Phase 4 で対応 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の Before/After フローを具体的に記載。コレクション蓄積方式を明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | テスト配置先（infra vs core-service）の判断理由を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象外セクションで読み取りの TX 対応とコード重複解消を除外 |
| 5 | 技術的前提 | コードに現れない前提が考慮 | OK | TxContext の `&mut` 借用での複数書き込み、drop 時の自動ロールバック、楽観的ロックの競合検出を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾なし | OK | ADR-051（TxContext 設計）、エンティティ影響マップ（不変条件）と整合 |
