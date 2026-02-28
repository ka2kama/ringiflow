# 実装計画: #727 固定テンプレートを新スキーマに移行する

## Context

Epic #405（ワークフローデザイナー）の最終 Story。デザイナー機能（#722-#726）は完成済み。既存のシードデータ（固定テンプレート）を新スキーマに適合させ、デザイナーで作成した定義での申請・承認フローを E2E で検証する。

## スコープ

対象:
- シードデータに `position` フィールドを追加するマイグレーション
- デザイナー形式の定義で申請→承認フローが動作する E2E テスト
- E2E テスト突合表の更新

対象外:
- フォームエディタの実装（Phase 2-4 の後続機能）
- デザイナー UI（キャンバスの D&D 操作）の E2E テスト（Elm ユニットテスト 445 件でカバー済み）

## 設計判断

D1: マイグレーション方式 — JSON 全体置換（`jsonb_set` ではなく）
- 理由: `jsonb_set` では JSON 配列内の全要素に `position` を追加できない。seed データは構造が既知のため全体置換が明快

D2: E2E テスト方式 — API アシスト型（定義作成・公開を API で、申請・承認を UI で）
- 理由: デザイナーの SVG キャンバス操作は Elm ユニットテストでカバー済み。E2E で検証すべきは「デザイナー形式の定義で申請・承認が動作するか」というインテグレーションギャップ
- トレードオフ: キャンバス操作の E2E カバレッジは得られないが、テストの安定性と本質的な検証ポイントにフォーカスできる

D3: 定義 JSON に `form: { fields: [] }` を含める
- 理由: `form` セクションを省略するとフロントエンドで「フォーム定義の読み込みに失敗しました。」エラーが表示される。空の fields 配列を含めれば、デコーダが `Ok []` を返し、何も表示されない（期待通りの動作）

D4: End ノードの座標 — 承認完了を左(x=250)、却下を右(x=550) に水平分割
- 理由: 分岐ワークフローの視覚的慣例に従う。同じ論理深度にあるため y は同じ

---

## Phase 1: シードデータマイグレーション

新しいマイグレーションファイルで既存シードデータの `definition` JSONB に `position` を追加する。

ファイル: `backend/migrations/20260221000001_add_position_to_seed_definitions.sql`

### 汎用申請（4 ステップ）

| step_id | type | position |
|---------|------|----------|
| start | start | `{ "x": 400, "y": 50 }` |
| approval | approval | `{ "x": 400, "y": 200 }` |
| end_approved | end | `{ "x": 250, "y": 350 }` |
| end_rejected | end | `{ "x": 550, "y": 350 }` |

### 2段階承認申請（5 ステップ）

| step_id | type | position |
|---------|------|----------|
| start | start | `{ "x": 400, "y": 50 }` |
| manager_approval | approval | `{ "x": 400, "y": 200 }` |
| finance_approval | approval | `{ "x": 400, "y": 350 }` |
| end_approved | end | `{ "x": 250, "y": 500 }` |
| end_rejected | end | `{ "x": 550, "y": 500 }` |

注意: `form`, `assignee`, `status`, `transitions` は既存のまま保持。`position` のみ各 step に追加。

#### 確認事項
- 型: DesignerCanvas の position エンコード形式 → `frontend/src/Data/DesignerCanvas.elm` の `encodeStep`
- パターン: 既存シード定義の JSON 構造 → `backend/migrations/20260115000008_seed_system_data.sql`, `backend/migrations/20260212000001_seed_multi_step_workflow_definition.sql`

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

API テスト:
- 既存の `get_workflow_definition.hurl`, `list_workflow_definitions.hurl` が引き続きパスすることを確認（`$.data.definition exists` のみチェックしているため影響なし）

E2E テスト（該当なし）

---

## Phase 2: E2E テスト — デザイナー定義の申請・承認フロー

ファイル: `tests/e2e/tests/designer-workflow-flow.spec.ts`

### テストシナリオ

```
test("デザイナーで作成した定義で申請して承認できる")

Given: 管理者が API でデザイナー形式の定義を作成・公開する
  - POST /api/v1/workflow-definitions（name, definition JSON）
  - POST /api/v1/workflow-definitions/{id}/publish（version: 1）
  - 定義 JSON: steps（start, approval, end_approved, end_rejected）+ transitions + position + form: { fields: [] }

When: 新規申請画面でデザイナー定義を選択して申請する
  - /workflows/new に移動
  - 定義名をクリック
  - タイトルを入力
  - 承認者を検索・選択
  - 「申請する」をクリック

Then: 申請完了メッセージが表示される

When: 承認者がタスク詳細から承認する
  - openTaskDetail(page, uniqueTitle)
  - approveTask(page)

Then: 申請一覧でステータスが「承認済み」
  - verifyWorkflowStatus(page, uniqueTitle, "承認済み")
```

### 定義 JSON（デザイナー形式、1段階承認）

```json
{
  "form": { "fields": [] },
  "steps": [
    { "id": "start_1", "type": "start", "name": "開始", "position": { "x": 400, "y": 50 } },
    { "id": "approval_1", "type": "approval", "name": "承認", "assignee": { "type": "user" }, "position": { "x": 400, "y": 200 } },
    { "id": "end_1", "type": "end", "name": "承認完了", "status": "approved", "position": { "x": 250, "y": 350 } },
    { "id": "end_2", "type": "end", "name": "却下", "status": "rejected", "position": { "x": 550, "y": 350 } }
  ],
  "transitions": [
    { "from": "start_1", "to": "approval_1" },
    { "from": "approval_1", "to": "end_1", "trigger": "approve" },
    { "from": "approval_1", "to": "end_2", "trigger": "reject" }
  ]
}
```

step ID は `type_counter` 形式（デザイナーが生成する形式）。

### API 呼び出し方法

Playwright の `page.request` を使用（storageState からセッション Cookie を自動送信）:

```typescript
const response = await page.request.post('/api/v1/workflow-definitions', {
  headers: { 'X-Tenant-ID': TENANT_ID, 'Content-Type': 'application/json' },
  data: { name: uniqueName, definition: designerDefinitionJson }
});
```

#### 確認事項
- パターン: E2E テスト構造 → `tests/e2e/tests/approval.spec.ts`
- パターン: ヘルパー関数 → `tests/e2e/helpers/workflow.ts`
- ライブラリ: `page.request` の API → Playwright 公式ドキュメントまたは既存の `helpers/auth.ts` でのパターン
- 技術的前提: CSRF トークン要否 → 既存 E2E の `page.request` が CSRF なしで動作しているか確認

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト:
- [ ] デザイナーで作成した定義で申請して承認できる

---

## Phase 3: ドキュメント更新

### E2E テスト突合表

ファイル: `docs/50_テスト/E2Eテスト突合表.md`

- E2E-010（ワークフローデザイナー）の行を更新: テストファイル `designer-workflow-flow.spec.ts`、テスト件数 1、状態を「部分カバー（定義作成→申請→承認フロー）」に変更
- シナリオ外テスト一覧に追加（必要に応じて）
- サマリーの数値を更新

#### 確認事項
- ドキュメント: E2E テスト突合表の現在の構造 → `docs/50_テスト/E2Eテスト突合表.md`（確認済み）

#### テストリスト

該当なし（ドキュメントのみ）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | デザイナー定義に `form` セクションがないと「フォーム定義の読み込みに失敗しました。」エラーが表示される | エッジケース | D3: 定義 JSON に `form: { fields: [] }` を含めて回避 |
| 2回目 | Hurl API テストがシード定義の内部構造をアサートしている可能性 | 既存テスト影響 | 検証: `$.data.definition exists` のみ。影響なし |
| 3回目 | CSRF トークンが API 呼び出しに必要かもしれない | 技術的前提 | Phase 2 の確認事項に追加。既存 E2E の auth.ts が CSRF なしで動作していることを確認予定 |
| 4回目 | マイグレーションで `version` を変更するとワークフローインスタンスの整合性が壊れる | データ整合性 | マイグレーションは `definition` カラムのみ UPDATE。`version` は変更しない |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 4 件すべてに対応する Phase がある（Phase 1: シード移行, Phase 2: E2E, Phase 3: ドキュメント, 全体: just check-all） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 座標値、定義 JSON、ファイル名がすべて具体的に記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | D1-D4 でマイグレーション方式、テスト方式、form 処理、座標を決定 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | フォームエディタ実装とデザイナー UI E2E テストを対象外と明示 |
| 5 | 技術的前提 | 前提が考慮されている | OK | フロントエンドの form 欠如時の挙動を検証済み、Hurl テストへの影響を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 設計書 L222-225 の position 後方互換性方針と整合 |

## 検証方法

1. `just check-all`（lint + test + API test + E2E test）が全パス
2. 既存の E2E テスト（workflow.spec.ts, approval.spec.ts）が引き続きパス（シード定義の後方互換性）
3. 新規 E2E テスト（designer-workflow-flow.spec.ts）がパス（デザイナー定義のインテグレーション検証）
