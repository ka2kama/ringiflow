# ワークフロー申請のファイル添付を実装する

## コンテキスト

### 目的
- Issue: #884
- Want: ワークフロー申請時にファイルを添付でき、承認者がダウンロードできる体験を提供する
- 完了基準:
  - ワークフロー定義にファイル添付フィールドを追加できる
  - 申請フォームでファイルをドラッグ&ドロップまたはファイル選択で添付できる
  - アップロード進捗バーが表示される
  - 承認者がワークフロー詳細画面で添付ファイルをダウンロードできる

### ブランチ / PR
- ブランチ: `feature/884-workflow-file-attachment`
- PR: #1031（Draft）

### As-Is（探索結果の要約）
- ファイルアップロード API 完成済み（#881）: Presigned URL 方式、`POST /api/v1/documents/upload-url` → S3 PUT → `POST /api/v1/documents/{id}/confirm`
- ファイルダウンロード/削除 API 完成済み（#882）: `POST /api/v1/documents/{id}/download-url`, `DELETE /api/v1/documents/{id}`
- ワークフロー添付ファイル一覧 API 完成済み: `GET /api/v1/workflows/{id}/attachments`
- documents テーブル: `workflow_instance_id` カラムで申請と関連付け（folder_id との XOR 制約）
- ドメインモデル: `UploadContext::Workflow(WorkflowInstanceId)`, `FileValidation`, `S3KeyGenerator` 実装済み
- バックエンド未実装: `definition_validator.rs` の `valid_types` に `"file"` なし（現在: text/textarea/number/select/date）
- フロントエンド: `Data/FormField.elm` に `File` バリアントあり、デコーダーも `"file" -> Decode.succeed File` あり
- フロントエンド未実装: `DynamicForm.elm` の `viewFileInput` はプレースホルダ（「準備中」表示）
- フロントエンド未実装: `Api/Document.elm`, `Data/Document.elm`, `Component/FileUpload.elm` は存在しない
- `elm/file` 1.0.5 は直接依存にあり（`File.Select`, `File.name`, `File.size`, `File.mime` 利用可能）
- `elm/http` 2.0.0 で `Http.track` による進捗追跡可能（subscriptions 必要）
- Workflow New ページは現在 subscriptions なし（`Main.elm` の `_ -> Sub.none` に該当）
- 詳細設計書: `docs/40_詳細設計書/17_ドキュメント管理設計.md`（FileUpload コンポーネント設計、アップロードフロー詳細あり）

### 進捗
- [x] Phase 1: バックエンド — definition_validator に file フィールド型追加
- [x] Phase 2: フロントエンド — Data/Document.elm + Api/Document.elm
- [x] Phase 3: フロントエンド — Component/FileUpload.elm
- [x] Phase 4: フロントエンド — 申請フォームへの FileUpload 統合
- [x] Phase 5: フロントエンド — 承認者の添付ファイル表示とダウンロード

## 設計判断

### 1. ファイルアップロードのタイミング

| 選択肢 | 説明 |
|--------|------|
| **下書き保存後にアップロード（採用）** | ファイル選択 → 下書き保存（workflow_instance_id 取得）→ アップロード |
| ファイル選択時に即座にアップロード | 一時的なコンテキストでアップロード → 後で関連付け |

採用理由:
- 既存 API が `workflow_instance_id` をアップロード時に要求する設計
- API 変更不要、既存のドメインモデル（`UploadContext::Workflow`）をそのまま活用
- 未保存の下書きにファイルが孤立するリスクがない

UX 対応: ファイル選択後、まだ保存されていない場合は「保存後にアップロードされます」と表示。保存済みの場合は即座にアップロード開始。

### 2. フォームデータとファイルの分離

| 選択肢 | 説明 |
|--------|------|
| **分離管理（採用）** | formValues（Dict String String）にはファイル情報を含めず、FileUpload コンポーネントが独立管理 |
| formValues に document_id を格納 | ファイル情報を文字列として formValues に含める |

採用理由:
- ファイルアップロードは非同期・多段階（URL 取得 → PUT → 確認）で、単一文字列では状態管理が不十分
- FileUpload コンポーネントが独自の状態（進捗、エラー）を持つ必要がある
- バリデーション（required チェック）は FileUpload の状態から完了ファイル数を参照

### 3. Elm ファイルアップロード方式

| 選択肢 | 説明 |
|--------|------|
| **elm/file + Http.request（採用）** | TEA 内で完結。Ports 不要 |
| Ports + JavaScript | JavaScript で XMLHttpRequest を使用 |

採用理由:
- 詳細設計書の設計判断 #5 に準拠
- `File.Select.files` でファイル選択、`Http.request` で Presigned URL への PUT が可能
- `Http.track` で進捗追跡が可能（`Sub` 経由）
- ドラッグ&ドロップは `Browser.Events` ではなく、Elm の `Html.Events` カスタムイベント（`hijackOn`）で実現

### 4. FieldType の File バリアント拡張

| 選択肢 | 説明 |
|--------|------|
| **File に設定情報を持たせる（採用）** | `File FileConfig` として maxFiles, maxFileSize, allowedTypes を保持 |
| File のまま、デフォルト値を使用 | コンポーネント側でハードコードされたデフォルト値を使用 |

採用理由:
- ワークフロー定義ごとにファイル制限をカスタマイズ可能（詳細設計書のスキーマ拡張仕様に準拠）
- デコーダーで JSON からファイル設定を読み取り、FileUpload コンポーネントに渡す
- バリデーションの責任がスキーマ定義に帰属し、フロントエンドは設定に従うだけ

## Phase 1: バックエンド — definition_validator に file フィールド型追加

### 対象
- `backend/crates/domain/src/workflow/definition_validator.rs`

### 対象外
- ファイルアップロード API（#881 で実装済み）
- ドキュメントドメインモデル（#881 で実装済み）

### 確認事項
- 型: `ValidationError`, `ValidationResult` → `definition_validator.rs`
- パターン: select フィールドの固有バリデーション（options チェック）→ `definition_validator.rs:412-423`
- パターン: テストの構造 → `definition_validator.rs:427+`（`valid_definition()` ヘルパー）

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | デザイナーがワークフロー定義に file フィールドを含めて公開する | 正常系 | ユニットテスト |
| 2 | file フィールドに maxFiles/maxFileSize/allowedTypes を設定する | 正常系 | ユニットテスト |
| 3 | file フィールドに不正な maxFiles（0 や負数）を設定する | 準正常系 | ユニットテスト |
| 4 | file フィールドに不正な allowedTypes を設定する | 準正常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] file タイプのフィールドがバリデーションを通過する（正常系）
- [ ] file フィールドに maxFiles, maxFileSize, allowedTypes を設定した定義がバリデーションを通過する
- [ ] file フィールドの maxFiles が 0 以下の場合にエラー
- [ ] file フィールドの maxFileSize が 0 以下の場合にエラー
- [ ] file フィールドの allowedTypes に無効な Content-Type がある場合にエラー
- [ ] file フィールドと他のフィールド型（text, select 等）が混在する定義がバリデーションを通過する

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装内容

1. `valid_types` に `"file"` を追加
2. file 固有のバリデーションを追加:
   - `maxFiles`: 任意。指定時は 1 以上の整数（デフォルト: 10）
   - `maxFileSize`: 任意。指定時は 1 以上の整数（デフォルト: 20MB）
   - `allowedTypes`: 任意。指定時は `FileValidation::ALLOWED_CONTENT_TYPES` のサブセット

## Phase 2: フロントエンド — Data/Document.elm + Api/Document.elm

### 対象
- `frontend/src/Data/Document.elm`（新規）
- `frontend/src/Api/Document.elm`（新規）
- `frontend/src/Data/FormField.elm`（FieldType 拡張）

### 確認事項
- パターン: 既存の Data モジュール（`Data/WorkflowInstance.elm` 等）のデコーダーパターン
- パターン: 既存の Api モジュール（`Api/Workflow.elm` 等）のリクエストパターン
- ライブラリ: `elm/file` の `File` 型 → `File.name`, `File.size`, `File.mime`
- ライブラリ: `elm/http` の `Http.request`, `Http.track` → Grep 既存使用

### 操作パス

該当なし（データ層・API 層のみ）

### テストリスト

ユニットテスト:
- [ ] Document デコーダーが正常な JSON をデコードできる
- [ ] UploadUrlResponse デコーダーが正常な JSON をデコードできる
- [ ] DownloadUrlResponse デコーダーが正常な JSON をデコードできる
- [ ] FileConfig デコーダーがデフォルト値で動作する（maxFiles 等が省略された場合）
- [ ] FileConfig デコーダーがカスタム値を読み取れる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装内容

#### Data/Document.elm
```elm
type alias Document =
    { id : String
    , filename : String
    , contentType : String
    , size : Int
    , status : String
    , createdAt : String
    }

type alias UploadUrlResponse =
    { documentId : String
    , uploadUrl : String
    , expiresIn : Int
    }

type alias DownloadUrlResponse =
    { downloadUrl : String
    , expiresIn : Int
    }
```

#### Api/Document.elm
```elm
requestUploadUrl : { config, body, toMsg } -> Cmd msg
confirmUpload : { config, documentId, toMsg } -> Cmd msg
requestDownloadUrl : { config, documentId, toMsg } -> Cmd msg
listWorkflowAttachments : { config, workflowInstanceId, toMsg } -> Cmd msg
uploadToS3 : { uploadUrl, file, trackerId, toMsg } -> Cmd msg
```

#### Data/FormField.elm 拡張
```elm
type FieldType
    = Text
    | Number
    | Select (List SelectOption)
    | Date
    | File FileConfig

type alias FileConfig =
    { maxFiles : Int          -- デフォルト: 10
    , maxFileSize : Int       -- デフォルト: 20MB
    , allowedTypes : List String  -- デフォルト: 全対応形式
    }
```

## Phase 3: フロントエンド — Component/FileUpload.elm

### 対象
- `frontend/src/Component/FileUpload.elm`（新規）

### 確認事項
- パターン: 既存の Component（`Component/ApproverSelector.elm` 等）の Model/Msg/update/view パターン
- ライブラリ: `File.Select.files` → docs: package.elm-lang.org/packages/elm/file/latest/File-Select
- ライブラリ: `Http.track` → docs: package.elm-lang.org/packages/elm/http/latest/Http#track
- ライブラリ: D&D の `hijackOn` パターン → docs: package.elm-lang.org/packages/elm/file/latest/File#extracting-file-content の "drag and drop" セクション
- パターン: デザインガイドライン → `docs/40_詳細設計書/13_デザインガイドライン.md`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーがファイル選択ボタンでファイルを選択し、進捗バーが表示され、アップロードが完了する | 正常系 | E2E |
| 2 | ユーザーがファイルをドラッグ&ドロップで添付する | 正常系 | E2E |
| 3 | ユーザーが許可されていないファイル形式を選択する | 準正常系 | ユニットテスト |
| 4 | ユーザーがサイズ上限を超えるファイルを選択する | 準正常系 | ユニットテスト |
| 5 | ユーザーがファイル数上限を超えて選択する | 準正常系 | ユニットテスト |
| 6 | アップロード中にネットワークエラーが発生する | 異常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] validateFile が許可された Content-Type のファイルを通過させる
- [ ] validateFile が許可されていない Content-Type を拒否する
- [ ] validateFile がサイズ上限超過を拒否する
- [ ] validateFiles がファイル数上限超過を拒否する（既存ファイル + 新規ファイルの合計）
- [ ] UploadProgress の状態遷移（RequestingUrl → Uploading → Confirming → Completed）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト: Phase 5 で統合実施

### 実装内容

詳細設計書（`docs/40_詳細設計書/17_ドキュメント管理設計.md`）の FileUpload コンポーネント設計に準拠:

```elm
-- Model
type alias Model =
    { files : List UploadingFile
    , dragOver : Bool
    , config : FileConfig
    , workflowInstanceId : Maybe String  -- Nothing = 未保存（ファイル選択のみ）
    }

type alias UploadingFile =
    { file : File              -- elm/file の File 型（未アップロード時）
    , documentId : Maybe String -- サーバーから返却（アップロード URL 取得後）
    , name : String
    , size : Int
    , progress : UploadProgress
    }

type UploadProgress
    = Pending             -- 未アップロード（workflow_instance_id 未取得）
    | RequestingUrl       -- Presigned URL 要求中
    | Uploading Float     -- アップロード中（0.0〜1.0）
    | Confirming          -- アップロード完了通知中
    | Completed           -- 完了
    | Failed String       -- エラー

-- Msg
type Msg
    = SelectFiles
    | FilesSelected File (List File)
    | DragEnter
    | DragLeave
    | FilesDropped File (List File)
    | GotUploadUrl String (Result ApiError UploadUrlResponse)
    | UploadProgress String Http.Progress
    | UploadCompleted String (Result Http.Error ())
    | ConfirmCompleted String (Result ApiError Document)
    | RemoveFile String
```

view:
- ドロップゾーン（点線ボーダー、D&D 対応）
- ファイル選択ボタン
- ファイルリスト（名前、サイズ、進捗バー、削除ボタン）
- バリデーションエラー表示

subscriptions:
- `Http.track` でアップロード進捗を購読（アップロード中のファイルがある場合のみ）

## Phase 4: フロントエンド — 申請フォームへの FileUpload 統合

### 対象
- `frontend/src/Form/DynamicForm.elm`（FileUpload 統合）
- `frontend/src/Page/Workflow/New/Types.elm`（FileUpload 状態追加）
- `frontend/src/Page/Workflow/New.elm`（update/view 拡張）
- `frontend/src/Page/Workflow/New/Api.elm`（保存後アップロード）
- `frontend/src/Main.elm`（WorkflowNewPage の subscriptions 追加）

### 確認事項
- パターン: DynamicForm の viewInput の case 分岐 → `Form/DynamicForm.elm:142-158`
- パターン: Main.elm の subscriptions ルーティング → `Main.elm:790-803`
- パターン: Page/Workflow/New の update フロー → 既存の SaveDraft/Submit ハンドリング
- 型: EditingState の構造 → `Page/Workflow/New/Types.elm:78-88`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ユーザーがファイルを選択し、下書き保存後にアップロードが自動開始される | 正常系 | E2E |
| 2 | ユーザーがファイルを選択し、「保存して申請」でアップロード→申請が連続実行される | 正常系 | E2E |
| 3 | 保存済みの下書きでファイルを追加選択し、即座にアップロードが開始される | 正常系 | E2E |
| 4 | ファイル必須フィールドにファイルが添付されていない状態で申請する | 準正常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] file フィールドが required の場合、完了ファイルがないとバリデーションエラー
- [ ] file フィールドが optional の場合、ファイルなしでもバリデーション通過

ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト:
- [ ] 申請フォームで file フィールドが表示される
- [ ] ファイルを選択して下書き保存し、アップロードが完了する
- [ ] 「保存して申請」でファイルアップロード後に申請が完了する

### 実装内容

1. DynamicForm の viewFileInput を FileUpload コンポーネントに委譲
   - `viewFields` のシグネチャ拡張: FileUpload の状態と Msg マッピングを受け取る
   - File フィールドごとに FileUpload.view を呼び出す

2. New/Types.elm の EditingState に FileUpload 状態を追加
   ```elm
   type alias EditingState =
       { ...既存フィールド...
       , fileUploads : Dict String FileUpload.Model  -- フィールド ID → FileUpload 状態
       }
   ```

3. New.elm の update で FileUpload メッセージをルーティング
   - `FileUploadMsg String FileUpload.Msg` を Msg に追加
   - SaveDraft / Submit 成功時: 未アップロードファイルのアップロード開始

4. New.elm に subscriptions を追加し、Main.elm にルーティング登録

5. 保存→アップロード→申請のフロー:
   - 未保存状態: ファイルは Pending 状態で保持
   - SaveDraft 成功: workflow_instance_id を FileUpload に通知 → アップロード開始
   - Submit: SaveDraft → アップロード完了待ち → submitWorkflow

## Phase 5: フロントエンド — 承認者の添付ファイル表示とダウンロード

### 対象
- `frontend/src/Page/Workflow/Detail.elm`（添付ファイルセクション追加）
- `frontend/src/Page/Workflow/Detail/Types.elm`（添付ファイル状態追加）

### 確認事項
- パターン: Detail ページの並列フェッチ（`Cmd.batch`）→ `Detail.elm` の handleGotWorkflow
- パターン: RemoteData の状態表示パターン → 既存の definition/comments 表示
- 型: LoadedState の構造 → `Detail/Types.elm:71-92`

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 承認者がワークフロー詳細画面で添付ファイル一覧を見る | 正常系 | E2E |
| 2 | 承認者がダウンロードボタンをクリックしてファイルをダウンロードする | 正常系 | E2E |
| 3 | 添付ファイルがない場合の表示 | 正常系 | E2E |

### テストリスト

ユニットテスト（該当なし — View ロジックのみ）

ハンドラテスト（該当なし）
API テスト（該当なし）

E2E テスト:
- [ ] ワークフロー詳細画面に添付ファイルセクションが表示される
- [ ] 添付ファイルのダウンロードリンクが機能する
- [ ] 添付ファイルがない場合に適切なメッセージが表示される

### 実装内容

1. Detail/Types.elm の LoadedState に添付ファイル状態を追加
   ```elm
   type alias LoadedState =
       { ...既存フィールド...
       , attachments : RemoteData ApiError (List Document)
       }
   ```

2. Detail.elm の init で添付ファイル一覧を並列フェッチ
   ```elm
   Cmd.batch
       [ WorkflowDefinitionApi.getDefinition { ... }
       , WorkflowApi.listComments { ... }
       , DocumentApi.listWorkflowAttachments { ... }  -- 追加
       ]
   ```

3. 添付ファイルセクションの view
   - ファイル名、サイズ、Content-Type アイコン
   - ダウンロードボタン（`requestDownloadUrl` → 新しいタブで開く）
   - EmptyState（添付ファイルなし）
   - LoadingSpinner / ErrorState

4. Msg に添付ファイル関連メッセージを追加
   ```elm
   | GotAttachments (Result ApiError (List Document))
   | RequestDownload String  -- document_id
   | GotDownloadUrl String (Result ApiError DownloadUrlResponse)
   ```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ファイルアップロードのタイミング: 未保存の新規申請では workflow_instance_id がない | 不完全なパス | 設計判断 #1 追加: 下書き保存後にアップロード。Pending 状態で UI 表示 |
| 2回目 | formData（Dict String String）にファイル情報をどう格納するか | 曖昧 | 設計判断 #2 追加: 分離管理。FileUpload コンポーネントが独立管理 |
| 3回目 | Main.elm の subscriptions に WorkflowNewPage がない | 未定義 | Phase 4 の実装内容に subscriptions 追加を明記 |
| 4回目 | FieldType の File バリアントにファイル設定情報がない | アーキテクチャ不整合 | 設計判断 #4 追加: `File FileConfig` に拡張 |
| 5回目 | DynamicForm.viewFields のシグネチャが FileUpload 状態を受け取れない | 既存手段の見落とし | Phase 4 でシグネチャ拡張を計画 |
| 6回目 | 「保存して申請」時のアップロード完了待ちフロー | 不完全なパス | Phase 4 実装内容に SaveDraft → アップロード完了待ち → submitWorkflow フローを追記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準4項目すべてが Phase 1-5 でカバー。バックエンド（Phase 1）、フロントエンドデータ層（Phase 2）、コンポーネント（Phase 3）、統合（Phase 4）、表示（Phase 5） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | アップロードタイミング、formData 管理方式、FieldType 拡張方針を設計判断として確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 4 つの設計判断（タイミング、データ管理、Elm 方式、FieldType 拡張）すべてに選択肢・理由あり |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | Phase 1 に対象外を明記。デザイナーのフォームエディタ側の file フィールド追加 UI は #885 のスコープ |
| 5 | 技術的前提 | 前提が考慮されている | OK | elm/file の File.Select/Http.track の利用可能性確認済み、subscriptions の必要性を把握 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書 17_ドキュメント管理設計.md の FileUpload コンポーネント設計・アップロードフロー・API 設計に準拠 |
