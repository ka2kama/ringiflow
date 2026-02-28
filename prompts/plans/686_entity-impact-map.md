# #686 エンティティ影響マップの作成と不変条件の形式知化

## Context

ワークフロー関連エンティティ（WorkflowInstance, WorkflowStep）に対し、複数のユースケースが状態更新を行うが、競合リスクと不変条件が暗黙知のままになっている。`docs/40_詳細設計書/エンティティ影響マップ/` にはテンプレートのみが存在し、実体のマップは0件。

#687〜#689 のトランザクション化作業の前提知識として、各ユースケースの更新パスと競合リスクを形式知化する。

## スコープ

対象:
- WorkflowInstance のエンティティ影響マップ
- WorkflowStep のエンティティ影響マップ
- 不変条件の文書化（影響マップ内に記載）
- README.md のリンク更新

対象外:
- トランザクション制御の実装（#687〜#689 で対応）
- WorkflowDefinition の影響マップ（更新パスが少なく、競合リスクが低いため）
- WorkflowComment の影響マップ（同上）
- tenant_id 条件の修正（#687 で対応）

## 成果物

| # | ファイル | 内容 |
|---|---------|------|
| 1 | `docs/40_詳細設計書/エンティティ影響マップ/WorkflowInstance.md` | Instance の影響マップ |
| 2 | `docs/40_詳細設計書/エンティティ影響マップ/WorkflowStep.md` | Step の影響マップ |
| 3 | `docs/40_詳細設計書/エンティティ影響マップ/README.md` | リンク一覧の更新 |

不変条件は各影響マップの「不変条件」セクションとして記載する。独立ファイルにはしない（影響マップと密結合のため）。

## Phase 1: WorkflowInstance 影響マップ

### 確認事項
- [x] TEMPLATE.md の構成 → 基本情報、更新パス、競合リスク、状態遷移、読み取りパス、関連エンティティの6セクション
- [x] WorkflowInstance のフィールド → `instance.rs` L78-93: id, tenant_id, definition_id, definition_version, display_number, title, form_data, status, version, current_step_id, initiated_by, submitted_at, completed_at, created_at, updated_at
- [x] 状態遷移メソッド → `instance.rs`: Draft→Pending(submitted), Pending→InProgress(with_current_step), InProgress→InProgress(advance_to_next_step), InProgress→Approved/Rejected/ChangesRequested, ChangesRequested→InProgress(resubmitted)

### 更新パス（調査結果）

| # | ユースケース | 操作 | 更新フィールド | 前提条件 |
|---|-------------|------|--------------|---------|
| 1 | create_workflow | INSERT | 全フィールド（status=Draft） | 定義が Published |
| 2 | submit_workflow | UPDATE | status(→InProgress), current_step_id, submitted_at, version | status=Draft |
| 3 | approve_step（次ステップあり） | UPDATE | current_step_id, version | status=InProgress |
| 4 | approve_step（最終ステップ） | UPDATE | status(→Approved), completed_at, version | status=InProgress |
| 5 | reject_step | UPDATE | status(→Rejected), completed_at, version | status=InProgress |
| 6 | request_changes_step | UPDATE | status(→ChangesRequested), version | status=InProgress |
| 7 | resubmit_workflow | UPDATE | status(→InProgress), form_data, current_step_id, completed_at(→None), version | status=ChangesRequested |

### 競合リスク（分析結果）

| フィールド | 更新元 | リスク | 現在の対策 |
|-----------|--------|--------|-----------|
| status | approve/reject/request_changes | 同一インスタンスへの同時判断操作 | 楽観的ロック（version check） |
| current_step_id | approve/submit/resubmit | approve 同時実行時に不整合 | 楽観的ロック |
| version | 全 UPDATE 操作 | 楽観的ロックの競合 | InfraError::Conflict → 409 |

### 不変条件（WorkflowInstance 側）

- INV-I1: status=Approved ⇒ completed_at IS NOT NULL
- INV-I2: status=Rejected ⇒ completed_at IS NOT NULL
- INV-I3: status=InProgress ⇒ current_step_id IS NOT NULL
- INV-I4: status=Draft ⇒ submitted_at IS NULL

## Phase 2: WorkflowStep 影響マップ

### 確認事項
- [x] WorkflowStep のフィールド → `step.rs` L90-107: id, instance_id, display_number, step_id, step_name, step_type, status, version, assigned_to, decision, comment, due_date, started_at, completed_at, created_at, updated_at, tenant_id
- [x] 状態遷移メソッド → `step.rs`: Pending→Active(activated), Active→Completed(approve/reject/request_changes), Pending→Skipped(skipped)

### 更新パス（調査結果）

| # | ユースケース | 操作 | 対象 | 更新フィールド | 前提条件 |
|---|-------------|------|------|--------------|---------|
| 1 | submit_workflow | INSERT | 全ステップ | 全フィールド（最初=Active, 残り=Pending） | Instance が Draft |
| 2 | approve_step | UPDATE | 当該ステップ | status(→Completed), decision(→Approved), comment, completed_at, version | status=Active |
| 3 | approve_step | UPDATE | 次ステップ | status(→Active), started_at | status=Pending |
| 4 | reject_step | UPDATE | 当該ステップ | status(→Completed), decision(→Rejected), comment, completed_at, version | status=Active |
| 5 | reject_step | UPDATE | Pending全ステップ | status(→Skipped) | status=Pending |
| 6 | request_changes_step | UPDATE | 当該ステップ | status(→Completed), decision(→RequestChanges), comment, completed_at, version | status=Active |
| 7 | request_changes_step | UPDATE | Pending全ステップ | status(→Skipped) | status=Pending |
| 8 | resubmit_workflow | INSERT | 新規全ステップ | 全フィールド（最初=Active, 残り=Pending） | Instance が ChangesRequested |

### 競合リスク（分析結果）

| フィールド | 更新元 | リスク | 現在の対策 |
|-----------|--------|--------|-----------|
| status（Active ステップ） | approve/reject/request_changes | 同一ステップへの同時判断 | 楽観的ロック |
| status（Pending ステップ） | reject + request_changes の Pending→Skipped | 同時実行による二重 Skip | 楽観的ロック（ただし skip 時は version check なし）|

### 不変条件（WorkflowStep 側）

- INV-S1: 同一 Instance 内で status=Active なステップは最大1つ
- INV-S2: status=Completed ⇒ decision IS NOT NULL
- INV-S3: status=Completed ⇒ completed_at IS NOT NULL
- INV-S4: status=Active ⇒ started_at IS NOT NULL

### クロスエンティティ不変条件

- INV-X1: Instance.status=Approved ⇒ 最終ステップの decision=Approved
- INV-X2: Instance.status=Rejected ⇒ いずれかのステップの decision=Rejected かつ残りは Skipped
- INV-X3: Instance.status=InProgress ⇒ 対応する Steps が1つ以上存在
- INV-X4: Step と Instance の状態更新は同一トランザクション内で完了すべき（現状未実装 → #687〜#689 で対応）

## Phase 3: README.md 更新

### 確認事項: なし（既知のパターンのみ）

README.md の「影響マップ一覧」セクションに、作成した2つのマップへのリンクを追加する。

## 設計判断

### 不変条件の配置場所

選択肢:
1. 各影響マップ内に記載 ← **採用**
2. `.claude/rules/` に独立ファイルとして記載
3. 影響マップ + rules の両方に記載

採用理由: 不変条件はエンティティの状態遷移と密結合であり、影響マップの更新パスと一緒に参照されることが多い。独立ファイルにすると参照が分散する。クロスエンティティ不変条件は両方の影響マップに記載する（同一内容を双方向リンク）。

### 不変条件の命名体系

- `INV-I*`: WorkflowInstance の不変条件
- `INV-S*`: WorkflowStep の不変条件
- `INV-X*`: クロスエンティティ不変条件

後続 Issue（#687〜#689）でトランザクション化する際、不変条件 ID で参照できる。

### skipped() の楽観的ロック不在

reject/request_changes で Pending ステップを Skipped にする処理は `step_repo.update_with_version_check` を使用しているが、`skipped()` メソッド自体は version をインクリメントしない。楽観的ロックの意図が曖昧なため、競合リスクとして記載する。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | create_workflow が更新パスに含まれていなかった | 未定義 | Phase 1 の更新パスに INSERT として追加 |
| 2回目 | 不変条件の配置場所が未決定だった | 曖昧 | 設計判断セクションで3つの選択肢を比較し、影響マップ内に記載する方針を決定 |
| 3回目 | skipped() が version をインクリメントしない点が競合リスクとして未記載 | 競合・エッジケース | Phase 2 の競合リスクに追記 |
| 4回目 | resubmit_workflow の Step 操作が INSERT であり UPDATE ではない点が不明瞭 | 曖昧 | Phase 2 の更新パス #8 を INSERT と明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全ユースケースが更新パスに含まれている | OK | create/submit/approve/reject/request_changes/resubmit の6ユースケース + 読み取り系を確認。comment は WorkflowComment エンティティでありスコープ外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各更新パスの操作（INSERT/UPDATE）、更新フィールド、前提条件が明示されている |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 不変条件の配置場所、命名体系、skipped のロック不在について判断済み |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | WorkflowDefinition/Comment を対象外とし理由を明記。tenant_id 修正も対象外 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | RLS による二重防御、楽観的ロックの仕組みを確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | TEMPLATE.md の構成、repository.md のルール、Issue #685 の記載と整合 |

## 検証方法

- ドキュメント作成のためコード変更・テストは対象外
- Issue #686 の完了基準チェックリストとの突合で検証
