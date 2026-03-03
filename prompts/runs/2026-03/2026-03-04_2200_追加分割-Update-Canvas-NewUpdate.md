# 追加分割: Designer/Update, Designer/Canvas, New/Update

Issue: #1019
PR: #1021
日付: 2026-03-04

## 概要

ADR-062 の分割実施後に 500行閾値を超過していた 3 ファイルを追加分割し、全ファイルを閾値以下に縮小した。

## 変更内容

### Phase 1: Designer/Update.elm（700 → 462行）

| ファイル | 行数 | 内容 |
|---------|------|------|
| `Update.elm` | 462 | ドラッグ処理、プロパティ編集、選択、キーボード、削除 |
| `UpdatePersistence.elm`（新規） | 352 | Save → Validate → Publish チェーン + 接続線付け替え |

分割軸: 永続化操作（API 呼び出しとその結果ハンドリング）を `UpdatePersistence` に抽出。

### Phase 2: Designer/Canvas.elm（663 → 325行）

| ファイル | 行数 | 内容 |
|---------|------|------|
| `Canvas.elm` | 325 | ステップノード、グリッド、ドラッグプレビュー |
| `CanvasTransitions.elm`（新規） | 351 | 矢印マーカー、接続線、ドラッグプレビュー、付け替えハンドル |

分割軸: SVG 描画対象の種類（ステップ vs 接続線）。`bezierPathData` の重複も解消。

### Phase 3: New/Update.elm（540 → 447行）

| ファイル | 行数 | 内容 |
|---------|------|------|
| `Update.elm` | 447 | updateLoaded, updateEditing + バリデーション + ヘルパー |
| `Api.elm`（新規） | 69 | saveDraft, submitWorkflow, saveAndSubmit, encodeFormValues |

二段階アプローチ:
1. DirtyState 統合: ローカルの `markDirty`/`clearDirty` を `Form.DirtyState` モジュールで置換（~30行削減）
2. API 関数抽出: API 呼び出し関数を `New/Api.elm` に抽出（~60行削減）

### その他

- 例外リスト `.config/file-size-exceptions.txt` から 3 ファイルを削除
- ADR-062 に追加分割結果を反映
- elm-review で検出された未使用インポート 5 件を修正
- `handleSaveResult` の未使用パラメータ `definitionId` を削除

## 判断ログ

### UpdatePersistence の責務範囲に handleReconnectionDrop を含めた理由

`handleReconnectionDrop` は永続化操作ではないが、CanvasMouseUp の DraggingReconnection アームで呼び出される「接続線の付け替え完了処理」。Update.elm にこのハンドラだけ残すと handleReconnectionDrop の呼び出しが遠くなるため、「接続線の状態変更」という関連責務として UpdatePersistence に含めた。

### New/Api.elm の import alias を NewApi にした理由

Update.elm には `import Api exposing (ApiError)` と `import Api.Workflow as WorkflowApi` が既存。`Page.Workflow.New.Api` を `Api` として import すると名前衝突するため、`NewApi` で回避。

## コミット一覧

1. `#1019 Extract persistence handlers from Designer/Update.elm into UpdatePersistence.elm`
2. `#1019 Extract transition SVG rendering from Canvas.elm into CanvasTransitions.elm`
3. `#1019 Extract API functions from New/Update.elm into New/Api.elm`
4. `#1019 Remove unused imports and parameters from extracted modules`
5. `#1019 Update ADR-062 with additional split results and add plan file`
6. `#1019 Remove split files from file-size exceptions list`
