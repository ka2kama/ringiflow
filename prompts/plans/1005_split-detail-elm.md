# #1005 Detail.elm の分割

## 概要

ADR-062 に基づき、`frontend/src/Page/Workflow/Detail.elm`（1479行）を機能ドメイン別にサブモジュール分割する。

## 設計判断

### Types.elm の追加（5 → 6 ファイル）

Designer.elm 分割（PR #1013）と同じ循環依存の問題が発生する:
- Detail.elm がサブモジュールの view/update 関数を呼び出す
- サブモジュールが Detail.elm の Msg 型を参照する

Types.elm に共有型を抽出して一方向の依存グラフを実現する。

代替案として検討したコールバックレコードパターンは、特に Resubmit（9 個の Msg バリアント）で冗長になるため不採用。ADR-062 でも同じ結論。

### サブモジュールの update + view 同居

Designer.elm では Update.elm に全 update ロジックを集約した。Detail.elm では機能ドメイン（承認・再提出・コメント）が独立しているため、各サブモジュールに update と view を同居させ、Detail.elm はルーティングのみを担う。

## ファイル構成

```
Page/Workflow/
├── Detail.elm            # 本体（init, update routing, view layout, basic views）
└── Detail/
    ├── Types.elm         # 共有型定義（Model, Msg, states）
    ├── Approval.elm      # 承認操作（update + view）
    ├── Resubmit.elm      # 再提出・編集（update + view）
    ├── Comments.elm      # コメント（update + view）
    └── StepProgress.elm  # 進捗表示（純粋 View）
```

### 各モジュールの責務と推定行数

| モジュール | 内容 | 推定行数 |
|-----------|------|---------|
| Types.elm | PendingAction, Model, PageState, LoadedState, EditState, EditingState, Msg, initLoaded | ~120 |
| Detail.elm | init, update routing, subscriptions, view layout, viewTitle/Status/BasicInfo, viewSteps, viewFormData | ~380 |
| Approval.elm | 承認/却下/差し戻し update + view, ConfirmDialog, findActiveStepForUser | ~250 |
| Resubmit.elm | 編集モード update + view, ApproverSelector 管理, バリデーション | ~330 |
| Comments.elm | コメント update + view, CommentList/Form | ~120 |
| StepProgress.elm | 進捗バー view, stepProgressStyle | ~85 |

全ファイル 500行以下。合計 ~1285行（元 1479行）。

## 実装計画

### Phase 1: Types.elm 作成 + Detail.elm 更新

Detail.elm から共有型を Types.elm に抽出し、Detail.elm の import を更新する。

#### 確認事項
- 型: Designer/Types.elm のパターン → `frontend/src/Page/WorkflowDefinition/Designer/Types.elm`
- パターン: Designer.elm の Types.elm import パターン → `frontend/src/Page/WorkflowDefinition/Designer.elm`

#### 操作パス: 該当なし（リファクタリング、動作変更なし）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — 既存テストで検証）

検証: `elm make` で全体がコンパイルされること。

### Phase 2: StepProgress.elm 抽出

純粋 View 関数（viewStepProgress, viewStepProgressItem, viewStepConnector, stepProgressStyle）を抽出。最もシンプルで依存が少ない。

#### 確認事項: なし（既知のパターンのみ）

#### 操作パス: 該当なし（リファクタリング）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: Comments.elm 抽出

update（GotComments, UpdateNewComment, SubmitComment, GotPostCommentResult）と view（viewCommentSection, viewCommentList, viewCommentItem, viewCommentForm）を抽出。

#### 確認事項
- パターン: Designer/Toolbar.elm 等の submodule が Types.elm から Msg を import するパターン

#### 操作パス: 該当なし（リファクタリング）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 4: Approval.elm 抽出

update（UpdateComment, ClickApprove/Reject/RequestChanges, ConfirmAction, CancelAction, GotApprove/Reject/RequestChangesResult）とhelper（nonEmptyComment, handleApprovalResult）とview（viewApprovalSection, viewCommentInput, viewApprovalButtons, findActiveStepForUser, viewConfirmDialog）を抽出。

#### 確認事項: なし（Phase 3 で確認済みのパターン）

#### 操作パス: 該当なし（リファクタリング）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 5: Resubmit.elm 抽出

update（StartEditing, CancelEditing, UpdateEditFormField, EditApprover*, SubmitResubmit, GotResubmitResult, GotUsers）とhelper（updateEditing, updateApproverState, validateResubmit, buildResubmitApprovers, encodeFormValues）とview（viewResubmitSection, viewEditableFormData, viewEditableFormField, viewEditableApprovers, viewEditableApproverStep, viewEditActions）を抽出。

#### 確認事項: なし（Phase 3-4 で確認済みのパターン）

#### 操作パス: 該当なし（リファクタリング）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 6: 最終検証

- `just check` 通過
- 行数確認（全ファイル 500行以下）
- E2E テスト確認（`just test-e2e` は UI 変更時のみ。今回はリファクタリングのため `just check` で十分）

#### 確認事項: なし

#### 操作パス: 該当なし

#### テストリスト

ユニットテスト（該当なし — 既存テストで検証）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — 既存 E2E テストで検証。新規テストは不要）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Issue は 5 ファイルだが循環依存回避のため Types.elm が必要 | 既存手段の見落とし | 6 ファイル構成に変更、Types.elm パターンを採用 |
| 2回目 | Designer.elm は Update.elm に集約したが Detail.elm はドメイン別分離が適切 | シンプルさ | 各サブモジュールに update+view を同居させる方式を採用 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | ADR-062 の責務分析の全行（1479行）が 6 ファイルに配分されている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase で移動する関数を明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | Types.elm 追加、update+view 同居の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: Detail.elm の分割。対象外: 新機能追加、動作変更 |
| 5 | 技術的前提 | 前提が考慮されている | OK | Elm の循環依存制約を確認済み（Designer.elm で実証済み） |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-062 の分割計画に準拠（Types.elm 追加は Designer.elm の先例に倣う） |
