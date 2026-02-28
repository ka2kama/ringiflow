# 計画: #722 ワークフローデザイナーの機能仕様書・詳細設計を作成する

## Context

Phase 2-4（ワークフローデザイナー）の実装に先立ち、設計ドキュメント一式を作成する。Phase 2-3（複数ステップ承認）は完了済みで、前提条件は満たされている。

現在の As-Is:
- 要件定義（CORE-11, WFD-001〜005）は詳細に定義済み
- DB テーブル（`workflow_definitions`）は存在し、JSONB `definition` カラムでスキーマレスに格納中
- seed データで `form` / `steps` / `transitions` 構造が既に使用されている（ただし `position` フィールドなし）
- バックエンド API は GET 系のみ実装済み（Create/Update/Delete 未実装）
- フロントエンドにデザイナー UI なし（Ports パターンは確立済み）

## 対象

| # | 成果物 | パス |
|---|--------|------|
| 1 | 技術選定 ADR | `docs/70_ADR/053_ワークフローデザイナー技術選定.md` |
| 2 | 機能仕様書 | `docs/20_機能仕様書/04_ワークフローデザイナー.md` |
| 3 | 詳細設計書 | `docs/40_詳細設計書/15_ワークフローデザイナー設計.md` |
| 4 | OpenAPI 更新 | `openapi/openapi.yaml` |
| 5 | はじめに更新 | `docs/20_機能仕様書/00_はじめに.md`（読む順序に追記） |

## 対象外

- 実装コード（#723〜#727 で対応）
- マイグレーション SQL
- テストコード

---

## Phase 1: 技術選定 ADR

`docs/70_ADR/053_ワークフローデザイナー技術選定.md`

### 概要

ワークフローデザイナーの Canvas/D&D 実現方式を決定する。

### 判断事項と推奨

**判断1: キャンバスの描画方式**

| 選択肢 | 概要 | Elm 統合 | 接続線描画 | 学習効果 |
|--------|------|---------|-----------|---------|
| A. SVG + Elm 直接レンダリング（推奨） | Elm の `Svg` モジュールでノード・エッジを描画 | 自然（Virtual DOM） | 容易（`<line>`, `<path>`） | 高（SVG 汎用スキル） |
| B. HTML Canvas + JS ライブラリ | Konva.js 等を Ports 経由で操作 | 間接的（Ports 依存） | ライブラリ依存 | 中（ライブラリ固有） |
| C. HTML DOM + CSS absolute | Elm で `div` を absolute 配置 | 自然（HTML） | 困難（SVG オーバーレイ必要） | 低 |

推奨: **A. SVG + Elm 直接レンダリング**

**判断2: ドラッグ&ドロップの実装方式**

推奨: **Elm マウスイベント**（`onMouseDown` / `Browser.Events.onMouseMove` / `onMouseUp`）で TEA Model に dragging 状態を管理。ブラウザの Drag and Drop API は SVG 要素で使いにくいため不採用。Ports は寸法取得（`getBoundingClientRect`）のみ。

### 確認事項
- [x] `elm/svg` パッケージの API → elm/svg 1.0.1 既にインストール済み
- [x] 既存の Ports パターン → `frontend/src/Ports.elm`, `frontend/src/main.js`（sendMessage/receiveMessage/showModalDialog/setBeforeUnloadEnabled）
- [x] Elm マウスイベントの使用パターン → Browser.Events 未使用（プロジェクト初導入）。elm/browser 1.0.2 インストール済み

---

## Phase 2: 機能仕様書

`docs/20_機能仕様書/04_ワークフローデザイナー.md`

### 概要

9 セクション体系に準拠。既存の `01_ワークフロー管理.md` をフォーマット参照元とする。

### セクション構成

1. **概要**: 目的（テナント管理者が GUI でワークフロー定義を作成・編集・公開する）、対象ユーザー（テナント管理者）、関連機能要件（WF-001, WFD-001〜005, CORE-11）

2. **シナリオ**（ペルソナベース）:
   - シナリオ1: 新しいワークフロー定義の作成（一連の操作: パレット→配置→接続→プロパティ設定→バリデーション→保存→公開）
   - シナリオ2: 既存定義の編集（一覧→選択→編集→保存）
   - シナリオ3: バリデーションエラーの解消（不完全なフロー→エラー表示→修正→再バリデーション）
   - シナリオ4: 定義のアーカイブ（使わなくなった定義の非表示化）

3. **画面・操作フロー**:
   - デザイナー画面のレイアウト構成図（Mermaid: パレット | キャンバス | プロパティパネル）
   - 定義管理の全体フロー（一覧→新規作成/編集→保存→公開）
   - キャンバス操作フロー（ステップ配置→接続線→プロパティ編集）

4. **機能詳細**:
   - 4.1 定義一覧・管理: 一覧表示項目、ステータスフィルタ、新規作成・編集・削除・公開・アーカイブ
   - 4.2 キャンバス（WFD-001）: ステップの配置・移動・選択・削除、グリッドスナップ
   - 4.3 ステップパレット（WFD-002）: Phase 2-4 では開始・承認・終了の 3 種
   - 4.4 接続線（WFD-003）: ステップ間の遷移定義、矢印表示、trigger（approve/reject）
   - 4.5 プロパティパネル（WFD-004）: 選択ステップの設定（名前、承認者指定方式、end ステータス）
   - 4.6 バリデーション（WFD-005）: 整合性チェック項目一覧
   - 4.7 フォーム定義: Phase 2-4 スコープのフォームフィールド設定

5. **状態遷移**: ワークフロー定義のライフサイクル（Draft → Published → Archived）。既存の `WorkflowDefinitionStatus` に対応

6. **権限**: テナント管理者のみ（SCR-006 アクセス制御）

7. **非ゴール**: 条件分岐（Phase 3）、並列承認（Phase 3）、プレビュー（WFD-006）、バージョン管理（WFD-007、Phase 4）、インポート/エクスポート（WFD-008）、通知/待機/スクリプト/外部連携ステップ（Phase 3+）、manager/role/group/field/script assignee type（Phase 3+）

8. **未解決事項**: 必要に応じてリスト化

9. **関連ドキュメント**: CORE-11、ADR-053、詳細設計書 15

### 確認事項
- [x] 既存機能仕様書のフォーマット詳細 → `docs/20_機能仕様書/01_ワークフロー管理.md` — 9 セクション体系、シナリオ駆動の記述スタイルを確認
- [x] CORE-11 のスキーマ定義（Phase 2-4 サブセット特定）→ `docs/10_要件定義書/01_コア要件.md` L1061-1340 — steps（start/approval/end）、transitions（trigger）、form.fields（text/textarea/number/select/date）を特定
- [x] ワークフロー定義のステータス遷移 → `backend/crates/domain/src/workflow/definition.rs` — Draft/Published/Archived の 3 状態、can_publish() で遷移制御
- [x] デザインガイドライン（UI 記述時の参照）→ `docs/40_詳細設計書/13_デザインガイドライン.md` — デザイントークン、タイポグラフィ、カラーパレットを参照

---

## Phase 3: 詳細設計書 + OpenAPI

`docs/40_詳細設計書/15_ワークフローデザイナー設計.md` + `openapi/openapi.yaml`

### 概要

既存の詳細設計書フォーマット（`11_ワークフロー承認却下機能設計.md` を参照）に準拠。

### セクション構成

1. **概要**: Phase 2-4 スコープでの実装設計

2. **要件**: WF-001, WFD-001〜005 参照

3. **アーキテクチャ**:
   - シーケンス図（定義作成・更新・公開）
   - フロントエンドのコンポーネント構成（Nested TEA）
   - SVG キャンバスのレンダリング構成
   - Ports 通信設計

4. **ワークフロー定義 JSON スキーマ（Phase 2-4 サブセット）**:
   - CORE-11 のフルスキーマから Phase 2-4 で使用するサブセットを正式定義
   - 既存 seed データとの後方互換性確保
   - `position` フィールドの追加（デザイナー向けキャンバス座標）

   ```
   Phase 2-4 サブセット:
   - form.fields[]: text, textarea, number, select, date（file は Phase 2-5）
   - steps[]: start, approval, end のみ
   - steps[].position: { x, y } — 新規追加
   - steps[].assignee.type: "user" のみ
   - transitions[]: from, to, trigger
   - transitions[].condition: 対象外（Phase 3）
   ```

5. **API 設計**（CRUD + バリデーション）:
   - `POST /api/v1/workflow-definitions` — 作成（Draft）
   - `PUT /api/v1/workflow-definitions/{id}` — 更新
   - `POST /api/v1/workflow-definitions/{id}/publish` — 公開
   - `POST /api/v1/workflow-definitions/{id}/archive` — アーカイブ
   - `DELETE /api/v1/workflow-definitions/{id}` — 削除（Draft のみ）
   - `POST /api/v1/workflow-definitions/{id}/validate` — バリデーション
   - 楽観的ロック: `version` フィールド（既存パターンに準拠）

6. **データモデル変更**:
   - `workflow_definitions` テーブル変更の要否評価
   - `display_id` パターン適用の検討

7. **ドメインロジック（Rust コード案）**:
   - `validate_definition()`: バリデーション関数
   - `WorkflowDefinition::update()`: 更新メソッド
   - CRUD ユースケース

8. **フロントエンド設計**:
   - Page/WorkflowDefinition/ 以下のモジュール構成
   - キャンバスの Model 型定義案
   - SVG ビュー構成
   - D&D イベントハンドリング

9. **バリデーションルール一覧**:
   - 開始ステップが 1 つ存在する
   - 終了ステップが 1 つ以上存在する
   - 承認ステップが 1 つ以上存在する
   - すべてのステップが遷移で接続されている（孤立ステップなし）
   - 循環参照がない
   - 承認ステップに approve/reject 両方の遷移がある

10. **テスト観点**:
    - ドメイン: バリデーションロジック
    - ハンドラ: CRUD API
    - API テスト: E2E API
    - E2E: デザイナー操作シナリオ

11. **OpenAPI 更新**: 上記エンドポイントを `openapi.yaml` に追加。`definition` フィールドの JSON Schema を部分的に型付け

### 確認事項
- [x] 既存の詳細設計書フォーマット → `docs/40_詳細設計書/11_ワークフロー承認却下機能設計.md` — 概要/要件/アーキテクチャ/API設計/データモデル/ドメインロジック/テスト観点の構成。Mermaid 図、Rust コード、diff 形式を使用
- [x] 既存 OpenAPI スキーマ構造 → `openapi/openapi.yaml` — OpenAPI 3.1.0、`ApiResponse_XXX` ラッパー形式、`WorkflowDefinitionData` の `definition` は `{}` (untyped)
- [x] 楽観的ロックのパターン → `ApproveRejectRequest` に `version: int32`、`update_with_version_check` で WHERE version = $N。Version 型は `u32` ラッパー
- [x] `display_id` パターン → `docs/40_詳細設計書/12_表示用ID設計.md` — workflow_definitions にはまだ display_id 未導入。Phase 2-4 では UUID ルーティングを維持
- [x] 既存の Elm ページモジュール構成 → `frontend/src/Page/Workflow/` に Detail.elm/List.elm/New.elm。WorkflowDefinition は新規ディレクトリが必要

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `position` フィールドが CORE-11 に未定義 | 未定義 | Phase 2-4 スキーマサブセットに `position` を追加。既存 seed データとの後方互換性（`position` なし時のデフォルト配置）を設計方針に追加 |
| 2回目 | CORE-11 の `assignee.type` は 6 種あるが Phase 2-4 では `user` のみ | スコープ境界 | Phase 2-4 は `user` のみサポート。非ゴールに他の type を明記 |
| 3回目 | 機能仕様書の `00_はじめに.md` の読む順序を更新する必要がある | 既存パターン整合 | Phase 2 の成果物に `00_はじめに.md` の更新を追加 |
| 4回目 | seed データは `form.fields[]` に `file` 型を含まないが、CORE-11 には定義されている | スコープ境界 | Phase 2-4 では `file` 型は対象外（Phase 2-5 の S3 連携で対応）。機能仕様書の非ゴールに明記 |
| 5回目 | `openapi.yaml` は utoipa から自動生成される（`just openapi-generate`）ため手動編集が不適切 | 技術的前提 | API 仕様は詳細設計書に記載。`openapi.yaml` は実装時に utoipa アノテーションから自動生成する方針に変更 |
| 6回目 | 既存の `can_publish()` は Archived → Published 遷移を許可してしまう | 不完全なパス | 詳細設計書の状態遷移は Draft → Published → Archived の一方向。実装時に `can_publish` を修正して Draft のみ許可に変更 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了基準 5 項目すべてが Phase に含まれている | OK | 機能仕様書(Phase 2)、詳細設計書(Phase 3)、OpenAPI(Phase 3)、ADR(Phase 1)、スコープ明記(全 Phase) |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 技術選択は 3 択から推奨を明示。Phase 2-4 スコープ（ステップ種別、assignee type、form field type）を具体的に限定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | Canvas 方式、D&D 方式、バリデーション境界、ロック方式、スキーマ範囲の判断事項を列挙 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象（5 成果物）と対象外（実装コード、マイグレーション、テスト）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | SVG マウスイベントと D&D API の関係、Elm `Browser.Events` subscription、Ports メッセージ形式を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | CORE-11 スキーマ、実装ロードマップ Phase 2-4、既存機能仕様書フォーマット、ADR テンプレートと照合済み |

## 検証方法

ドキュメント作成タスクのため、テスト実行は不要。以下で検証する:

- 各ドキュメントが既存フォーマットに準拠していること
- Mermaid 図がレンダリングされること（GitHub プレビューで確認）
- OpenAPI の構文が正しいこと（`just check` で lint 確認）
- 相互参照が正しいこと（リンク先が存在すること）
