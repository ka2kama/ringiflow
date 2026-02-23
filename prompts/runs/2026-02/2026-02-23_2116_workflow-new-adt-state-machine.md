# #817 Workflow/New.elm ADT ステートマシンリファクタリング

## 概要

Page.Workflow.New.elm のフラットな 13 フィールド Model を ADT ベースステートマシンにリファクタリングした。ADR-054 に基づき、#818（Task/Detail.elm）で確立されたパターンを踏襲。二段階 ADT（PageState + FormState）により、不正な状態を型レベルで表現不可能にした。

## 実施内容

### 型定義変更

- フラットな Model → `PageState (Loading | Failed | Loaded)` + `FormState (SelectingDefinition | Editing)` の二段階 ADT に変更
- `users` は `PageState` と独立して並行 fetch するため、外側 Model に `RemoteData ApiError (List UserItem)` として配置
- `selectedDefinition : WorkflowDefinition` で Maybe を排除し、`getSelectedDefinition` ヘルパーを削除

### update 分割

- `update` → `updateLoaded` → `updateEditing` の三段階に分割
- `GotDefinitions`/`GotUsers` は外側 `update` で処理
- `SelectDefinition` は `updateLoaded` で処理（SelectingDefinition → Editing 遷移）
- フォーム操作メッセージは `updateEditing` で処理

### view 分割

- `view` → `viewBody`（PageState パターンマッチ）→ `viewLoaded`（FormState パターンマッチ）
- Failed 状態のエラー表示に `ErrorState.viewSimple` + `ErrorMessage.toUserMessage` を使用

### テスト更新

- 行動的テストアプローチ（`sendMsg` ヘルパー）に移行
- `expectEditing` アサーションヘルパーで ADT のパターンマッチを隠蔽
- 状態遷移テスト 3 件追加、不正状態テスト 1 件削除（型レベルで不要に）
- elm-review 除外 2 件削除（`NoUnused.CustomTypeConstructors`/`CustomTypeConstructorArgs`）

### コンパイルエラー対応

- `PageState.Loading` と `RemoteData.Loading` の名前衝突を `RemoteData.Loading` で修飾して解決
- elm-review で検出された未使用インポート（`ApproverSelector` alias、`SaveMessage(..)`）を修正

## 判断ログ

- `users` を外側 Model に配置: `definitions` と独立した並行 fetch のため。`PageState` 内に入れると Loading 状態で GotUsers を受け取れない
- 二段階 ADT: Loading/Loaded に加え、Loaded 内の「定義未選択/編集中」も型で分離。フォームフィールドが定義選択前に型レベルで存在しなくなる
- `SaveMessage(..)` → `SaveMessage` にエクスポートを変更: elm-review が検出した通り、コンストラクタはモジュール外で使用されていない
- Refactor: `validateForm` の三重ネスト・パターンマッチが単一 Result マッチに簡略化されたことを確認（ADT の型安全性による構造的改善）

## 成果物

### コミット

- `434b92f` #817 WIP: Refactor Workflow/New.elm Model to ADT state machine
- `e0037e1` #817 Refactor Workflow/New.elm Model to ADT state machine

### 変更ファイル

| ファイル | 変更 |
|---------|------|
| `frontend/src/Page/Workflow/New.elm` | Model ADT 化 + update/view 分割 |
| `frontend/tests/Page/Workflow/NewTest.elm` | 行動的テストアプローチに移行 + 状態遷移テスト追加 |
| `frontend/review/src/ReviewConfig.elm` | New.elm 向け elm-review 除外 2 件削除 |
| `prompts/plans/817_workflow-new-adt-state-machine.md` | 計画ファイル |

### 検証

- `just check-all`: exit code 0（全テスト・リント通過）
- elm-test: 456 テスト全パス
- E2E テスト: 既存テスト変更なしでパス
