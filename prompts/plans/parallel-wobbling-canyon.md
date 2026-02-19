# #712 タスク詳細画面に差し戻しボタンを追加

## Context

タスク詳細画面（`Page/Task/Detail.elm`）に承認・却下は実装済みだが差し戻し（Request Changes）が未実装。申請詳細画面（`Page/Workflow/Detail.elm`）には差し戻しが実装済みで、そのパターンを正確に踏襲する。

## 対象・対象外

- 対象: `frontend/src/Page/Task/Detail.elm` の差し戻し機能追加
- 対象外: API（`WorkflowApi.requestChangesStep` は実装済み）、バックエンド（実装済み）、他ページ

## Phase 1: 差し戻し機能の追加

変更ファイル: `frontend/src/Page/Task/Detail.elm`

### 変更内容

`Page/Workflow/Detail.elm` の差し戻しパターンを踏襲し、以下を追加する。

1. **PendingAction 型** — `| ConfirmRequestChanges WorkflowStep` を追加
2. **Msg 型** — `| ClickRequestChanges WorkflowStep` と `| GotRequestChangesResult (Result ApiError WorkflowInstance)` を追加
3. **update 関数**:
   - `ClickRequestChanges step` → `pendingAction` に `ConfirmRequestChanges` をセット + ダイアログ表示
   - `ConfirmAction` の `ConfirmRequestChanges step` → API 呼び出し（`requestChangesStep` ヘルパー）
   - `GotRequestChangesResult result` → `handleApprovalResult "差し戻しました"` で処理
4. **requestChangesStep ヘルパー関数** — `rejectStep` と同じ構造で `WorkflowApi.requestChangesStep` を呼び出す
5. **viewApprovalButtons** — 承認と却下の間に差し戻しボタンを追加（`Button.Warning` バリアント）
6. **viewConfirmDialog** — `ConfirmRequestChanges step` のケースを追加（`ConfirmDialog.Caution` スタイル、メッセージ: 「この申請を差し戻しますか？」）
7. **モジュールドキュメント** — 「承認/却下操作」→「承認/却下/差し戻し操作」に更新

### 確認事項

- [x] 型: `PendingAction`, `Msg` → `Page/Task/Detail.elm` L66-68, L126-136 — 2バリアント（ConfirmApprove/ConfirmReject）、差し戻しの追加で3バリアント
- [x] パターン: `Page/Workflow/Detail.elm` の差し戻し実装 → L74 (`ConfirmRequestChanges`), L173 (`ClickRequestChanges`), L178 (`GotRequestChangesResult`), L295-304 (ConfirmAction 内), L320 (結果ハンドリング), L1184-1193 (確認ダイアログ), L1261-1273 (ボタン)
- [x] ライブラリ: `WorkflowApi.requestChangesStep` → `Api/Workflow.elm` L221-241、`approveStep`/`rejectStep` と同じシグネチャ
- [x] コンポーネント: `Button.Warning` → `Component/Button.elm` L49, `ConfirmDialog.Caution` → `Component/ConfirmDialog.elm` L61

### テストリスト

ユニットテスト（該当なし — 分岐ロジックなし、全て Elm コンパイラの型チェックで保証される配線コード）

ハンドラテスト（該当なし）

API テスト（該当なし — `requestChangesStep` API は既存テスト済み）

E2E テスト（該当なし — 自動 E2E テスト基盤未導入。手動確認で代替）

### 手動検証手順

1. `just dev-all` でサーバー起動
2. Active なタスクがあるユーザーでログイン
3. タスク詳細画面を開く
4. 差し戻しボタンが表示されることを確認
5. 差し戻しボタンをクリック → 確認ダイアログが表示される
6. 確認ダイアログで「差し戻す」をクリック → 成功メッセージが表示される

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ボタン配置順序が未定義 | 曖昧 | Workflow/Detail と同じ順序（承認→差し戻し→却下）を明記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Workflow/Detail の差し戻し関連コードすべて（型、Msg、update、view、dialog）を列挙済み |
| 2 | 曖昧さ排除 | OK | 各変更箇所に具体的な値・パターンを明記 |
| 3 | 設計判断の完結性 | OK | 判断不要（既存パターン踏襲のみ） |
| 4 | スコープ境界 | OK | 対象・対象外を明記、1ファイルのみ |
| 5 | 技術的前提 | OK | API・コンポーネントの存在を Grep/Read で確認済み |
| 6 | 既存ドキュメント整合 | OK | Issue #712 の完了基準と計画が一致 |
