# Workflow/Detail.elm ADT ステートマシン - 機能解説

対応 PR: #839
対応 Issue: #816
Epic: #822（ADT ベースステートマシンの既存コード適用）

## 概要

`Page/Workflow/Detail.elm`（申請詳細ページ）の Model を、18 フィールドのフラット構造から ADT（Algebraic Data Type）ベースステートマシンにリファクタリングした。Loading 中に承認・コメント・再提出の操作フィールドが型レベルで存在しないようにすることで、不正な状態をコンパイル時に排除する。

## 背景

### ADT ベースステートマシンパターン（ADR-054）

[ADR-054](../../05_ADR/054_ADTベースステートマシンパターンの標準化.md) は、状態によって有効なフィールドが異なる場合に ADT で状態を分離するパターンを標準化した。Richard Feldman の "Making Impossible States Impossible"（2016）を Elm コミュニティの確立パターンとして明文化したもの。

このプロジェクトでは「型で表現できるものは型で表現する。不正な状態を表現不可能にする」を設計原則としており、ADR-054 はその状態遷移への適用である。

### 変更前の課題

申請詳細ページの Model は 18 フィールドのフラット構造で、3 つの操作モードのフィールドが混在していた:

- 承認/却下フロー: `comment`, `pendingAction`, `isSubmitting`
- コメントスレッド: `newCommentBody`, `isPostingComment`
- 再提出編集: `isEditing`, `editFormData`, `editApprovers`, `isResubmitting`

Loading/Failure 状態でこれらのフィールドが存在し、操作が型レベルで許可されていた。

### Epic #822 の中での位置づけ

```mermaid
flowchart LR
    ADR["ADR-054\n標準化"]
    D["#796\nDesigner.elm"]
    T["#818\nTask/Detail.elm"]
    N["#817\nWorkflow/New.elm"]
    W["#816\nWorkflow/Detail.elm"]

    ADR --> D
    ADR --> T
    ADR --> N
    ADR --> W

    style W fill:#e8f5e9,stroke:#4caf50
```

| Issue | 内容 | 状態 |
|-------|------|------|
| #796 | Designer.elm のリファクタリング（最初の適用） | 完了 |
| #818 | Task/Detail.elm のリファクタリング | 完了 |
| #817 | Workflow/New.elm のリファクタリング | 完了 |
| #816 | Workflow/Detail.elm のリファクタリング（本 PR） | 完了 |

## 用語・概念

| 用語 | 説明 | 関連コード |
|------|------|-----------|
| ADT ベースステートマシン | 状態ごとに異なる型を定義し、有効なフィールドだけを持たせるパターン | `PageState` |
| パターン A | ADR-054 の推奨パターン。外側の型に共通フィールド、内側に状態固有フィールド | `Model` + `LoadedState` |
| LoadedState | Loaded 時のみ存在する 16 フィールドを集約した型 | `LoadedState` |
| RemoteData | 非同期データの 4 状態（NotAsked/Loading/Failure/Success）を表す型 | `RemoteData ApiError a` |

## ビフォー・アフター

### Before（変更前）

```mermaid
stateDiagram-v2
    direction LR
    state "Model (18 fields)" as M {
        state "workflow: RemoteData" as W
        state "comment, pendingAction, ..." as Op
        state "newCommentBody, ..." as Cmt
        state "isEditing, editFormData, ..." as Edit
    }

    note right of M
        Loading 中も全 18 フィールドが存在
        操作フィールドへのアクセスが型で防げない
    end note
```

フラットな Model 構造:

| フィールド群 | フィールド数 | Loading 中の状態 |
|-------------|------------|-----------------|
| 共通 | 2（shared, workflowDisplayNumber） | 有効 |
| API データ | 2（workflow, definition） | `Loading` / `NotAsked` |
| 承認/却下 | 5（comment, isSubmitting, ...） | 空文字 / False / Nothing |
| コメント | 3（comments, newCommentBody, ...） | `Loading` / 空文字 / False |
| 再提出 | 6（isEditing, editFormData, ...） | False / empty / NotAsked |

#### 制約・課題

- Loading 中に操作フィールドが型レベルで存在し、誤ったアクセスをコンパイラが検出できない
- `init` で `getWorkflow` と `listComments` を並列発行するが、`GotComments` が先に届いた場合のハンドリングが曖昧

### After（変更後）

```mermaid
stateDiagram-v2
    direction LR

    [*] --> Loading
    Loading --> Failed: GotWorkflow Err
    Loading --> Loaded: GotWorkflow Ok
    Failed --> Loading: Refresh

    state Loaded {
        state "LoadedState (16 fields)" as LS
        state "workflow: WorkflowInstance" as WI
        state "definition: RemoteData" as Def
        state "comments: RemoteData" as Cmt
        state "承認/却下/差し戻し fields" as Approval
        state "再提出 fields" as Resubmit
    }

    Loaded --> Loading: Refresh
```

ADT 分離後の型構造:

| 型 | フィールド | 存在条件 |
|----|----------|---------|
| `Model` | shared, workflowDisplayNumber, state | 常に存在 |
| `PageState` | Loading / Failed ApiError / Loaded LoadedState | — |
| `LoadedState` | workflow, definition, comment, ... (16 fields) | Loaded 時のみ |

#### 改善点

- Loading/Failed 状態で操作フィールドにアクセスするとコンパイルエラーになる
- `workflow` が `RemoteData` から直値（`WorkflowInstance`）に変わり、Loaded 内での不要な case 分岐が解消
- コメント取得タイミングを `handleGotWorkflow` に移動し、Loading 中の格納先問題を構造的に解決

## アーキテクチャ

```mermaid
flowchart TB
    subgraph Frontend["フロントエンド（Elm）"]
        Main["Main.elm"]
        Detail["Page/Workflow/Detail.elm"]
        subgraph DetailModule["Detail.elm 内部構造"]
            Init["init"]
            Update["update"]
            HandleGW["handleGotWorkflow"]
            UpdateLoaded["updateLoaded"]
            View["view"]
            ViewBody["viewBody"]
            ViewLoaded["viewLoaded"]
        end
    end

    subgraph API["API レイヤー"]
        WfApi["WorkflowApi"]
        WfDefApi["WorkflowDefinitionApi"]
    end

    Main --> Detail
    Init --> WfApi
    HandleGW --> WfDefApi
    HandleGW --> WfApi
    Update --> HandleGW
    Update --> UpdateLoaded
    View --> ViewBody
    ViewBody --> ViewLoaded
```

## データフロー

### フロー 1: ページ初期ロード

```mermaid
sequenceDiagram
    participant Main as Main.elm
    participant Detail as Detail.elm
    participant WfApi as WorkflowApi
    participant WfDefApi as WorkflowDefinitionApi

    Main->>Detail: init(shared, displayNumber)
    Detail->>Detail: state = Loading
    Detail->>WfApi: getWorkflow

    WfApi-->>Detail: GotWorkflow Ok workflow
    Detail->>Detail: state = Loaded(initLoaded workflow)
    Detail->>WfDefApi: getDefinition(workflow.definitionId)
    Detail->>WfApi: listComments(displayNumber)

    WfDefApi-->>Detail: GotDefinition Ok definition
    Detail->>Detail: loaded.definition = Success definition

    WfApi-->>Detail: GotComments Ok comments
    Detail->>Detail: loaded.comments = Success comments
```

#### 処理ステップ

| # | レイヤー | ファイル:関数 | 処理内容 |
|---|---------|-------------|---------|
| 1 | Page | `Detail.elm:init` | Model を `Loading` 状態で初期化、`getWorkflow` を発行 |
| 2 | Page | `Detail.elm:handleGotWorkflow` | `initLoaded` で `LoadedState` を構築、`getDefinition` + `listComments` を並列発行 |
| 3 | Page | `Detail.elm:updateLoaded` | `GotDefinition`/`GotComments` を `LoadedState` に反映 |

### フロー 2: 承認操作

```mermaid
sequenceDiagram
    participant User as ユーザー
    participant Detail as Detail.elm
    participant WfApi as WorkflowApi

    User->>Detail: ClickApprove step
    Detail->>Detail: loaded.pendingAction = Just(ConfirmApprove step)
    Detail->>Detail: showModalDialog

    User->>Detail: ConfirmAction
    Detail->>Detail: loaded.isSubmitting = True
    Detail->>WfApi: approveStep

    WfApi-->>Detail: GotApproveResult Ok workflow
    Detail->>Detail: loaded.workflow = workflow
    Detail->>Detail: loaded.successMessage = "承認しました"
```

## 状態遷移

### PageState

```mermaid
stateDiagram-v2
    [*] --> Loading: init

    Loading --> Loaded: GotWorkflow Ok
    Loading --> Failed: GotWorkflow Err

    Loaded --> Loading: Refresh
    Failed --> Loading: Refresh

    note right of Loading: getWorkflow のみ発行
    note right of Loaded: definition + comments を並列 fetch
    note right of Failed: viewError で ApiError を表示
```

### Loaded 内の操作状態

```mermaid
stateDiagram-v2
    state Loaded {
        [*] --> Idle

        state "承認/却下フロー" as Approval {
            Idle --> PendingAction: ClickApprove/Reject/RequestChanges
            PendingAction --> Submitting: ConfirmAction
            Submitting --> Idle: GotResult Ok
            Submitting --> ErrorState: GotResult Err
            PendingAction --> Idle: CancelAction
        }

        state "再提出フロー" as Resubmit {
            Idle --> Editing: StartEditing
            Editing --> Resubmitting: SubmitResubmit
            Resubmitting --> Idle: GotResubmitResult Ok
            Resubmitting --> Editing: GotResubmitResult Err
            Editing --> Idle: CancelEditing
        }
    }
```

## 設計判断

機能・仕組みレベルの判断を記載する。コード実装レベルの判断は[コード解説](./01_ADTステートマシン_コード解説.md#設計解説)を参照。

### 1. コメント取得タイミングをどうするか

ADT 分離により、Loading 状態に `LoadedState` が存在しなくなった。`init` で `listComments` を並列発行すると、`GotComments` が Loading 中に届いた場合の格納先がない。

| 案 | UX 影響 | コード複雑度 | Task/Detail との一貫性 |
|----|---------|------------|---------------------|
| handleGotWorkflow で並列発行（採用） | コメント表示がわずかに遅延 | 低 | 一致 |
| init で並列発行 + Loading 中のバッファリング | なし | 高（バッファ機構が必要） | 不一致 |
| init で逐次発行（getWorkflow → getComments） | コメント表示が遅延 | 低 | 不一致 |

**採用理由**: コメントはページ下部の付随データであり UX 影響は最小。Task/Detail.elm と同一パターンで一貫性を確保できる。

### 2. Loaded 時の GotWorkflow をどう処理するか

承認/却下操作後にサーバーから最新の `WorkflowInstance` が返る。この応答も `GotWorkflow` Msg を通るため、Loaded 状態での `GotWorkflow` のハンドリングが必要。

| 案 | 操作フィールドの保持 | 副作用の再発行 |
|----|-------|--------|
| Loaded 時は workflow フィールドのみ更新（採用） | 保持される | なし |
| initLoaded で完全再構築 | リセットされる | definition + comments を再 fetch |

**採用理由**: 承認後はフォーム状態やコメントを維持すべき。部分更新により不要な再 fetch を避ける。

## 関連ドキュメント

- [コード解説](./01_ADTステートマシン_コード解説.md)
- [ADR-054: ADT ベースステートマシンパターンの標準化](../../05_ADR/054_ADTベースステートマシンパターンの標準化.md)
- [Issue #816](https://github.com/ka2kama/ringiflow/issues/816)
- [申請フォーム UI 設計](../../03_詳細設計書/10_ワークフロー申請フォームUI設計.md)
