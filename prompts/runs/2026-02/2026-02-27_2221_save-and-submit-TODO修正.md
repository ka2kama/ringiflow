# save-and-submit TODO コメント修正

## 概要

Issue #889（ワークフロー申請の保存→申請連続処理の実装）に着手し、Issue 精査の As-Is 検証で保存→申請の連続処理がメッセージチェーンパターンで既に実装済みであることを発見した。Issue のスコープを「不正確な TODO コメント・docstring の修正」に変更し、コメント修正のみを実施した。

## 実施内容

### Issue 精査と As-Is 検証

- Issue #889 の前提「保存のみを行い、保存成功後にユーザーが再度申請ボタンを押す必要がある」を `origin/main` 基準で検証
- `GotSaveAndSubmitResult` ハンドラ（`New.elm:559-571`）が保存成功時に `submitWorkflow` を自動呼び出ししており、連続処理は実装済みと判明
- TODO コメント（`:805-807`）の「保存のみ行い」記述と実際の挙動が矛盾
- スコープを「コメント修正」に変更して続行

### コメント修正

- `saveAndSubmit` 関数の docstring を実際の挙動（メッセージチェーンによる保存→申請連続処理）に合わせて更新
- MVP 簡略化コメント 3 行と `TODO(#889)` を削除

## 判断ログ

- Issue 精査で As-Is 検証の結果、スコープを「保存→申請連続処理の実装」から「コメント修正」に変更した（理由: 連続処理は既に実装済み）

## 成果物

### コミット

- `#889 Clean up misleading saveAndSubmit comments and TODO`

### 変更ファイル

- `frontend/src/Page/Workflow/New.elm`: docstring 更新、TODO コメント削除（+3/-8）

### PR

- #965: Ready for Review
