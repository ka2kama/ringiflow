# #725 ワークフローデザイナー キャンバスとステップ配置

## コンテキスト

ワークフローデザイナーの中核 UI を実装する。ADR-053 で決定した SVG + Elm 直接レンダリング方式に基づき、キャンバス上にステップをドラッグ&ドロップで配置・操作する機能を構築する。

バックエンド CRUD API（#723）は実装済み。設計ドキュメント（ADR-053、機能仕様書、詳細設計書）は完備。フロントエンドのグリーンフィールド実装。

## スコープ

対象:
- SVG キャンバスコンポーネント（グリッド + ステップノード描画）
- ステップパレット（開始・承認・終了の 3 種）
- パレット → キャンバスへのドラッグ&ドロップ配置
- ステップの選択・移動・削除
- Elm Ports によるキャンバス座標変換
- ルーティング（`/workflow-definitions/new`）

対象外:
- ステップ間の接続線描画（#726）
- プロパティパネル（#726）
- API 連携（保存・読込・公開）— 後続 Story
- ツールバーのアクションボタン（保存・公開）— 後続 Story
- 定義一覧ページ — 後続 Story

## 設計判断

### 1. モジュール構成

`Page/WorkflowDefinition/Designer.elm` に TEA のページモジュールを作成。#725 のスコープでは1ファイルで収まる規模（推定 ~350 行）のため、Canvas/Palette を別コンポーネントに分離しない。#726 でコードが増加した時点で抽出する。

データ型と純粋関数は `Data/DesignerCanvas.elm` に配置。テスタビリティの確保と、既存の Data/ パターンへの準拠。

### 2. ルート設計

`/workflow-definitions/new` のみ追加。編集ルート（`/workflow-definitions/{id}`）は API 連携 Story で追加する。

```elm
type Route
    = ...
    | WorkflowDefinitionDesignerNew  -- /workflow-definitions/new
```

### 3. Ports 設計

キャンバス SVG 要素の getBoundingClientRect を取得する専用 Port を追加。init 時に1回リクエストし、Bounds をキャッシュする。

```elm
-- Ports.elm
port requestCanvasBounds : String -> Cmd msg      -- SVG 要素の id
port receiveCanvasBounds : (Encode.Value -> msg) -> Sub msg
```

```javascript
// main.js
app.ports.requestCanvasBounds.subscribe((elementId) => {
  requestAnimationFrame(() => {
    const el = document.getElementById(elementId);
    if (el) {
      const rect = el.getBoundingClientRect();
      app.ports.receiveCanvasBounds.send({
        x: rect.x, y: rect.y, width: rect.width, height: rect.height
      });
    }
  });
});
```

### 4. 座標変換

マウスの clientX/clientY → SVG viewBox 座標への変換:

```
canvasX = (clientX - bounds.x) / bounds.width * viewBoxWidth
canvasY = (clientY - bounds.y) / bounds.height * viewBoxHeight
```

### 5. ステップ ID 生成

`stepType ++ "_" ++ String.fromInt nextStepNumber` 形式。nextStepNumber は Model で管理し、ステップ作成ごとにインクリメント。API 連携時に UUID に置き換え可能な設計。

### 6. グリッドスナップ

20px グリッドにスナップ。`snap x = toFloat (round (x / gridSize) * gridSize)`

## 主要な型定義

```elm
-- Data/DesignerCanvas.elm

type StepType = Start | Approval | End

type alias Position = { x : Float, y : Float }

type alias StepNode =
    { id : String
    , stepType : StepType
    , name : String
    , position : Position
    }

type alias Bounds = { x : Float, y : Float, width : Float, height : Float }

type DraggingState
    = DraggingExistingStep String Position  -- stepId, offset from step origin
    | DraggingNewStep StepType Position      -- from palette, current canvas position
```

```elm
-- Page/WorkflowDefinition/Designer.elm

type alias Model =
    { shared : Shared
    , steps : Dict String StepNode
    , selectedStepId : Maybe String
    , dragging : Maybe DraggingState
    , canvasBounds : Maybe Bounds
    , nextStepNumber : Int
    }

type Msg
    = PaletteMouseDown StepType
    | StepMouseDown String Float Float     -- stepId, offsetX, offsetY
    | CanvasMouseMove Float Float          -- clientX, clientY
    | CanvasMouseUp Float Float
    | StepClicked String
    | CanvasBackgroundClicked
    | KeyDown String
    | GotCanvasBounds Encode.Value
    | NoOp
```

## 変更対象ファイル

| ファイル | 操作 | 内容 |
|---------|------|------|
| `frontend/src/Data/DesignerCanvas.elm` | 新規 | StepType, StepNode, Position, Bounds, DraggingState, 純粋関数 |
| `frontend/src/Page/WorkflowDefinition/Designer.elm` | 新規 | デザイナーページ（Model, Msg, init, update, view, subscriptions, updateShared） |
| `frontend/src/Route.elm` | 変更 | WorkflowDefinitionDesignerNew ルート追加 |
| `frontend/src/Main.elm` | 変更 | DesignerPage 追加（Page, Msg, initPage, update, viewPage, subscriptions, updatePageShared） |
| `frontend/src/Ports.elm` | 変更 | requestCanvasBounds, receiveCanvasBounds 追加 |
| `frontend/src/main.js` | 変更 | requestCanvasBounds ハンドラ追加 |
| `frontend/tests/Data/DesignerCanvasTest.elm` | 新規 | データ型テスト |
| `frontend/tests/Page/WorkflowDefinition/DesignerTest.elm` | 新規 | ページ update ロジックテスト |
| `frontend/tests/RouteTest.elm` | 変更 | 新ルートのテスト追加 |

## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: データ型、ルート、ページスキャフォールド

キャンバスのデータ型を定義し、ルートとページを登録する。この Phase で Designer ページに空のキャンバス（グリッド線のみ）が表示される状態にする。

#### 確認事項
- [x] 型: Route 型のバリアント追加パターン → `Route.elm` L93, type Route にバリアント追加 + parser/toString/isRouteActive/pageTitle の 4 箇所
- [x] パターン: Main.elm のページ登録パターン → WorkflowNew を参照、7 integration points（Page, Msg, initPage, update, viewPage, subscriptions, updatePageShared）
- [x] パターン: Data/ モジュールの型定義パターン → `Data/WorkflowDefinition.elm` を参照、type alias + exposing リスト形式
- [x] ライブラリ: `Dict` の import パス → Grep 結果 20+ 箇所、`import Dict exposing (Dict)` パターン
- [x] パターン: SVG レンダリングパターン → `Component/Icons.elm` を参照、`Svg`, `Svg.Attributes as SvgAttr` import パターン

#### テストリスト

ユニットテスト:
- [ ] `StepType` の各バリアント（Start, Approval, End）を文字列に変換できる
- [ ] `snapToGrid` が 20px グリッドにスナップする（境界値: 0, 10, 19, 20, 30）
- [ ] `defaultStepName` が StepType に応じた日本語名を返す（開始/承認/終了）
- [ ] Route: `/workflow-definitions/new` が WorkflowDefinitionDesignerNew にマッチ
- [ ] Route: `toString WorkflowDefinitionDesignerNew` が正しい URL を生成

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし — Phase 4 完了後に開発サーバーで手動確認）

### Phase 2: SVG キャンバス描画とステップパレット

グリッド線の描画、ステップノードの SVG レンダリング（種別ごとの色・形状）、ステップパレット UI を実装する。この Phase でパレットの見た目とキャンバス上のステップ描画（ハードコードされたテストデータ）が動作する。

#### 確認事項
- [x] ライブラリ: `Svg.line`, `Svg.rect`, `Svg.text_`, `Svg.g` の引数パターン → `Component/Icons.elm` で使用確認、属性リスト + 子要素リストの 2 引数
- [x] ライブラリ: `Svg.Attributes.viewBox`, `Svg.Attributes.fill`, `Svg.Attributes.stroke` → Icons.elm で使用確認、String 引数
- [x] パターン: デザインガイドラインの色トークン → `13_デザインガイドライン.md` 参照、success/primary/secondary の 100/600 レベル
- [x] パターン: ステップ種別ごとの色設計 → `15_ワークフローデザイナー設計.md` 参照、Start=success, Approval=primary, End=secondary

#### テストリスト

ユニットテスト:
- [ ] `stepColor` が StepType に応じた色を返す（Start=緑系, Approval=青系, End=灰系）
- [ ] `stepDimensions` が固定サイズ（幅 120, 高さ 60）を返す

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 3: Ports とドラッグ&ドロップ（パレット → キャンバス）

Ports でキャンバス座標変換を実装し、パレットからキャンバスへのドラッグ&ドロップでステップを配置する機能を実装する。

#### 確認事項
- [x] パターン: 既存 Ports の定義パターン → `Ports.elm` L1 `port module` 宣言、Cmd msg / Sub msg シグネチャ、exposing リストにアルファベット順追加
- [x] パターン: main.js の Port ハンドラパターン → `main.js` L223-250、`if (app.ports.xxx)` ガード + subscribe パターン
- [x] ライブラリ: `Browser.Events.onMouseMove`, `Browser.Events.onMouseUp` → elm/browser ドキュメント確認、`Decode.Decoder msg -> Sub msg` シグネチャ
- [x] ライブラリ: `Json.Decode.field`, `Json.Decode.float` でマウスイベントをデコード → Grep 結果 10+ 箇所、`Decode.field "key" Decode.string` 等のパターン
- [x] パターン: subscriptions の Model 依存パターン → Main.elm L763 で `Designer.subscriptions subModel` と Model を渡す方式。既存ページ（`Sub Msg`）とは異なる

#### テストリスト

ユニットテスト:
- [ ] `clientToCanvas` がマウス座標を SVG 座標に正しく変換する
- [ ] `clientToCanvas` が Bounds 未取得時（Nothing）に Nothing を返す
- [ ] `createStepFromDrop` がパレットドロップからグリッドスナップされた StepNode を生成する
- [ ] `generateStepId` が stepType と番号から一意な ID を生成する（"start_1", "approval_2" 等）
- [ ] Port メッセージのデコーダが `{ x, y, width, height }` を Bounds にデコードする
- [ ] update: `PaletteMouseDown` で dragging が `DraggingNewStep` に遷移する
- [ ] update: `CanvasMouseUp` で DraggingNewStep 時に新しい StepNode が steps に追加される
- [ ] update: `CanvasMouseUp` で dragging が Nothing にリセットされる
- [ ] update: `CanvasMouseMove` で DraggingNewStep の位置が更新される

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### Phase 4: ステップ操作（選択・移動・削除）

配置済みステップのクリック選択、ドラッグ移動、Delete キー削除、背景クリック選択解除を実装する。

#### 確認事項
- [x] ライブラリ: `Browser.Events.onKeyDown` → elm/browser ドキュメント確認、`Decode.Decoder msg -> Sub msg` シグネチャ、`Decode.field "key" Decode.string` で文字列取得
- [x] パターン: SVG 要素の `onMouseDown` イベント → `Html.Events.stopPropagationOn` を使用、offsetX/offsetY は SVG で不正確なため clientX/clientY を採用（判断ログに記載）
- [x] パターン: 既存の keyboard event handling → プロジェクト初使用（Grep 結果 0 件）。elm/browser ドキュメントの `onKeyDown` パターンに従った

#### テストリスト

ユニットテスト:
- [ ] update: `StepClicked` で selectedStepId が設定される
- [ ] update: `CanvasBackgroundClicked` で selectedStepId が Nothing になる
- [ ] update: `StepMouseDown` で dragging が `DraggingExistingStep` に遷移する
- [ ] update: `CanvasMouseMove` で DraggingExistingStep 時にステップ位置が更新される（グリッドスナップ）
- [ ] update: `CanvasMouseUp` で DraggingExistingStep 時に dragging が Nothing になり位置が確定する
- [ ] update: `KeyDown "Delete"` で選択中のステップが steps から削除される
- [ ] update: `KeyDown "Delete"` で選択中ステップがない場合は何も起きない
- [ ] update: `KeyDown "Backspace"` でも選択中ステップが削除される
- [ ] 削除後に selectedStepId が Nothing になる

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし — 全 Phase 完了後に開発サーバーで手動確認）

## サイドバーナビゲーション

管理セクション（`viewAdminSection`）に「ワークフロー定義」リンクを追加する。`/workflow-definitions/new` に遷移する。Icons モジュールにデザイナー用アイコンを追加する。

## デザイン仕様

### キャンバスレイアウト

```
┌──────────────────────────────────────────────┐
│  ◀ 戻る          ワークフローデザイナー        │  ← ミニマルツールバー
├─────────┬────────────────────────────────────┤
│ パレット │                                    │
│         │        SVG キャンバス               │
│ ○ 開始  │     （グリッド + ステップ）         │
│ ✓ 承認  │                                    │
│ ◎ 終了  │                                    │
│         │                                    │
├─────────┴────────────────────────────────────┤
│  ステータスバー: N ステップ                    │
└──────────────────────────────────────────────┘
```

### ステップ描画仕様

| StepType | 色（bg/border） | 角丸 | サイズ |
|----------|----------------|------|--------|
| Start | success-100/success-600 | rounded（8px） | 120×60 |
| Approval | primary-100/primary-600 | rounded（8px） | 120×60 |
| End | secondary-100/secondary-600 | rounded（8px） | 120×60 |

選択中: border を 2px → 3px、`ring-2 ring-primary-500` 相当の強調枠。

### SVG viewBox

`0 0 1200 800`。キャンバス全体を固定サイズで表示し、レスポンシブに `width="100%" height="100%"` でスケーリング。

### グリッド

20px 間隔の薄い線（secondary-200, stroke-width: 0.5）。

## 検証方法

1. `cd frontend && pnpm run test` — ユニットテスト全パス
2. `just check-all` — lint + test + API test + E2E test 全パス
3. 手動確認（`just dev-all` で開発サーバー起動）:
   - `/workflow-definitions/new` にアクセスしてデザイナー画面が表示される
   - パレットからステップをキャンバスにドラッグ&ドロップして配置できる
   - 配置済みステップをクリックして選択（青枠ハイライト）できる
   - 選択ステップをドラッグして移動できる
   - Delete/Backspace キーで選択ステップを削除できる
   - キャンバスの空白クリックで選択解除される
   - ステップがグリッドにスナップして配置される

### ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | 編集ルート（`/workflow-definitions/{id}`）を含めるべきか | スコープ境界 | #725 は new のみ。API 連携がないため edit ルートは後続 Story で追加 |
| 2回目 | Canvas/Palette を別コンポーネントに分離すべきか | シンプルさ | #725 は ~350行推定で1ファイルに収まる。#726 でコード増加時に抽出 |
| 3回目 | Ports を既存の sendMessage/receiveMessage で実現するか専用 Port か | 既存手段の見落とし | 専用 Port を採用。sendMessage は未使用で汎用メッセージング基盤が不要。専用 Port が明示的で保守しやすい |
| 4回目 | subscriptions の型シグネチャが既存パターン（`Sub Msg`）と異なる（`Model -> Sub Msg`） | アーキテクチャ不整合 | Main.elm の subscriptions ルーティングで `Designer.subscriptions subModel` と Model を渡す。条件付き subscription は dragging 状態に依存するため必要。既存の Sub Msg ページには影響なし |
| 5回目 | サイドバーにワークフロー定義へのナビゲーションが必要 | 未定義 | 管理セクションに「ワークフロー定義」リンクを追加。Phase 1 のスキャフォールドで対応 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準が全て Phase に割り当てられている | OK | D&D配置→Phase3、パレット→Phase2、選択/移動/削除→Phase4、Ports→Phase3、check-all→Phase4 後 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 型定義、色仕様、サイズ、グリッド間隔が具体値で記載 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | モジュール構成、ルート設計、Ports 方式、座標変換、ID 生成の 5 判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象 6 項目、対象外 5 項目を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | SVG viewBox のスケーリング、getBoundingClientRect の仕様、Browser.Events のグローバル subscription 動作 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | ADR-053（SVG+Elm D&D）、詳細設計書15（型定義、レイヤー構成）、機能仕様書04（ステップ種別、操作）と照合 |
