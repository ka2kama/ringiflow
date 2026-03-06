# #885 ドキュメント管理フロントエンド実装

## 概要

Issue #885 のドキュメント管理フロントエンドを実装した。フォルダツリー + ファイル一覧の2ペインレイアウトで、フォルダ CRUD 操作とファイルのアップロード・ダウンロード・削除を提供する。Phase 1〜5（Data/API 層、ルーティング、フォルダツリー、フォルダ CRUD、ファイル操作）を完了し、品質ゲートを通過した。

## 実施内容

### Phase 1: Data & API 層

- `Data.Folder` モジュール: フォルダ型定義とデコーダ
- `Api.Folder` モジュール: フォルダ CRUD API クライアント（listFolders, createFolder, renameFolder, deleteFolder）
- `Api.Document` に `requestUploadUrlForFolder` を追加（フォルダ向けアップロード URL 取得）
- テスト: `FolderTest`（デコーダテスト）

### Phase 2: ルーティング & ページスケルトン

- `Route.elm` に `Documents` ルート追加
- `Main.elm` にページ接続（init, update, view, subscriptions）
- サイドバーに「ドキュメント管理」リンク追加
- `Page.Document.List` のスケルトン作成

### Phase 3: フォルダツリーコンポーネント & ファイル一覧

- `Component.FolderTree`: フラットなフォルダリストから再帰ツリーを構築
  - `FolderNode` opaque type + `folderOf`/`childrenOf` アクセサ関数
  - `Dict` ベースの O(n) ツリー構築アルゴリズム
- テスト: `FolderTreeTest`（7テスト）
- 2ペインレイアウト実装（左: フォルダツリー w-72、右: ファイル一覧 flex-1）
- `stopPropagationOn` でフォルダ展開/折りたたみとフォルダ選択のイベント分離

### Phase 4: フォルダ CRUD

- `FolderDialog` union type: `CreateFolderDialog | RenameFolderDialog` で型安全なダイアログ管理
- 管理者限定の操作ボタン表示（`Shared.isAdmin`）
- カスタムダイアログ（テキスト入力が必要なため ConfirmDialog は不使用）

### Phase 5: ファイル操作

- `PendingDelete` union type: `DeleteFolder | DeleteDocument` で統一削除確認
- Presigned URL アップロードフロー
- ダウンロード（`Ports.openUrl`）
- `formatFileSize` ヘルパー（B/KB/MB）
- `MessageAlert` によるフィードバック表示

### 品質ゲート

- elm-review エラー修正（未使用 import、未使用コンストラクタ、subscriptions シグネチャ簡略化）
- Elm 4要素タプル制約への対応（separate let bindings に分割）
- `file-size-exceptions.txt` に `Page/Document/List.elm` を追加（ADR-043 判定）
- rebase on main、CI 全パス

## 判断ログ

- Refactor: `FolderNode` を opaque type にし、コンストラクタを直接公開せずアクセサ関数経由にした（内部構造の隠蔽）
- Refactor: フォルダ/ドキュメント削除を `PendingDelete` union type で統一し、1つの `ConfirmDialog` を共有する設計にした（ConfirmDialog の dialogId 制約への対応）
- Refactor: `FolderDialog` を union type にし、作成/名前変更の状態を型で分離した
- 判断: FileUpload コンポーネントの拡張ではなく、ページ内の直接アップロード実装を選択した（FileUpload はワークフロー固有の設計で複雑すぎるため）
- Refactor: `subscriptions` を `Model -> Sub Msg` から `Sub Msg` に簡略化（引数未使用のため）

## 成果物

### コミット

1. `#885 WIP: Implement document management frontend` — 空コミット（Draft PR 作成用）
2. `#885 Add implementation plan for document management frontend` — 計画ファイル
3. `#885 Add Folder data module and API clients` — Phase 1
4. `#885 Add Documents route, page skeleton, and sidebar link` — Phase 2
5. `#885 Implement folder tree component and document list view` — Phase 3
6. `#885 Add folder CRUD operations (create, rename, delete)` — Phase 4
7. `#885 Add file operations (upload, download, delete)` — Phase 5
8. `#885 Update plan progress and add file size exception` — 計画更新
9. `#885 Fix 4-element tuple to comply with Elm's 3-element limit` — Elm 制約修正
10. `#885 Fix elm-review lint errors in document management page` — lint 修正

### 作成ファイル

- `frontend/src/Data/Folder.elm` — フォルダデータ型
- `frontend/src/Api/Folder.elm` — フォルダ API クライアント
- `frontend/src/Component/FolderTree.elm` — フォルダツリー構築ロジック
- `frontend/src/Component/Icons.elm` — アイコン追加
- `frontend/src/Page/Document/List.elm` — ドキュメント管理ページ（884行）
- `frontend/tests/Component/FolderTreeTest.elm` — ツリー構築テスト
- `frontend/tests/Data/FolderTest.elm` — フォルダデコーダテスト
- `prompts/plans/885-document-management-frontend.md` — 計画ファイル

### 更新ファイル

- `frontend/src/Main.elm` — ページ接続
- `frontend/src/Route.elm` — ルート追加
- `frontend/src/Api/Document.elm` — フォルダ向けアップロード関数追加
- `.config/file-size-exceptions.txt` — 例外追加
