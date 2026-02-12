# 計画: #475 複数ステップ承認のバックエンド実装

## Context

Phase 1 ではワークフローの1段階承認のみ実装済み。Phase 2-3 の最初の Story として、2〜3段階の順次承認をバックエンドで実装する。現在の実装ではステップ作成がハードコードされており（`step_id: "approval"` 固定）、承認時にインスタンスを即座に完了する。これを定義 JSON から複数ステップを抽出し、順次実行する仕組みに拡張する。

## 対象

- ドメインモデル（WorkflowInstance, WorkflowStep）の拡張
- Submit ユースケースの複数ステップ対応
- Approve/Reject ユースケースの次ステップ遷移ロジック
- API 層（BFF, Core Service）のリクエスト/レスポンス更新
- OpenAPI 仕様書の更新
- 多段階ワークフロー定義のシードデータ
- API テスト（Hurl）

## 対象外

- フロントエンド（Story #478）
- 差し戻し（Story #476）
- コメント機能（Story #477）
- 並列承認（Phase 3）

## 設計判断

### 1. Submit API の変更

現在: `SubmitWorkflowRequest { assigned_to: Uuid }`
変更後: `SubmitWorkflowRequest { approvers: Vec<StepApprover> }`

各承認ステップに対して step_id と assigned_to のペアを指定する。1段階承認は `approvers: [{ step_id: "approval", assigned_to: "..." }]` で表現。

理由: 複数ステップの承認者指定を明示的に行うため。step_id で定義との対応を保証する。

### 2. 定義 JSON からの承認ステップ抽出

定義 JSON の `steps` 配列から `type == "approval"` のステップを配列順で抽出する。この順序が承認の実行順序になる。

`transitions` は使用しない（順次承認では配列順で十分。条件分岐は Phase 3 で対応）。

### 3. 承認時の次ステップ遷移

ステップ承認後:
1. 定義から承認ステップの順序リストを取得
2. 現在のステップの次を特定（step_id で照合）
3. 次がある → 次ステップを Active 化、current_step_id を更新
4. 次がない → インスタンスを Approved に遷移

### 4. 却下時の残ステップ処理

ステップ却下後:
1. 残りの Pending ステップを全て Skipped に遷移
2. インスタンスを Rejected に遷移

### 5. ステップの display_number

Submit 時に承認ステップの順序に基づいて 1, 2, 3... を割り当てる。

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: ドメインモデルの拡張

#### 確認事項
- [ ] 型: WorkflowInstance のメソッド一覧 → `backend/crates/domain/src/workflow/instance.rs`
- [ ] 型: WorkflowStep の skipped() メソッド → `backend/crates/domain/src/workflow/step.rs`
- [ ] パターン: 既存の状態遷移テスト → 同ファイル内 `#[cfg(test)]`

#### テストリスト

ユニットテスト:
- [ ] WorkflowInstance: advance_to_next_step — InProgress 状態でステップ ID を更新できる
- [ ] WorkflowInstance: advance_to_next_step — InProgress 以外ではエラー
- [ ] WorkflowInstance: advance_to_next_step — version がインクリメントされる
- [ ] WorkflowStep: skipped — Pending 状態から Skipped に遷移できる
- [ ] WorkflowStep: skipped — Pending 以外ではエラー

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 2: 定義 JSON パーサーと Submit ユースケースの拡張

#### 確認事項
- [ ] 型: SubmitWorkflowInput の定義 → `backend/apps/core-service/src/usecase/workflow/command.rs`
- [ ] パターン: 既存の submit_workflow 実装 → 同ファイル
- [ ] ライブラリ: serde_json の Value 操作 → Grep `serde_json::Value` in domain/
- [ ] 型: CounterRepository の next_display_number → `backend/crates/infra/src/repository/`

#### テストリスト

ユニットテスト:
- [ ] extract_approval_steps — 定義 JSON から承認ステップを順序付きで抽出できる
- [ ] extract_approval_steps — 承認ステップがない定義でエラー
- [ ] extract_approval_steps — approval 以外のステップ（start, end）は除外される
- [ ] submit_workflow — 2ステップ定義で2つのステップが作成される（最初が Active、2番目が Pending）
- [ ] submit_workflow — approvers と定義のステップが一致しない場合エラー
- [ ] submit_workflow — current_step_id が最初の承認ステップの ID に設定される

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: Approve ユースケースの拡張（次ステップ遷移）

#### 確認事項
- [ ] 型: approve_step の現在の実装 → `backend/apps/core-service/src/usecase/workflow/command.rs`
- [ ] パターン: ステップ一覧の取得パターン → `find_by_instance` の使用箇所

#### テストリスト

ユニットテスト:
- [ ] approve_step（中間ステップ） — 次のステップが Active になる
- [ ] approve_step（中間ステップ） — current_step_id が次のステップ ID に更新される
- [ ] approve_step（中間ステップ） — インスタンスのステータスは InProgress のまま
- [ ] approve_step（最終ステップ） — インスタンスが Approved になる
- [ ] approve_step（最終ステップ） — current_step_id は変更なし（最終ステップのまま）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 4: Reject ユースケースの拡張（残ステップスキップ）

#### 確認事項
- [ ] パターン: reject_step の現在の実装 → 同ファイル
- [ ] 型: WorkflowStep の skipped メソッドのシグネチャ → Phase 1 で確認済み

#### テストリスト

ユニットテスト:
- [ ] reject_step（中間ステップ） — 残りの Pending ステップが Skipped になる
- [ ] reject_step（中間ステップ） — インスタンスが Rejected になる
- [ ] reject_step（最終ステップ） — インスタンスが Rejected になる（スキップ対象なし）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 5: API 層の更新

#### 確認事項
- [ ] 型: BFF の SubmitWorkflowRequest → `backend/apps/bff/src/handler/workflow.rs`
- [ ] 型: Core Service の submit ハンドラ → `backend/apps/core-service/src/handler/workflow.rs`
- [ ] パターン: BFF → Core Service のプロキシパターン → 既存 submit エンドポイント

#### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト:
- [ ] 2段階承認: 作成 → 申請 → ステップ1承認 → ステップ2承認 → Approved
- [ ] 2段階却下: 作成 → 申請 → ステップ1承認 → ステップ2却下 → Rejected（ステップなし）
- [ ] 1段階承認（後方互換）: 新 API 形式で1段階承認が動作する
- [ ] 申請時エラー: approvers と定義のステップが一致しない場合 400

E2E テスト（該当なし — フロントエンドは Story #478）

### Phase 6: シードデータと OpenAPI 更新

#### 確認事項
- [ ] パターン: 既存のシードデータ → `backend/migrations/` の seed_ ファイル
- [ ] パターン: OpenAPI の既存エンドポイント定義 → `openapi/openapi.yaml`

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし — Phase 5 で実施済み）
E2E テスト（該当なし）

## 変更対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/src/workflow/instance.rs` | `advance_to_next_step` メソッド追加 |
| `backend/crates/domain/src/workflow/step.rs` | `skipped()` メソッドの確認・修正（now パラメータ追加が必要か） |
| `backend/crates/domain/src/workflow/definition.rs` | `extract_approval_steps` 関数追加 |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | submit, approve, reject の拡張 |
| `backend/apps/core-service/src/handler/workflow.rs` | submit ハンドラの入力型変更 |
| `backend/apps/core-service/src/dto/workflow.rs` | SubmitInput の変更 |
| `backend/apps/bff/src/handler/workflow.rs` | SubmitWorkflowRequest の変更 |
| `openapi/openapi.yaml` | submit リクエストスキーマの更新 |
| `backend/migrations/` | 多段階ワークフロー定義のシードデータ追加 |
| `tests/api/` | 多段階承認の API テスト追加 |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | submit_workflow の SubmitWorkflowInput 変更が後方互換性を壊す | 不完全なパス | 1段階承認も新 API 形式（approvers 配列）で動作するよう設計。既存 API テストも更新 |
| 2回目 | 却下時に残りステップのスキップが未考慮 | 未定義 | Phase 4 に reject 時の残ステップスキップロジックを追加 |
| 3回目 | step display_number の割り当て方法が未定義 | 曖昧 | Submit 時に承認ステップ順で 1, 2, 3... を割り当てる方針を明記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 完了基準6項目すべてが Phase 1-6 でカバーされている |
| 2 | 曖昧さ排除 | OK | Submit API 変更、ステップ順序決定方法、display_number 割り当てを具体化 |
| 3 | 設計判断の完結性 | OK | 定義解析方法、API 変更方針、次ステップ遷移ロジックに判断理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明記。差し戻し、コメント、フロントエンドは対象外 |
| 5 | 技術的前提 | OK | 既存 DB スキーマで複数ステップ対応可能（スキーマ変更不要）を確認済み |
| 6 | 既存ドキュメント整合 | OK | 機能仕様書（更新済み）、ロードマップ Phase 2-3、承認却下機能設計と整合 |
