# #1032 FileUpload コンポーネントの同名ファイル衝突を解消する

## コンテキスト

### 目的
- Issue: #1032
- Want: 同名ファイルを複数選択した場合に、それぞれが独立してアップロード・削除・進捗管理されること
- 完了基準:
  - `UploadingFile` にユニーク ID フィールドを追加
  - ファイル操作（追加、削除、URL 取得、進捗更新）で ID ベースの識別に変更
  - 同名ファイルを複数追加・個別削除できるテストを追加

### ブランチ / PR
- ブランチ: `feature/1032-fileupload-unique-id`
- PR: #1033（Draft）

### As-Is（探索結果の要約）

対象ファイル: `frontend/src/Component/FileUpload.elm`

現在の問題箇所:
- `UploadingFile` に `name : String` フィールドがあり、ファイル名で識別
- `Msg` 型: `GotUploadUrl String`, `RemoveFile String` で fileName を識別子に使用
- `GotUploadUrl` ハンドラ: `f.name == fileName` で全同名ファイルに documentId を設定
- `uploadCmd`: `List.head` で最初の 1 つのみアップロード
- `RemoveFile` ハンドラ: `f.name /= fileName` で全同名ファイルを削除
- `updateFileProgress`: `f.name == fileName` で全同名ファイルの進捗を更新
- `addFiles`: `GotUploadUrl (File.name f)` で fileName をキーに Msg を生成
- `startPendingUploads`: `GotUploadUrl f.name` で同様

外部利用状況:
- 親ページは `FileUpload.Model`, `FileUpload.Msg`（opaque）, `FileUpload.update`, `FileUpload.view` を利用
- `UploadingFile` は `type alias` で expose されているが、外部で直接構築・フィールドアクセスなし
- 変更は `Component/FileUpload.elm` 内部に閉じる

テスト: `frontend/tests/Component/FileUploadTest.elm` にはバリデーションテストのみ存在

### 進捗
- [x] Phase 1: ユニーク ID の導入とテスト

## Phase 1: ユニーク ID の導入

### 設計判断

1. ID の生成方式: カウンタベース（`nextId : Int`）
   - 代替案: UUID → Elm では Random が必要で Cmd が発生、コンポーネント内の識別にはオーバースペック
   - 理由: 単一セッション内の一意性で十分。シンプルかつ決定的

2. 変更する Msg バリアント:
   - `GotUploadUrl String → GotUploadUrl Int` (fileName → fileId)
   - `RemoveFile String → RemoveFile Int` (fileName → fileId)
   - `GotUploadProgress`, `UploadCompleted`, `ConfirmCompleted` は既に `documentId`（String）で識別しており変更不要

3. `updateFileProgress` の廃止:
   - fileName ベースの `updateFileProgress` は `GotUploadUrl` の Err 分岐でのみ使用
   - ID ベースの新関数 `updateFileProgressById` に置き換え

### 対象外
- 外部モジュール（Page/Workflow/New/ 等）の変更（不要）
- `UploadingFile` の expose 変更（後方互換のため維持）

### 確認事項
- 型: `UploadingFile` の現在のフィールド → `frontend/src/Component/FileUpload.elm:91-97`
- 型: `Msg` の現在のバリアント → `frontend/src/Component/FileUpload.elm:112-122`
- パターン: `addFiles` での UploadingFile 構築 → `frontend/src/Component/FileUpload.elm:353-361`
- パターン: `startPendingUploads` での GotUploadUrl 生成 → `frontend/src/Component/FileUpload.elm:419-429`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 異なるフォルダから同名ファイルを複数追加し、それぞれが独立してリストに表示される | 正常系 | ユニットテスト |
| 2 | 同名ファイルが複数ある状態で特定の 1 つを削除し、他は残る | 正常系 | ユニットテスト |
| 3 | 同名ファイルが複数ある状態で URL 取得結果が正しいファイルのみに反映される | 正常系 | ユニットテスト |
| 4 | 同名ファイルが複数ある状態で URL 取得失敗が正しいファイルのみに反映される | 準正常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] 同名ファイルを複数追加した場合、それぞれ異なる ID が付与される
- [ ] RemoveFile で特定 ID のファイルのみ削除され、同名の他ファイルは残る
- [ ] GotUploadUrl 成功時に対象 ID のファイルのみ documentId が設定される
- [ ] GotUploadUrl 失敗時に対象 ID のファイルのみ Failed になる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 変更内容

#### Model の変更
```elm
type alias Model =
    { files : List UploadingFile
    , dragOver : Bool
    , config : FileConfig
    , workflowInstanceId : Maybe String
    , nextId : Int  -- 追加
    }
```

#### UploadingFile の変更
```elm
type alias UploadingFile =
    { id : Int  -- 追加
    , file : File
    , documentId : Maybe String
    , name : String
    , size : Int
    , progress : UploadProgress
    }
```

#### Msg の変更
```elm
type Msg
    = ...
    | GotUploadUrl Int (Result ApiError UploadUrlResponse)  -- String → Int
    | RemoveFile Int  -- String → Int
```

#### 内部関数の変更
- `addFiles`: `nextId` を使って各ファイルに連番 ID を付与、`GotUploadUrl fileId` を生成
- `startPendingUploads`: `GotUploadUrl f.id` に変更
- `GotUploadUrl` ハンドラ: `f.id == fileId` で検索
- `RemoveFile` ハンドラ: `f.id /= fileId` でフィルタ
- `updateFileProgress` → `updateFileProgressById`: `f.id == fileId` で検索
- `viewFileItem`: `RemoveFile file.id` に変更

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `startPendingUploads` も fileName ベースで GotUploadUrl を生成 | 不完全なパス | 変更内容に `startPendingUploads` の修正を追加 |
| 2回目 | `updateFileProgress` が GotUploadUrl Err でのみ使用されることを確認 | 既存手段の見落とし | 関数を廃止して `updateFileProgressById` に統一 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | fileName で識別している全箇所が計画に含まれている | OK | GotUploadUrl, RemoveFile, updateFileProgress, addFiles, startPendingUploads, viewFileItem すべて列挙済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 全変更箇所にコードスニペットあり |
| 3 | 設計判断の完結性 | ID 生成方式が確定 | OK | カウンタベース、理由と代替案あり |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象: FileUpload.elm、対象外: 外部モジュール |
| 5 | 技術的前提 | Elm の型システムの制約を考慮 | OK | Msg が opaque のため外部影響なし |
| 6 | 既存ドキュメント整合 | 矛盾なし | OK | 新規コンポーネントの内部改善、設計書への影響なし |
