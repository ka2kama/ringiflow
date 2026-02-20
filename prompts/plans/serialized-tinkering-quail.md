# #718 Api モジュールの Decode.field "data" インライン使用を Data モジュールのデコーダに統一する

## Context

`frontend.md` にデコーダ命名規約（`decoder` / `detailDecoder` / `listDecoder`）を追加済みだが、`Api/Workflow.elm` に 8 箇所の `Decode.field "data"` インライン使用が残っている。規約に従い、`"data"` ラッパーのデコード責務を Data モジュールに移動する。

## スコープ

対象:
- `Data/WorkflowInstance.elm` — `detailDecoder` 追加
- `Data/WorkflowComment.elm` — `detailDecoder` 追加
- `Api/Workflow.elm` — インライン `Decode.field "data"` 8 箇所を Data モジュールのデコーダに置換
- テスト追加: `WorkflowInstanceTest.elm`, `WorkflowCommentTest.elm`

対象外:
- `Api/Auth.elm` — `User` 型が `Shared` モジュールに定義されており、Data モジュールパターンの直接適用が不適切。別 Issue で対応

## Phase 1: Data モジュールに `detailDecoder` を追加 + テスト

### 確認事項
- [x] 型: `WorkflowDefinition.detailDecoder` のパターン → `frontend/src/Data/WorkflowDefinition.elm` L84-86: `Decode.field "data" decoder`
- [x] パターン: `listDecoder` テストのパターン → `frontend/tests/Data/WorkflowInstanceTest.elm` L466-528, `frontend/tests/Data/WorkflowCommentTest.elm` L79-129

### テストリスト

ユニットテスト:
- [ ] `WorkflowInstance.detailDecoder`: data フィールドから単一インスタンスをデコード
- [ ] `WorkflowInstance.detailDecoder`: data フィールドがない場合はエラー
- [ ] `WorkflowComment.detailDecoder`: data フィールドから単一コメントをデコード
- [ ] `WorkflowComment.detailDecoder`: data フィールドがない場合はエラー

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### 変更内容

#### `Data/WorkflowInstance.elm`

```elm
-- exposing に detailDecoder を追加

{-| 単一のワークフローインスタンスレスポンスをデコード

API レスポンスの `{ data: {...} }` 形式に対応。

-}
detailDecoder : Decoder WorkflowInstance
detailDecoder =
    Decode.field "data" decoder
```

#### `Data/WorkflowComment.elm`

```elm
-- exposing に detailDecoder を追加

{-| 単一のコメントレスポンスをデコード

API レスポンスの `{ data: {...} }` 形式に対応。

-}
detailDecoder : Decoder WorkflowComment
detailDecoder =
    Decode.field "data" decoder
```

## Phase 2: `Api/Workflow.elm` のインライン使用を置換

### 確認事項

確認事項: なし（Phase 1 で追加したデコーダを使用するのみ）

### テストリスト

ユニットテスト（該当なし — 振る舞いの変更なし、内部リファクタリングのみ）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### 変更内容

`Api/Workflow.elm` の 8 箇所を置換:

| 関数/デコーダ | 行 | 変更前 | 変更後 |
|---|---|---|---|
| `getWorkflow` | L97 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |
| `approveStep` | L177 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |
| `rejectStep` | L208 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |
| `requestChangesStep` | L239 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |
| `resubmitWorkflow` | L264 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |
| `postComment` | L310 | `Decode.field "data" WorkflowComment.decoder` | `WorkflowComment.detailDecoder` |
| `createResponseDecoder` | L439 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |
| `submitResponseDecoder` | L449 | `Decode.field "data" WorkflowInstance.decoder` | `WorkflowInstance.detailDecoder` |

置換後、`createResponseDecoder` と `submitResponseDecoder` は `WorkflowInstance.detailDecoder` のエイリアスになる。これらは Api/Workflow.elm 内でのみ使われており（`createWorkflow`, `submitWorkflow` から参照）、Refactor ステップで削除してインラインに `WorkflowInstance.detailDecoder` を使う方が簡潔。

さらに、`Json.Decode as Decode` の import から未使用の `Decode` を除去する（`Decode.field` が不要になるため）。ただし `Decode` の他の用途（`Decode.at` 等）がないか確認する。→ `Decode` は `exposing (Decoder)` で型注釈に使用しているため、import 自体は残すが `Decode.field` の直接使用がなくなる。

## 完了基準の検証

```bash
# Api モジュール内に Decode.field "data" がゼロであることを確認
grep -r 'Decode.field "data"' frontend/src/Api/

# テスト通過
just check
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `createResponseDecoder` / `submitResponseDecoder` が置換後にただのエイリアスになる | シンプルさ | Phase 2 の Refactor で削除し、直接 `WorkflowInstance.detailDecoder` を使用する方針を追記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Grep 結果 8 箇所（Api/Workflow.elm）すべてが Phase 2 の変更一覧に含まれている。Api/Auth.elm はスコープ外として明記 |
| 2 | 曖昧さ排除 | OK | 各箇所の行番号・変更前後を明記。曖昧表現なし |
| 3 | 設計判断の完結性 | OK | `createResponseDecoder` / `submitResponseDecoder` の削除判断を記載 |
| 4 | スコープ境界 | OK | 対象（Api/Workflow.elm + Data 2 ファイル）と対象外（Api/Auth.elm）を明記 |
| 5 | 技術的前提 | OK | 既存パターン（WorkflowDefinition.detailDecoder, Task.detailDecoder）で実証済み |
| 6 | 既存ドキュメント整合 | OK | `frontend.md` のデコーダ命名規約に準拠 |
