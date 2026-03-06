# 計画: #885 ドキュメント管理フロントエンド

## コンテキスト

### 目的
- Issue: #885
- Want: ドキュメント管理画面を提供し、ユーザーがフォルダツリーでファイルを整理・操作できるようにする
- 完了基準:
  - ドキュメント管理画面でフォルダツリーが表示される
  - フォルダ選択でファイル一覧が切り替わる
  - フォルダ作成・名前変更・削除操作ができる
  - ファイルのアップロード・ダウンロード・削除操作ができる
  - ファイル削除時に確認ダイアログが表示される

### ブランチ / PR
- ブランチ: `feature/885-document-management-frontend`
- PR: #1067（Draft）

### As-Is（探索結果の要約）

バックエンド API:
- フォルダ API（#883）: `GET/POST /api/v1/folders`, `PUT/DELETE /api/v1/folders/{id}` — 実装済み
- ドキュメント API: `GET /api/v1/documents?folder_id=xxx`, `DELETE /api/v1/documents/{id}` — 実装済み
- アップロード/ダウンロード: `POST .../upload-url`, `POST .../confirm`, `POST .../download-url` — 実装済み

フロントエンド既存:
- `Component/FileUpload.elm` — Presigned URL アップロードフロー完備。`workflowInstanceId` ベース。`folder_id` 対応は未実装
- `Data/Document.elm` — Document 型 + デコーダー。`listDecoder` あり
- `Api/Document.elm` — `requestUploadUrl`（`workflowInstanceId` のみ）、`confirmUpload`、`requestDownloadUrl`、`listWorkflowAttachments`、`uploadToS3`
- `Api/Folder.elm` — 未作成
- `Data/Folder.elm` — 未作成
- `Page/Document/` — 未作成
- `Component/FolderTree.elm` — 未作成
- `Route.elm` — Documents ルートなし
- `Main.elm` — サイドバーに「ドキュメント管理」なし
- `Component/Icons.elm` — documents アイコンなし

キーファイル:
- フロントエンド構造参照: `frontend/src/Page/AuditLog/List.elm`（一覧ページパターン）
- ルーティング: `frontend/src/Route.elm`
- Main wiring: `frontend/src/Main.elm`
- API 基盤: `frontend/src/Api.elm`（`get`/`post`/`put`/`delete`/`deleteNoContent`）
- Shared: `frontend/src/Shared.elm`（`toRequestConfig`, `isAdmin`）
- OpenAPI: `openapi/openapi.yaml`
- 詳細設計: `docs/40_詳細設計書/17_ドキュメント管理設計.md`

API レスポンス形式:
- FolderData: `{ id, name, parent_id?, path, depth, created_at, updated_at }`（`data` ラッパー）
- DocumentData: `{ id, filename, content_type, size, status, created_at }`（`data` ラッパー）
- RequestUploadUrlRequest: `{ filename, content_type, content_length, folder_id?, workflow_instance_id? }`
- DELETE /documents/{id}: 204 No Content
- DELETE /folders/{id}: 204（空フォルダのみ、非空は 400）

### 進捗
- [ ] Phase 1: データ & API 層
- [ ] Phase 2: ルーティング & ページスケルトン
- [ ] Phase 3: フォルダツリーコンポーネント & ファイル一覧
- [ ] Phase 4: フォルダ CRUD 操作
- [ ] Phase 5: ファイル操作（アップロード・ダウンロード・削除）

---

## 仕様整理

### スコープ
- 対象:
  - `Data/Folder.elm` — フォルダデータ型・デコーダー
  - `Api/Folder.elm` — フォルダ API クライアント
  - `Api/Document.elm` — `listDocuments`（folder_id）、`deleteDocument` 追加
  - `Component/FolderTree.elm` — フォルダツリーコンポーネント
  - `Page/Document/List.elm` — ドキュメント管理ページ
  - `Route.elm` — Documents ルート追加
  - `Main.elm` — ページ wiring、サイドバーリンク追加
  - `Component/Icons.elm` — documents アイコン追加
  - `Component/FileUpload.elm` — `folderId` 対応の拡張
- 対象外:
  - バックエンド API の変更（実装済み）
  - ファイルプレビュー機能（Phase 3 以降）
  - フォルダ移動機能（名前変更のみ対応。移動は UX が複雑なため対象外）
  - E2E テスト（Elm ユニットテスト + 手動確認で検証）

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | サイドメニューから「ドキュメント管理」画面を開き、フォルダツリーが表示される | 正常系 | E2E（手動確認） |
| 2 | フォルダをクリックして選択し、ファイル一覧が切り替わる | 正常系 | ユニット（update ロジック） |
| 3 | 「新しいフォルダ」ボタンでフォルダを作成する | 正常系 | ユニット（update ロジック） |
| 4 | フォルダ名を変更する | 正常系 | ユニット（update ロジック） |
| 5 | 空フォルダを削除する | 正常系 | ユニット（update ロジック） |
| 6 | ファイルをアップロード（D&D またはファイル選択）する | 正常系 | ユニット（FileUpload 拡張） |
| 7 | ファイルをダウンロードする | 正常系 | ユニット（update ロジック） |
| 8 | ファイルを削除する（確認ダイアログ表示 → 削除実行） | 正常系 | ユニット（update ロジック） |
| 9 | フォルダ作成時に名前が重複する | 準正常系 | ユニット（エラーハンドリング） |
| 10 | 非空フォルダを削除しようとする | 準正常系 | ユニット（エラーハンドリング） |
| 11 | API エラー発生時にエラー表示される | 準正常系 | ユニット（RemoteData Failure） |
| 12 | フォルダ/ファイルが空の場合に EmptyState が表示される | 準正常系 | ユニット（view ロジック） |

---

## 設計

### 設計判断

| # | 判断 | 選択肢 | 選定理由 | 状態 |
|---|------|--------|---------|------|
| 1 | フォルダツリーのデータ構造 | A: フラットリスト + parentId で view 時にツリー構築 / B: 再帰型 Tree | A: API がフラットリストを返す（path 順）。ツリー構築は view 用のヘルパーで対応。状態管理がシンプル | 確定 |
| 2 | FileUpload の folder_id 対応 | A: FileUpload に folderId パラメータ追加 / B: 別コンポーネントを新規作成 | A: 既存コンポーネントの拡張で対応。`workflowInstanceId` と `folderId` を排他的 Union 型にする | 確定 |
| 3 | フォルダ作成 UI | A: モーダルダイアログ / B: インライン入力 | A: 機能仕様書のシナリオ 2 にダイアログ記載あり。ConfirmDialog コンポーネントを参考にした独自ダイアログ | 確定 |
| 4 | フォルダ名前変更 UI | A: モーダルダイアログ / B: インライン編集 | A: フォルダ作成と統一した UX。同じダイアログコンポーネントを再利用 | 確定 |
| 5 | フォルダ移動 | 対象外 | UX が複雑（ドラッグ&ドロップ or ツリー選択ダイアログ）。Issue スコープに明示的記載なし | 確定 |
| 6 | ダウンロードの実行方法 | A: window.open で Presigned URL を開く（Ports） / B: a タグの href に設定 | A: Presigned URL は POST で取得後に使う必要がある。Ports で window.open を呼ぶ | 確定 |

### Phase 1: データ & API 層

#### 確認事項
- 型: `Data/Document.elm` の既存型 → `frontend/src/Data/Document.elm`
- パターン: `Api/Role.elm` の CRUD パターン → `frontend/src/Api/Role.elm`
- パターン: `Api.elm` の `deleteNoContent` → `frontend/src/Api.elm`
- ライブラリ: `Json.Decode.Pipeline` → Grep 既存使用

#### テストリスト

ユニットテスト:
- [ ] Folder デコーダー: 全フィールド（parent_id あり/なし）をデコードできる
- [ ] Folder リストデコーダー: data ラッパーからリストをデコードできる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 2: ルーティング & ページスケルトン

#### 確認事項
- パターン: `Route.elm` のルート追加パターン → `frontend/src/Route.elm`
- パターン: `Main.elm` のページ wiring パターン → `frontend/src/Main.elm`
- パターン: `Component/Icons.elm` のアイコン定義 → `frontend/src/Component/Icons.elm`

#### テストリスト

ユニットテスト:
- [ ] Documents ルートのパース: `/documents` → `Documents`
- [ ] Documents ルートの toString: `Documents` → `"/documents"`

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: フォルダツリーコンポーネント & ファイル一覧

#### 確認事項
- 型: `RemoteData` の使用パターン → Grep `RemoteData` in `frontend/src/Page/`
- パターン: `Component/ConfirmDialog.elm` → `frontend/src/Component/ConfirmDialog.elm`
- パターン: `Page/AuditLog/List.elm` のデータ取得パターン → `frontend/src/Page/AuditLog/List.elm`
- パターン: `Shared.isAdmin` の使用 → Grep `isAdmin`

#### テストリスト

ユニットテスト:
- [ ] フォルダツリー構築: フラットリストからツリー構造を構築できる
- [ ] フォルダツリー構築: 空リストの場合は空ツリーを返す
- [ ] フォルダツリー構築: ルートフォルダのみの場合
- [ ] フォルダツリー構築: 多階層（3階層以上）のネスト
- [ ] フォルダ選択: フォルダ選択でドキュメント一覧取得コマンドが発行される
- [ ] フォルダ展開/折りたたみ: expandedFolderIds の更新

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 4: フォルダ CRUD 操作

#### 確認事項
- パターン: `Page/Role/New.elm` のフォーム送信パターン → `frontend/src/Page/Role/New.elm`
- パターン: `Component/ConfirmDialog.elm` の使用 → Grep `ConfirmDialog` in `frontend/src/Page/`
- パターン: `Component/MessageAlert.elm` の使用 → Grep `MessageAlert` in `frontend/src/Page/`

#### テストリスト

ユニットテスト:
- [ ] フォルダ作成: ダイアログ表示 → 名前入力 → 送信で createFolder API が呼ばれる
- [ ] フォルダ作成成功: フォルダ一覧が再取得される
- [ ] フォルダ作成エラー: エラーメッセージが表示される（名前重複等）
- [ ] フォルダ名前変更: ダイアログ表示 → 名前変更 → 送信で updateFolder API が呼ばれる
- [ ] フォルダ削除: 確認ダイアログ → 確認で deleteFolder API が呼ばれる
- [ ] フォルダ削除エラー: 非空フォルダ削除時にエラーメッセージが表示される
- [ ] 管理者のみ: フォルダ操作ボタンは管理者にのみ表示される

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 5: ファイル操作（アップロード・ダウンロード・削除）

#### 確認事項
- 型: `Component/FileUpload.elm` の Model/Msg → `frontend/src/Component/FileUpload.elm`
- パターン: `Data/FormField.elm` の FileConfig → `frontend/src/Data/FormField.elm`
- ライブラリ: Ports パターン（window.open） → Grep `port` in `frontend/src/`

#### テストリスト

ユニットテスト:
- [ ] ファイルアップロード: FileUpload コンポーネントが folderId 付きで requestUploadUrl を呼ぶ
- [ ] ファイルダウンロード: ダウンロードボタンクリックで requestDownloadUrl API が呼ばれる
- [ ] ファイル削除: 確認ダイアログ → 確認で deleteDocument API が呼ばれる
- [ ] ファイル削除成功: ファイル一覧が再取得される
- [ ] ファイル削除確認ダイアログ: ファイル名が表示される

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## ブラッシュアップ

### ギャップ発見の観点 進行状態

| 観点 | 状態 | メモ |
|------|------|------|
| 未定義 | 完了 | FileUpload の folderId 対応方法を設計判断 #2 で定義済み |
| 曖昧 | 完了 | フォルダ移動の対象外を明示（設計判断 #5） |
| 競合・エッジケース | 完了 | 非空フォルダ削除は API が 400 を返す。フロントでもエラー表示で対応 |
| 不完全なパス | 完了 | 操作パス #9-12 で準正常系を網羅 |
| アーキテクチャ不整合 | 完了 | 既存パターン（TEA + RemoteData）に準拠 |
| 責務の蓄積 | 完了 | Page/Document/List.elm が大きくなる可能性あるが、初期実装では単一ファイルで開始 |
| 既存手段の見落とし | 完了 | ConfirmDialog, EmptyState, ErrorState, LoadingSpinner, MessageAlert, Button, Badge を活用 |
| デザイントークン乖離 | 完了 | 既存ページ（AuditLog/List.elm, Role/List.elm）のクラス名パターンを踏襲 |
| アクセシビリティ欠陥 | 完了 | セマンティック HTML（nav, button, aria-label）、focus-visible を使用 |
| 状態網羅漏れ | 完了 | folders: RemoteData 4 状態 + 空データ。documents: RemoteData 4 状態 + 空データ + NotAsked（フォルダ未選択時） |
| 状態依存フィールド | 完了 | フォルダダイアログは CreateFolder/RenameFolder の Union 型で分離 |
| テスト層網羅漏れ | 完了 | 全 Phase で 4 層明記済み。フロントエンドのみのため ハンドラ/API テストは該当なし |
| 操作パス網羅漏れ | 完了 | 完了基準 5 項目すべてに対応する操作パスあり |

### ループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | FileUpload の folderId 対応が未定義 | 未定義 | 設計判断 #2 を追加。排他的 Union 型で workflowInstanceId と folderId を表現 |
| 1回目 | フォルダ移動の扱いが曖昧 | 曖昧 | 設計判断 #5 で対象外を明示 |
| 1回目 | ダウンロードの実装方法が未定義 | 未定義 | 設計判断 #6 を追加。Ports で window.open |
| 2回目 | フォルダダイアログの状態モデルが未定義 | 状態依存フィールド | 作成/名前変更を Union 型で分離する方針を記載 |

### 未解決の問い
- なし

---

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 5 項目すべてに対応する Phase・テストリストあり |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | フォルダ移動の対象外、ダウンロード方法を明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 6 件の設計判断すべて確定 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 仕様整理セクションで明示 |
| 5 | 技術的前提 | 前提が考慮されている | OK | Ports（window.open）の必要性を認識。Elm の File/Http API は確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 詳細設計書・機能仕様書・OpenAPI と照合済み |
