# #1019 Designer.elm / New.elm の Update 系モジュール追加分割

## 概要

ADR-043 の 500 行閾値を超過している 3 ファイルを分割する。

| ファイル | 現在行数 | 目標 |
|---------|---------|------|
| `Designer/Update.elm` | 700 | ≤500 |
| `Designer/Canvas.elm` | 663 | ≤500 |
| `New/Update.elm` | 540 | ≤500 |

## 対象外

- `Detail.elm` 系: 閾値内（PR #1016 で分割済み）
- `instance.rs` 系: 閾値内
- Main.elm: アーキテクチャパターンの帰結（ADR-043 例外）

## 分割の設計判断

### 判断 1: Designer/Update.elm の分割軸

**選択: 永続化チェーン（Save→Validate→Publish）の抽出**

理由:
- Save/Validate/Publish は連鎖的なワークフローを形成し、凝集度が高い
- キャンバス操作（ドラッグ、選択、プロパティ編集）とは明確に異なる責務
- handleReconnectionDrop も同モジュールに配置（Update.elm から呼び出されるヘルパー関数として）

代替案:
- ドラッグ処理の抽出 → 永続化チェーンより凝集度が低い（CanvasMouseMove/Up は薄い dispatch）
- Msg をサブタイプに分割 → Types.elm の変更が必要で影響範囲が広い

### 判断 2: Designer/Canvas.elm の分割軸

**選択: ステップ描画 vs 接続線描画**

理由:
- ステップ（StepNode）と接続線（Transition）は独立した視覚要素
- viewTransitionLine (118行) が単独で最大の関数であり、接続線関連を分離する効果が大きい
- viewCanvasArea（オーケストレーター）は Canvas.elm に残し、各描画関数を呼び出す

### 判断 3: New/Update.elm の削減方法

**選択: DirtyState 統合 + API 関数抽出**

理由:
- ローカルの markDirty/clearDirty が Form.DirtyState と完全に重複（Designer/Update.elm は既に Form.DirtyState を使用）
- API 呼び出し関数（saveDraft, submitWorkflow 等）は純粋な Cmd 生成であり、独立性が高い

## Phase 1: Designer/Update.elm 分割（700 → ~450 + ~260）

### 確認事項

- 型: `CanvasState`, `Msg(..)` → `Designer/Types.elm`（Read で確認）
- パターン: Designer.elm が `DesignerUpdate.updateLoaded` をどう呼び出すか → `Designer.elm`（Read で確認）
- ライブラリ: `Form.DirtyState.markDirty`/`clearDirty`、`List.Extra` → Grep 既存使用
- リファクタリング: 分割対象の重複パターン → Update.elm 内のベジェ曲線パスは Canvas.elm 側のため Phase 2 で対応

### 新規ファイル: `Designer/UpdatePersistence.elm`（~260行）

永続化チェーン（Save→Validate→Publish）のハンドラ関数 + handleReconnectionDrop。

exposing:
- handleSave
- handleSaveResult
- handleValidate
- handleValidationResult
- handlePublishClicked
- handleConfirmPublish
- handleCancelPublish
- handlePublishResult
- handleDismissMessage
- handleReconnectionDrop

各関数のシグネチャ: 必要なパラメータ + `CanvasState -> ( CanvasState, Cmd Msg )`

### 変更: `Designer/Update.elm`（~450行）

- import `UpdatePersistence` を追加
- updateLoaded の永続化関連 case アーム（SaveClicked 〜 DismissMessage）を 1 行の委譲に変更
- CanvasMouseUp の DraggingReconnection ケースを `UpdatePersistence.handleReconnectionDrop` に委譲
- handleReconnectionDrop のローカル定義を削除
- モジュールドキュメントの 500 行超過コメントを更新

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト（該当なし — リファクタリングのため新規テスト不要）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: 既存テスト（elm-test）が全て通過すること。

## Phase 2: Designer/Canvas.elm 分割（663 → ~305 + ~345）

### 確認事項

- 型: `CanvasState`, `Msg(..)` → `Designer/Types.elm`（Phase 1 で確認済み）
- パターン: Canvas.elm の viewCanvasArea が各描画関数をどう呼び出すか → `Canvas.elm`（Read で確認）
- リファクタリング: viewTransitionLine と viewPreviewLine のベジェ曲線パス生成の重複 → 共通ヘルパー抽出を検討

### 新規ファイル: `Designer/CanvasTransitions.elm`（~345行）

接続線（Transition）関連の SVG 描画関数群。

exposing:
- viewArrowDefs
- viewTransitions
- viewReconnectionHandleLayer
- viewConnectionDragPreview

内部関数（expose しない）:
- viewArrowMarker
- viewTransitionLine
- viewReconnectionHandles
- viewPreviewLine

### 変更: `Designer/Canvas.elm`（~305行）

- import `CanvasTransitions` を追加
- viewCanvasArea 内で `CanvasTransitions.viewArrowDefs` 等を呼び出すように変更
- 接続線関連の関数を削除
- 不要になった import を除去
- モジュールドキュメントの 500 行超過コメントを更新

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: 既存テスト（elm-test）が全て通過すること。

## Phase 3: New/Update.elm 削減（540 → ~460）

### 確認事項

- 型: `EditingState` の `isDirty_` フィールド → `New/Types.elm`（Read で確認済み）
- パターン: Designer/Update.elm での `DirtyState.markDirty` 使用パターン → `Designer/Update.elm`（Read で確認済み）
- ライブラリ: `Form.DirtyState` の extensible record シグネチャ → `Form/DirtyState.elm`（Read で確認済み）

### 変更 3a: DirtyState 統合（~29行削減）

- `import Form.DirtyState as DirtyState` を追加
- `import Ports` を削除（DirtyState 統合後は不要）
- ローカルの `markDirty`/`clearDirty` 関数を削除（~30行）
- 呼び出し箇所を `DirtyState.markDirty` / `DirtyState.clearDirty` に変更

### 変更 3b: API 関数抽出（~52行削減）

### 新規ファイル: `New/Api.elm`（~75行）

ページ固有の API 呼び出し関数。

exposing:
- saveDraft
- submitWorkflow
- saveAndSubmit

内部関数:
- encodeFormValues

### 変更: `New/Update.elm`

- import `New.Api as Api` を追加（既存の `Api` モジュールとの名前衝突に注意 → `NewApi` 等で回避）
- API 関数（saveDraft, encodeFormValues, submitWorkflow, saveAndSubmit）を削除
- 呼び出し箇所を `NewApi.saveDraft` 等に変更

### 操作パス: 該当なし（リファクタリングのみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証: 既存テスト（elm-test）が全て通過すること。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | New/Update.elm の markDirty/clearDirty が Form.DirtyState と重複 | 既存手段の見落とし | DirtyState 統合を Phase 3a として追加 |
| 2回目 | DirtyState 統合だけでは 500行以下にならない（511行） | 不完全なパス | API 関数抽出を Phase 3b として追加 |
| 3回目 | UpdatePersistence に handleReconnectionDrop を含めると責務が不明確 | 曖昧 | ヘルパー関数セクションとして明示的に分離、モジュールドキュメントで説明 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 3 ファイル全てに分割計画がある | OK | Phase 1-3 で全ファイルをカバー |
| 2 | 曖昧さ排除 | 各 Phase の具体的な変更が確定 | OK | exposing リスト、削除対象、委譲パターンを明示 |
| 3 | 設計判断の完結性 | 分割軸の選択理由が記載 | OK | 3 つの判断に選択理由と代替案を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象外セクションで Detail 系・instance.rs を除外 |
| 5 | 技術的前提 | Elm の循環依存制約を考慮 | OK | Types.elm パターンは既に確立済み。新モジュールは Types.elm を import するのみ |
| 6 | 既存ドキュメント整合 | ADR-062 との整合 | OK | ADR-062 の「さらなる分割は Epic #996 で追跡中」に対応 |
