# 2026-02-23 Designer.elm ADT ベースステートマシンリファクタリング

## 概要

#796: Designer.elm のフラットな Model（21 フィールド + `RemoteData`）を ADT ベースステートマシンパターン（ADR-054）にリファクタリングした。Loading 状態でキャンバス関連フィールドが型レベルで存在しない構造にし、不正な状態を表現不可能にした。

## 実施内容

### 型構造の変更

フラットな Model を 3 層構造に変更:

```elm
type alias Model =
    { shared : Shared
    , definitionId : String
    , state : PageState
    }

type PageState
    = Loading
    | Failed ApiError
    | Loaded CanvasState
```

- `RemoteData` の import を削除し、自前の `PageState` カスタム型に置換
- `CanvasState` type alias に 20 フィールドを格納（Loaded 時のみ存在）
- `DirtyState` の extensible record 互換性を維持（`isDirty_` フィールド）

### update 関数の分割

- 外側 `update`: `GotDefinition` のみ処理（状態遷移を担当）
- 内側 `updateLoaded`: Loaded 状態での全 Msg を処理
- `updateLoaded` は `shared` と `definitionId` をパラメータとして受け取る設計

### view 関数の変更

- `view` で `model.state` をパターンマッチ（Loading / Failed / Loaded）
- `viewLoaded : CanvasState -> Html Msg` を新設
- 全 14 view サブ関数: `Model ->` → `CanvasState ->`

### subscriptions の最適化

- `receiveCanvasBounds`: 全状態で購読（レスポンスは Loaded 遷移後に到着するが、購読は遷移前から必要）
- `onKeyDown`, `onMouseMove`, `onMouseUp`: Loaded 時のみ

### テストの構造変更

- Canvas レベルヘルパー導入: `defaultCanvas`, `canvasWithBounds`, `canvasWithOneStep`, `canvasWithEndStep`
- `expectLoaded` ヘルパーでアサーションの可読性を維持
- record update パターンを 2 層構造に変換

## 判断ログ

- 設計判断は計画ファイル（`prompts/plans/796_designer-adt-state-machine.md`）に 5 項目記載済み
- 実装時の追加判断なし（計画通りに実装完了）

## 成果物

コミット:
- `ed5a7b0` #796 WIP: Refactor Designer.elm Model to ADT state machine（作業中間コミット）
- 本セッションで Designer.elm / DesignerTest.elm の実装を完了（未コミット）

変更ファイル:
- `frontend/src/Page/WorkflowDefinition/Designer.elm` — Model を ADT ベースステートマシンに変更
- `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm` — テストヘルパーと全テストを新構造に対応

検証:
- Elm コンパイル: 成功
- テスト: 454 件全パス
- `just check-all`: 全パス
