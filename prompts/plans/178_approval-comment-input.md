# Issue #178: 申請詳細ページの承認セクションにコメント入力欄を追加

## 概要

`Page/Workflow/Detail.elm` の承認/却下セクションに、`Page/Task/Detail.elm` と同等のコメント入力欄を追加する。
API は既に `comment` パラメータをサポートしており、フロントエンドのみの変更。

## 変更ファイル

- `frontend/src/Page/Workflow/Detail.elm` （唯一の変更対象）

## 参照ファイル

- `frontend/src/Page/Task/Detail.elm` （参考実装）

## 設計判断

1. **`nonEmptyComment` の配置**: 複製する（プロジェクト方針「3回繰り返すまでは重複許容」に従う）
2. **View 関数のシグネチャ**: 個別フィールドを渡す方式（既存スタイルと一貫）
3. **テスト**: Elm のページモジュールテストは未導入のため、手動 E2E 確認
4. **設計フェーズ**: 簡略化（既存パターンの踏襲）

## 実装ステップ

### Step 1: Model に `comment` フィールドを追加

- `Model` 型に `comment : String` を追加（80行目、`isSubmitting` の前）
- `init` 関数に `comment = ""` を追加（95行目付近）

### Step 2: Msg に `UpdateComment` を追加

- `Msg` 型に `| UpdateComment String` を追加（128行目、`Refresh` の後）

### Step 3: update 関数を拡張

- `UpdateComment` ハンドラを追加:
  ```elm
  UpdateComment newComment ->
      ( { model | comment = newComment }, Cmd.none )
  ```

### Step 4: `nonEmptyComment` ヘルパーを追加

- `handleApprovalResult` の近くに追加:
  ```elm
  nonEmptyComment : String -> Maybe String
  nonEmptyComment comment =
      if String.isEmpty (String.trim comment) then Nothing
      else Just (String.trim comment)
  ```

### Step 5: `ConfirmAction` の `comment = Nothing` を置換

- 203行目: `comment = Nothing` → `comment = nonEmptyComment model.comment`
- 214行目: 同上

### Step 6: `handleApprovalResult` で成功時に comment をクリア

- 成功ケースに `comment = ""` を追加
- エラーケースは comment を保持（リトライ可能にするため）

### Step 7: import に `onInput` を追加

- 44行目: `Html.Events exposing (onClick)` → `Html.Events exposing (onClick, onInput)`

### Step 8: View をリファクタリング

既存の `viewApprovalButtons` を3つの関数に分割:

```elm
-- 新: 承認セクション全体（ステップ検索 + コメント + ボタン）
viewApprovalSection : WorkflowInstance -> String -> Bool -> Shared -> Html Msg

-- 新: コメント入力欄（Task Detail と同じ）
viewCommentInput : String -> Html Msg

-- リネーム: ボタンのみ（引数変更: WorkflowStep -> Bool -> Html Msg）
viewApprovalButtons : WorkflowStep -> Bool -> Html Msg
```

`viewWorkflowDetail` の呼び出しを更新:
- シグネチャに `String`（comment）を追加
- `viewContent` から `model.comment` を渡す

## 検証方法

```bash
just check-all  # lint + test（コンパイル通過を確認）
just dev-all    # 開発サーバー起動後、手動テスト
```

手動テスト項目:
- [ ] 申請詳細画面でコメント入力欄が表示されること
- [ ] コメントを入力して承認できること
- [ ] コメントを入力して却下できること
- [ ] コメントが空でも承認/却下できること
- [ ] 承認/却下成功後にコメント欄がクリアされること
- [ ] エラー時にコメント内容が保持されること
