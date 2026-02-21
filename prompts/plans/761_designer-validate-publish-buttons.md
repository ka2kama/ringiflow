# #761 ワークフローデザイナーの検証・公開ボタン修正

## Context

ワークフローデザイナー画面の「検証」ボタンと「公開」ボタンが動作しない。
- 検証: API リクエストボディに `definition` ラッパーが欠落し、デシリアライズエラー
- 公開: ダイアログ ID のハードコード（`"designer-publish-dialog"`）が `ConfirmDialog.dialogId`（`"confirm-dialog"`）と不一致で、確認ダイアログが表示されない

バックエンド側は正しく実装されており、フロントエンドのみの修正。

## 対象・対象外

対象:
- Designer.elm の検証・公開ボタンの修正
- `encodeValidationRequest` エンコード関数の追加
- Elm ユニットテスト追加
- E2E テスト追加

対象外:
- バックエンド側の変更（不要）
- 一覧画面の公開（正常動作済み）

## Phase 1: フロントエンド修正

### 修正対象ファイル

| # | ファイル | 修正内容 |
|---|---------|---------|
| 1 | `frontend/src/Data/WorkflowDefinition.elm` | `encodeValidationRequest` 関数を追加、exposing に追加 |
| 2 | `frontend/src/Page/WorkflowDefinition/Designer.elm` | (a) ValidateClicked で `encodeValidationRequest` を使用、(b) ConfirmPublish で同様、(c) PublishClicked のダイアログ ID を `ConfirmDialog.dialogId` に修正 |

### 設計

`encodeValidationRequest` は既存の `encodeUpdateRequest` パターンに倣う:

```elm
encodeValidationRequest : { definition : Encode.Value } -> Encode.Value
encodeValidationRequest { definition } =
    Encode.object
        [ ( "definition", definition )
        ]
```

Designer.elm の修正箇所:

1. ValidateClicked（行 476-479）: `body = definition` → `body = WorkflowDefinition.encodeValidationRequest { definition = definition }`
2. ConfirmPublish の else ブランチ（行 556-559）: 同上
3. PublishClicked（行 522）: `"designer-publish-dialog"` → `ConfirmDialog.dialogId`

### 確認事項

- 型: `encodeValidationRequest` のシグネチャ → `Data/WorkflowDefinition.elm` の既存パターン
- パターン: `encodeUpdateRequest` のエンコードパターン → `Data/WorkflowDefinition.elm:189-196`
- パターン: `ConfirmDialog.dialogId` の使い方 → `Page/WorkflowDefinition/List.elm:215`（正しい使用例）

### テストリスト

ユニットテスト:
- [ ] `encodeValidationRequest` が `definition` フィールドでラップすること

ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト:
- [ ] デザイナー画面で検証ボタンをクリックすると検証結果が表示されること
- [ ] デザイナー画面で公開ボタンをクリックすると確認ダイアログが表示され、公開できること

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ConfirmPublish の else ブランチ（行 556-559）も同じ validateDefinition 呼び出しで definition ラッパーが欠落 | 不完全なパス | Phase 1 の修正箇所に追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全修正箇所が計画に含まれている | OK | ValidateClicked, ConfirmPublish(else), PublishClicked の 3 箇所を特定済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各修正の具体的なコード変更を記載済み |
| 3 | 設計判断の完結性 | 全ての判断が記載されている | OK | encodeValidationRequest の配置場所・シグネチャを決定済み |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | 対象・対象外セクションに記載 |
| 5 | 技術的前提 | 前提が考慮されている | OK | バックエンド側の ValidateDefinitionRequest の構造を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 既存パターン（encodeUpdateRequest）に準拠 |

## 検証

1. `just fmt` でフォーマット確認
2. `just check-all` で全テスト通過
3. 開発サーバーで手動確認: デザイナー画面の検証・公開ボタンが動作すること
