# 接続線プロパティパネルと削除UI

Issue: #906
PR: #909
計画ファイル: `prompts/plans/906_transition-property-panel.md`

## 概要

ワークフローデザイナーで接続線（Transition）を選択した際に、右側プロパティパネルに接続情報（接続元・接続先・トリガー種別）と削除UIを表示する機能を実装した。

## 実施内容

### Phase 1: 接続線プロパティパネルの view 実装と削除 UI

1. `DesignerCanvas.elm` に `triggerLabel` 関数を追加（トリガー種別の表示ラベル: 承認/却下/なし）
2. `Designer.elm` に `DeleteSelectedTransition` Msg を追加
3. `deleteSelectedTransition` ヘルパー関数を抽出し、`KeyDown "Delete"` ハンドラと共通化
4. `viewPropertyPanel` を拡張して `selectedTransitionIndex` の分岐を追加
5. `viewTransitionProperties` 関数を新規作成（接続元・接続先のステップ名解決、トリガー種別表示、削除ボタン）
6. `viewNoSelection` ヘルパーを抽出し、未選択時メッセージの重複を排除
7. デフォルトメッセージを「ステップを選択してください」→「ステップまたは接続線を選択してください」に更新
8. `DesignerTest.elm` に `DeleteSelectedTransition` のテストを追加（削除成功・未選択時の no-op）

## 判断ログ

- `deleteSelectedTransition` 共通化: `KeyDown "Delete"` と `DeleteSelectedTransition` で削除ロジックが重複するため、ヘルパー関数に抽出した。`deleteSelectedStep` と同じパターン
- `triggerLabel` の配置: `Transition` 型と同じ `DesignerCanvas.elm` に配置。`defaultStepName : StepType -> String` と同パターン（データ表現の責務）
- `viewPropertyPanel` の分岐優先順位: `selectedTransitionIndex` → `selectedStepId` → デフォルト。`KeyDown "Delete"` の既存優先順位と統一
- Refactor: `viewNoSelection` を抽出して 3 箇所の重複を排除

## 成果物

コミット:
- `063b6fac` #906 Add transition property panel and delete UI in workflow designer

変更ファイル:
- `frontend/src/Data/DesignerCanvas.elm`: `triggerLabel` 関数追加（+19行）
- `frontend/src/Page/WorkflowDefinition/Designer.elm`: Msg・update・view の拡張（+106行, -21行）
- `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm`: `deleteSelectedTransitionTests` 追加（+89行）
- `prompts/plans/906_transition-property-panel.md`: 計画ファイル
