# Workflow/Detail.elm ADT ステートマシンリファクタリング

## 概要

Issue #816 に基づき、`Page/Workflow/Detail.elm` の 18 フィールドのフラット Model を ADT ベースステートマシン（ADR-054 パターン A）にリファクタリングした。Loading 中に承認/コメント/再提出の操作フィールドが型レベルで存在しないようにすることが目的。

## 実施内容

### 1. Issue 精査と計画作成

- Issue #816 の As-Is を検証し、Model が依然フラットであることを確認
- Task/Detail.elm（#818）と Workflow/New.elm（#817）の確立済みパターンを調査
- 計画ファイルを作成し、全 Msg ハンドラ・view 関数のシグネチャ変更を網羅

### 2. ADT リファクタリング

型定義の変更:
- `Model` を `{ shared, workflowDisplayNumber, state: PageState }` に簡素化
- `PageState = Loading | Failed ApiError | Loaded LoadedState` を定義
- `LoadedState` に 16 フィールドを集約（`definition`, `comments` は `RemoteData` を維持）

新規ヘルパー関数:
- `initLoaded`: `WorkflowInstance` から `LoadedState` を構築
- `handleGotWorkflow`: Loading→Loaded 遷移と Loaded 時の部分更新を処理
- `updateLoaded`: Loaded 状態専用の Msg ハンドラ

### 3. コンストラクタ名衝突の解決

`PageState.Loading` が `RemoteData.Loading` をシャドウする問題が発生。Workflow/New.elm の既存パターンに倣い、`RemoteData.Loading` と修飾して解決。`NotAsked`/`Success`/`Failure` は `PageState` と名前が衝突しないため修飾不要。

### 4. elm-review 指摘の対応

`NoUnused.CustomTypeConstructorArgs` が `Failed ApiError` の引数未使用を検出。`Failed _ -> viewError` を `Failed err -> viewError err` に変更し、`ErrorMessage.toUserMessage` でユーザーフレンドリーなエラー表示に改善。Task/Detail.elm のパターンと統一した。

## 判断ログ

- コメント fetch 順序の変更: `init` から `listComments` を除去し、`handleGotWorkflow` で `getDefinition` + `listComments` を並列発行する方式に変更。理由: Loading 状態に `LoadedState` が存在しないため `GotComments` の格納先がない
- `RemoteData.Loading` の修飾: `import RemoteData exposing (RemoteData(..))` を維持し、衝突する `Loading` のみ修飾。`RemoteData exposing (RemoteData)` に変更して全修飾する案は、非衝突コンストラクタまで冗長になるため不採用
- `viewError` の改善: elm-review 指摘を契機に、ハードコードされたエラーメッセージから `ErrorMessage.toUserMessage` パターンに改善。スコープ外の改善だが、elm-review がパスしない以上必須かつ Task/Detail.elm との一貫性向上に寄与

## 成果物

### コミット

- `ade7cfa` #816 WIP: Refactor Workflow/Detail.elm Model to ADT state machine
- `b731152` #816 Refactor Workflow/Detail.elm Model to ADT state machine

### 作成/更新ファイル

- `frontend/src/Page/Workflow/Detail.elm` — ADT リファクタリング本体
- `prompts/plans/imperative-napping-riddle.md` — 計画ファイル

### PR

- #839 (Draft): https://github.com/ka2kama/ringiflow/pull/839
