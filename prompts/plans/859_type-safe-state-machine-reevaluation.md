# #859 型安全ステートマシン: 既存実装の新判断基準での再評価

## Context

#855 で型安全ステートマシンの判断基準が「ADT で表現できるか」から「不正な状態遷移が型レベルで防止されているか」に変更された。Issue #859 の精査で既存6実装を再評価した結果、4つの改善候補が特定された。本計画は改善候補 2, 3, 4 を実装する（候補3は候補1の上位互換）。

## スコープ

### 対象

| # | 改善候補 | 分類 | 対象ファイル |
|---|---------|------|------------|
| 3 | `CancelledState` の遷移元 ADT 分離 | ゼロ→プラス | `backend/crates/domain/src/workflow/instance.rs` |
| 2 | `completed_at()` のワイルドカード明示化 | マイナス→ゼロ | 同上 |
| 4 | `Workflow/Detail.elm` LoadedState 編集状態分離 | ゼロ→プラス | `frontend/src/Page/Workflow/Detail.elm` |

### 対象外

| # | 改善候補 | 理由 |
|---|---------|------|
| 1 | `CancelledState` の `from_db()` 相関バリデーション | 候補3（ADT 分離）で上位互換。型レベルで制約が強制されるため不要 |

## 設計判断

### 候補3（ADT 分離）を採用する理由

候補1（ロジックベースのバリデーション）は `from_db()` に `if` 文を追加するが、これは:
- 将来 AI / ヒトが削除・回避・書き忘れる可能性がある
- コンパイラが強制しない — テストか実行時にしか検出できない

候補3（型ベースの ADT 分離）は:
- `FromActive` バリアントを構築するには `current_step_id` と `submitted_at` の両方が必須 — コンパイラが強制
- `from_db()` の pattern match で `(Some(_), None)` → Err が自然に発生 — 追加のバリデーションロジック不要
- 「型で表現できるものは型で表現する」原則に完全に合致
- getter の複雑化は `CancelledState` にヘルパーメソッドを定義して解消

getter 複雑化の解消:

```rust
impl CancelledState {
    pub fn completed_at(&self) -> DateTime<Utc> { ... }
    pub fn current_step_id(&self) -> Option<&str> { ... }
    pub fn submitted_at(&self) -> Option<DateTime<Utc>> { ... }
}
```

### Detail.elm の `users` フィールド配置

選択: `users` は LoadedState に残す（EditingState に移動しない）

根拠:
- New.elm でも `users` は Model レベルに配置（EditingState の外）
- 編集セッション間でユーザー一覧をキャッシュ可能（毎回再取得不要）
- ただし現在の挙動（StartEditing 時に毎回 Loading にリセット → 再取得）は変更しない（挙動変更は別 Issue のスコープ）

## Phase 構成

### Phase 1: CancelledState 遷移元 ADT 分離 + getter ワイルドカード明示化

対象: `backend/crates/domain/src/workflow/instance.rs`

#### 変更内容

1. `CancelledState` を struct から enum に変更（L122-134）:

```rust
/// 取り消し状態
///
/// 遷移元に応じて保持するフィールドが異なる。
/// ADT で遷移元ごとのバリアントに分離し、不正なフィールド組み合わせを型レベルで防止する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelledState {
    /// Draft から取り消し（申請前のため submitted_at, current_step_id なし）
    FromDraft {
        completed_at: DateTime<Utc>,
    },
    /// Pending から取り消し（申請済みだがステップ未開始のため current_step_id なし）
    FromPending {
        submitted_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    },
    /// InProgress/ChangesRequested から取り消し（ステップ処理中）
    FromActive {
        current_step_id: String,
        submitted_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    },
}
```

2. `CancelledState` にヘルパーメソッドを追加:

```rust
impl CancelledState {
    pub fn completed_at(&self) -> DateTime<Utc> {
        match self {
            Self::FromDraft { completed_at, .. }
            | Self::FromPending { completed_at, .. }
            | Self::FromActive { completed_at, .. } => *completed_at,
        }
    }

    pub fn current_step_id(&self) -> Option<&str> {
        match self {
            Self::FromActive { current_step_id, .. } => Some(current_step_id),
            Self::FromDraft { .. } | Self::FromPending { .. } => None,
        }
    }

    pub fn submitted_at(&self) -> Option<DateTime<Utc>> {
        match self {
            Self::FromPending { submitted_at, .. }
            | Self::FromActive { submitted_at, .. } => Some(*submitted_at),
            Self::FromDraft { .. } => None,
        }
    }
}
```

3. `from_db()` の Cancelled 分岐（L305-316）を pattern match に変更:

```rust
WorkflowInstanceStatus::Cancelled => {
    let completed_at = record.completed_at.ok_or_else(|| {
        DomainError::Validation(
            "Cancelled インスタンスには completed_at が必要です".to_string(),
        )
    })?;
    let cancelled_state = match (record.current_step_id, record.submitted_at) {
        (None, None) => CancelledState::FromDraft { completed_at },
        (None, Some(submitted_at)) => CancelledState::FromPending {
            submitted_at,
            completed_at,
        },
        (Some(current_step_id), Some(submitted_at)) => CancelledState::FromActive {
            current_step_id,
            submitted_at,
            completed_at,
        },
        (Some(_), None) => {
            return Err(DomainError::Validation(
                "Cancelled インスタンスで current_step_id がある場合は submitted_at が必要です"
                    .to_string(),
            ));
        }
    };
    WorkflowInstanceState::Cancelled(cancelled_state)
}
```

4. `cancelled()` メソッド（L475-520）の各分岐を新バリアントに変更:

```rust
WorkflowInstanceState::Draft => Ok(Self {
    state: WorkflowInstanceState::Cancelled(CancelledState::FromDraft {
        completed_at: now,
    }),
    ..
}),
WorkflowInstanceState::Pending(pending) => Ok(Self {
    state: WorkflowInstanceState::Cancelled(CancelledState::FromPending {
        submitted_at: pending.submitted_at,
        completed_at: now,
    }),
    ..
}),
WorkflowInstanceState::InProgress(in_progress) => Ok(Self {
    state: WorkflowInstanceState::Cancelled(CancelledState::FromActive {
        current_step_id: in_progress.current_step_id,
        submitted_at: in_progress.submitted_at,
        completed_at: now,
    }),
    ..
}),
WorkflowInstanceState::ChangesRequested(changes) => Ok(Self {
    state: WorkflowInstanceState::Cancelled(CancelledState::FromActive {
        current_step_id: changes.current_step_id,
        submitted_at: changes.submitted_at,
        completed_at: now,
    }),
    ..
}),
```

5. getter メソッドを CancelledState ヘルパーを使うよう更新:

```rust
// current_step_id() — 既存の CancelledState 行を変更
WorkflowInstanceState::Cancelled(s) => s.current_step_id(),

// submitted_at() — 既存の CancelledState 行を変更
WorkflowInstanceState::Cancelled(s) => s.submitted_at(),

// completed_at() — ワイルドカード明示化 + ヘルパー使用
pub fn completed_at(&self) -> Option<DateTime<Utc>> {
    match &self.state {
        WorkflowInstanceState::Approved(s) | WorkflowInstanceState::Rejected(s) => {
            Some(s.completed_at)
        }
        WorkflowInstanceState::Cancelled(s) => Some(s.completed_at()),
        WorkflowInstanceState::Draft
        | WorkflowInstanceState::Pending(_)
        | WorkflowInstanceState::InProgress(_)
        | WorkflowInstanceState::ChangesRequested(_) => None,
    }
}
```

#### 確認事項

- 型: `CancelledState` のフィールド定義 → instance.rs L122-134
- 型: `CancelledState` の外部使用（直接フィールドアクセス） → Grep で確認済み（instance.rs のみ）
- パターン: 既存の from_db バリデーション（ChangesRequested の pattern match） → instance.rs L317-330
- パターン: getter のパターン列挙 → instance.rs L397-424 (current_step_id, submitted_at)
- パターン: cancelled() メソッドの遷移パターン → instance.rs L475-520

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | cancelled() で各状態から CancelledState を構築 | 正常系 | ユニット（既存） |
| 2 | from_db で正当な CancelledState 3パターンを復元 | 正常系 | ユニット（既存） |
| 3 | from_db で不正な相関（current_step_id=Some, submitted_at=None）を拒否 | 準正常系 | ユニット |

#### テストリスト

ユニットテスト:
- [ ] from_db: Cancelled(current_step_id=Some, submitted_at=None) → DomainError
- [ ] 既存の cancelled() テスト（L835-943）が通過すること（リグレッション確認）
- [ ] 既存の from_db テスト（L1278-1289）が通過すること（リグレッション確認）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 2: Workflow/Detail.elm 編集状態分離

対象: `frontend/src/Page/Workflow/Detail.elm`

#### 変更内容

LoadedState から編集関連フィールドを ADT に分離:

```elm
type alias LoadedState =
    { workflow : WorkflowInstance
    , definition : RemoteData ApiError WorkflowDefinition
    -- 承認/却下/差し戻し
    , comment : String
    , isSubmitting : Bool
    , pendingAction : Maybe PendingAction
    , errorMessage : Maybe String
    , successMessage : Maybe String
    -- コメントスレッド
    , comments : RemoteData ApiError (List WorkflowComment)
    , newCommentBody : String
    , isPostingComment : Bool
    -- ユーザー一覧（承認者選択で使用）
    , users : RemoteData ApiError (List UserItem)
    -- 編集状態
    , editState : EditState
    }

type EditState
    = Viewing
    | Editing EditingState

type alias EditingState =
    { editFormData : Dict String String
    , editApprovers : Dict String ApproverSelector.State
    , resubmitValidationErrors : Dict String String
    , isResubmitting : Bool
    }
```

削除フィールド（LoadedState から）: `isEditing`, `editFormData`, `editApprovers`, `resubmitValidationErrors`, `isResubmitting`
追加フィールド（LoadedState に）: `editState : EditState`
移動なし（LoadedState に残る）: `users`

#### update 関数の変更

`StartEditing`（L432-485）:

```elm
StartEditing ->
    -- ... 既存の formDataDict, approverStates 構築ロジックは維持 ...
    ( { loaded
        | editState =
            Editing
                { editFormData = formDataDict
                , editApprovers = approverStates
                , resubmitValidationErrors = Dict.empty
                , isResubmitting = False
                }
        , users = RemoteData.Loading
      }
    , UserApi.listUsers { ... }
    )
```

`CancelEditing`（L487-495）:

```elm
CancelEditing ->
    ( { loaded | editState = Viewing }, Cmd.none )
```

`UpdateEditFormField`（L497-500）:

```elm
UpdateEditFormField fieldId fieldValue ->
    case loaded.editState of
        Editing editing ->
            ( { loaded
                | editState =
                    Editing { editing | editFormData = Dict.insert fieldId fieldValue editing.editFormData }
              }
            , Cmd.none
            )
        Viewing ->
            ( loaded, Cmd.none )
```

編集系メッセージ（`EditApprover*`, `SubmitResubmit`, `GotResubmitResult`）も同パターンで `case loaded.editState of` ガードを追加。

`GotResubmitResult`（L578-603）:

```elm
GotResubmitResult result ->
    case result of
        Ok workflow ->
            ( { loaded
                | workflow = workflow
                , editState = Viewing
                , successMessage = Just "再申請しました"
                , errorMessage = Nothing
              }
            , WorkflowApi.listComments { ... }
            )
        Err error ->
            case loaded.editState of
                Editing editing ->
                    ( { loaded
                        | editState = Editing { editing | isResubmitting = False }
                        , errorMessage = Just (ErrorMessage.toUserMessage { entityName = "ワークフロー" } error)
                      }
                    , Cmd.none
                    )
                Viewing ->
                    ( loaded, Cmd.none )
```

#### view 関数の変更

`viewWorkflowDetail`（L793-799）:

```elm
, case loaded.editState of
    Editing editing ->
        viewEditableFormData loaded editing
    Viewing ->
        viewFormData loaded.workflow loaded.definition
```

`viewEditableFormData` の引数に `EditingState` を追加し、`loaded.editFormData` → `editing.editFormData` 等に変更。

`viewResubmitSection` も同様に `EditState` を考慮するよう変更。

#### initLoaded の変更

```elm
initLoaded workflow =
    { ...
    , users = NotAsked
    , editState = Viewing
    }
```

#### 確認事項

- 型: LoadedState の現在のフィールド定義 → Detail.elm L103-126
- パターン: New.elm の FormState パターン → New.elm L103-122
- パターン: update 関数の編集メッセージハンドリング全体 → Detail.elm L432-619
- パターン: view 関数の isEditing 分岐 → Detail.elm L793-799
- ライブラリ: Dict, RemoteData の既存使用パターン → Detail.elm 内で確認済み

#### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 詳細画面を表示（閲覧モード） | 正常系 | コンパイル検証 |
| 2 | 編集ボタン → 編集モード → フォーム入力 → 再申請 | 正常系 | コンパイル検証 |
| 3 | 編集ボタン → 編集モード → キャンセル | 正常系 | コンパイル検証 |
| 4 | 再申請でバリデーションエラー | 準正常系 | コンパイル検証 |
| 5 | 再申請で API エラー | 異常系 | コンパイル検証 |

操作パスは型リファクタリングのため、Elm コンパイラの型チェックが主要な検証手段。

#### テストリスト

ユニットテスト（該当なし — Elm のリファクタリング。型チェックが主要な検証）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし — E2E テスト環境は未構築）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `users` フィールドの配置が未決定 | 未定義 | LoadedState に残す方針を明記（New.elm との一貫性） |
| 2回目 | 候補1（ロジック制約）は AI が将来間違える可能性がある | 型の活用 | 候補3（型制約）に変更。型レベルで不正な組み合わせを排除 |
| 3回目 | `GotResubmitResult Err` のケースで editing → Viewing の不整合 | 不完全なパス | Err 時は Editing 維持（isResubmitting を False に戻すのみ）と明記 |
| 4回目 | getter 複雑化の解消策が未定義 | シンプルさ | CancelledState にヘルパーメソッドを定義し、WorkflowInstance の getter からデリゲート |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の4候補全てに実装/除外の判断あり。候補3が候補1を包含 |
| 2 | 曖昧さ排除 | OK | 各 Phase の変更内容が具体的コードスニペットで示されている |
| 3 | 設計判断の完結性 | OK | 候補1→3 変更の根拠（型 vs ロジック）、users 配置の判断に選択肢・根拠・トレードオフを記載 |
| 4 | スコープ境界 | OK | 対象（候補2,3,4）・対象外（候補1=候補3で包含, #854=別Issue）を明記 |
| 5 | 技術的前提 | OK | CancelledState は instance.rs のみで使用（Grep 確認済み）。Elm の ADT パターンは New.elm で確認済み |
| 6 | 既存ドキュメント整合 | OK | ADR-054「不正な状態遷移を型レベルで防止」の判断基準に合致 |

## 検証

- Phase 1: `just check`（backend のリント + テスト）
- Phase 2: `just check`（frontend のコンパイル）
- 全体: `just check-all`（全体のリント + テスト）
