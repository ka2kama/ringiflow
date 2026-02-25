# #906 接続線プロパティパネルと削除 UI

## Context

ワークフローデザイナーで接続線（Transition）を選択した際、プロパティパネルに何も表示されない。
ステップ選択時にはプロパティパネルに情報と削除 UI が表示されるが、接続線との間で UI の対称性が崩れている。
Delete キーによる削除は実装済みだが、UI からその機能の存在を知る手段がない。

## スコープ

対象:
- `Designer.elm`: `viewPropertyPanel` の拡張、`DeleteSelectedTransition` Msg 追加
- `DesignerCanvas.elm`: `triggerLabel` ヘルパー追加
- `DesignerTest.elm`: `DeleteSelectedTransition` のテスト追加

対象外:
- バックエンド（API 変更なし）
- 接続線のトリガー種別の編集機能（読み取り専用表示のみ）
- 詳細設計書の更新（既存の Designer 設計に view 追加のみ）

## Phase 1: 接続線プロパティパネルの view 実装と削除 UI

### 確認事項

- 型: `Transition = { from : String, to : String, trigger : Maybe String }` → `frontend/src/Data/DesignerCanvas.elm:83-87`
- 型: `CanvasState.selectedTransitionIndex : Maybe Int` → `Designer.elm:83`
- パターン: `viewStepProperties` の構造（種別ラベル + フィールド + 削除ボタン + ヒント） → `Designer.elm:1757-1793`
- パターン: `DeleteSelectedStep` Msg と `deleteSelectedStep` 関数 → `Designer.elm:159, 756-776`
- パターン: `KeyDown "Delete"` の接続線削除ロジック → `Designer.elm:702-716`
- ライブラリ: `FormField.viewReadOnlyField : String -> String -> String -> Html msg` → `Component/FormField.elm:143`
- ライブラリ: `Button.view : { variant, disabled, onClick } -> List (Html msg) -> Html msg` → 既存使用 `Designer.elm:1783-1788`

### 設計判断

1. `DeleteSelectedTransition` Msg を新規追加し、`KeyDown "Delete"` の接続線削除ロジックを共通化する
   - 代替案: `KeyDown "Delete"` のロジックをインラインで重複させる → DRY 違反のため不採用
   - 共通化方法: `deleteSelectedTransition : CanvasState -> ( CanvasState, Cmd Msg )` ヘルパー関数を抽出し、`KeyDown "Delete"` と `DeleteSelectedTransition` の両方から呼び出す

2. `triggerLabel` 関数は `DesignerCanvas.elm` に配置する
   - 理由: `Transition` 型と同じモジュールに配置し、データ表現の責務を集約する
   - `defaultStepName : StepType -> String` と同じパターン

3. `viewPropertyPanel` の分岐優先順位: `selectedTransitionIndex` → `selectedStepId` → デフォルト
   - 理由: `KeyDown "Delete"` の既存ロジックと同じ優先順位（`Designer.elm:704`）

4. 接続情報の表示フィールド:
   - 接続元ステップ名: `Dict.get transition.from canvas.steps` で名前を解決、未発見時はステップ ID を表示
   - 接続先ステップ名: 同上
   - トリガー種別: `triggerLabel transition.trigger` で表示（承認 / 却下 / なし）

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 接続線をクリックしてプロパティパネルに情報が表示される | 正常系 | ユニット |
| 2 | プロパティパネルの「接続を削除」ボタンで接続線を削除する | 正常系 | ユニット |
| 3 | 接続線未選択時に DeleteSelectedTransition を受けても何も起きない | 準正常系 | ユニット |

### テストリスト

ユニットテスト:
- [ ] `DeleteSelectedTransition` で選択中の接続線が削除される
- [ ] `DeleteSelectedTransition` で isDirty が true になる
- [ ] `DeleteSelectedTransition` で selectedTransitionIndex が Nothing になる
- [ ] 接続線未選択時に `DeleteSelectedTransition` を受けても状態が変わらない

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — E2E テストインフラが Designer にない）

### 実装手順

#### 1. `DesignerCanvas.elm` に `triggerLabel` を追加

```elm
triggerLabel : Maybe String -> String
triggerLabel trigger =
    case trigger of
        Just "approve" ->
            "承認"

        Just "reject" ->
            "却下"

        Just other ->
            other

        Nothing ->
            "なし"
```

expose リストに `triggerLabel` を追加。

#### 2. `Designer.elm` に `DeleteSelectedTransition` Msg を追加

`Msg` 型に `DeleteSelectedTransition` バリアントを追加。

#### 3. 接続線削除ロジックの共通化

`KeyDown "Delete"` の接続線削除ロジック（Designer.elm:705-716）を `deleteSelectedTransition` 関数として抽出:

```elm
deleteSelectedTransition : CanvasState -> ( CanvasState, Cmd Msg )
deleteSelectedTransition canvas =
    case canvas.selectedTransitionIndex of
        Just index ->
            let
                ( dirtyCanvas, dirtyCmd ) =
                    DirtyState.markDirty canvas
            in
            ( { dirtyCanvas
                | transitions = removeAt index dirtyCanvas.transitions
                , selectedTransitionIndex = Nothing
              }
            , dirtyCmd
            )

        Nothing ->
            ( canvas, Cmd.none )
```

`KeyDown "Delete"` ハンドラと `DeleteSelectedTransition` ハンドラの両方から呼び出す。

#### 4. `viewPropertyPanel` を拡張

`selectedTransitionIndex` の分岐を `selectedStepId` の前に追加:

```elm
viewPropertyPanel : CanvasState -> Html Msg
viewPropertyPanel canvas =
    div [ class "w-64 shrink-0 border-l border-secondary-200 bg-white p-4 overflow-y-auto" ]
        [ h2 [ class "mb-3 text-xs font-semibold uppercase tracking-wider text-secondary-500" ]
            [ text "プロパティ" ]
        , case canvas.selectedTransitionIndex of
            Just index ->
                case List.Extra.getAt index canvas.transitions of
                    Just transition ->
                        viewTransitionProperties canvas transition

                    Nothing ->
                        viewNoSelection

            Nothing ->
                case canvas.selectedStepId of
                    Just stepId ->
                        case Dict.get stepId canvas.steps of
                            Just step ->
                                viewStepProperties canvas step

                            Nothing ->
                                viewNoSelection

                    Nothing ->
                        viewNoSelection
        ]
```

`viewNoSelection` ヘルパーを抽出して重複を排除:

```elm
viewNoSelection : Html msg
viewNoSelection =
    p [ class "text-sm text-secondary-400" ]
        [ text "ステップまたは接続線を選択してください" ]
```

注: デフォルトメッセージを「ステップを選択してください」→「ステップまたは接続線を選択してください」に更新。接続線も選択可能になったため。

#### 5. `viewTransitionProperties` 関数を作成

ステッププロパティパネルの構造を踏襲:

```elm
viewTransitionProperties : CanvasState -> Transition -> Html Msg
viewTransitionProperties canvas transition =
    let
        stepName stepId =
            Dict.get stepId canvas.steps
                |> Maybe.map .name
                |> Maybe.withDefault stepId
    in
    div [ class "space-y-4" ]
        [ -- 種別ラベル
          div [ class "mb-2" ]
            [ span
                [ class "inline-block rounded-full bg-secondary-100 px-2 py-0.5 text-xs font-medium text-secondary-600" ]
                [ text "接続" ]
            ]
        , FormField.viewReadOnlyField "transition-from" "接続元" (stepName transition.from)
        , FormField.viewReadOnlyField "transition-to" "接続先" (stepName transition.to)
        , FormField.viewReadOnlyField "transition-trigger" "トリガー" (DesignerCanvas.triggerLabel transition.trigger)
        , div [ class "mt-6 border-t border-secondary-200 pt-4" ]
            [ Button.view
                { variant = Button.Error
                , disabled = False
                , onClick = DeleteSelectedTransition
                }
                [ text "接続を削除" ]
            , p [ class "mt-1 text-xs text-secondary-400" ]
                [ text "Delete キーでも削除できます" ]
            ]
        ]
```

#### 6. テスト追加

`DesignerTest.elm` に `deleteSelectedTransitionTests` を追加し、suite に登録。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `KeyDown "Delete"` と `DeleteSelectedTransition` でロジック重複が発生する | 重複の排除 | `deleteSelectedTransition` ヘルパー関数を抽出して共通化 |
| 1回目 | デフォルトメッセージ「ステップを選択してください」が接続線選択を反映していない | 状態網羅漏れ | 「ステップまたは接続線を選択してください」に更新 |
| 1回目 | `List.Extra.getAt` が必要（既にインポート済み確認 `Designer.elm:33`） | 既存手段の見落とし | `List.Extra` は既にインポート済み、問題なし |
| 1回目 | `viewNoSelection` の重複（3箇所） | 重複の排除 | ヘルパー関数として抽出 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 完了基準 3 項目（接続情報表示、削除ボタン、Delete ヒント）すべて Phase 1 でカバー |
| 2 | 曖昧さ排除 | OK | トリガー種別の表示値（承認/却下/なし）、フィールド ID、CSS クラスを具体的に記載 |
| 3 | 設計判断の完結性 | OK | `deleteSelectedTransition` 共通化、`triggerLabel` 配置先、分岐優先順位に判断理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明記、トリガー編集は対象外 |
| 5 | 技術的前提 | OK | `List.Extra.getAt` のインポート確認済み、`FormField.viewReadOnlyField` のシグネチャ確認済み |
| 6 | 既存ドキュメント整合 | OK | ADR-054（型安全ステートマシン）の方針と整合、新規 ADR 不要 |

## 検証方法

1. `just test-frontend` で Elm ユニットテストが全て通ること
2. `just check-all` で全体のリント + テストが通ること
3. 開発サーバーで以下を手動確認:
   - 接続線をクリック → プロパティパネルに情報表示
   - 「接続を削除」ボタン → 接続線が削除される
   - ステップ選択 → 従来通りのプロパティ表示（デグレなし）
