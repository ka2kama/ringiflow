# ApproverSelection 型リファクタリング（#492）

## 概要

`ApproverSelection` 型に `Preselected UserRef` バリアントを追加し、再提出モード開始時のダミー値を型レベルで排除した。Elm の "make impossible states impossible" 原則に基づく型設計の改善。

## 実施内容

### Phase 1: ApproverSelector の型変更 + ヘルパー追加

- `ApproverSelection` を 2 バリアント（`NotSelected | Selected UserItem`）から 3 バリアント（`NotSelected | Preselected UserRef | Selected UserItem`）に拡張
- `selectedUserId : ApproverSelection -> Maybe String` ヘルパー関数を追加
- `viewSelectedApprover` のシグネチャを `UserItem -> msg -> Html msg` から `String -> Maybe String -> msg -> Html msg` に変更
- `Preselected` 時は名前のみ表示、`Selected` 時は名前 + displayId を表示
- `selectedUserId` の 3 バリアントに対するユニットテストを追加

### Phase 2: Detail.elm のバグ修正 + New.elm のパターンマッチ対応

- `Detail.elm` の `StartEditing` でダミー値 `Selected { id = ref.id, name = ref.name, displayNumber = 0, displayId = "", email = "" }` を `Preselected ref` に置換
- `Detail.elm` の `validateResubmit` / `buildResubmitApprovers` を `selectedUserId` で簡潔化
- `New.elm` の `validateFormWithApprover` / `buildApprovers` も同様に `selectedUserId` で簡潔化

## 判断ログ

- 設計判断: 3 バリアント方式を採用。代替案として `Selected { id, name, detail : Maybe {...} }` を検討したが、Maybe のハンドリングが煩雑で不正な組み合わせを型で防げないため不採用
- Refactor: `selectedUserId` ヘルパーにより `buildApprovers`/`validate*` の 3 パターン分岐を `Maybe.map` に集約。DRY に寄与することを確認
- Refactor: `viewSelectedApprover` のシグネチャ変更で将来の拡張余地（email 表示等）が失われるが、YAGNI に従い現状で十分と判断

## 成果物

コミット:
- `cb685be` — `#492 Replace dummy UserItem values with Preselected UserRef variant`（実装 + テスト、4 ファイル）
- `c88ce6f` — `#492 Add plan file for ApproverSelection type redesign`（計画ファイル）

変更ファイル:
- `frontend/src/Component/ApproverSelector.elm` — 型定義 + ヘルパー + ビュー
- `frontend/src/Page/Workflow/Detail.elm` — バグ修正
- `frontend/src/Page/Workflow/New.elm` — パターンマッチ対応
- `frontend/tests/Component/ApproverSelectorTest.elm` — テスト追加

PR: #591
