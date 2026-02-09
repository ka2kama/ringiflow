# #334 フィールド単位アサーション → 全体比較への移行

## Context

テストのフィールド単位アサーション（`assert_eq!(result.status(), ...)`）は、新フィールド追加時に既存テストが壊れず、バグが暗黙にすり抜ける構造的な問題がある。エンティティに `PartialEq` を derive し、`assert_eq!(result, expected)` の全体比較に移行することで、テストの自己メンテナンス性を確保する。

## 設計判断

### 1. `PartialEq` + `Eq` を derive

全 Value Object は既に `PartialEq, Eq` を derive 済み。エンティティのフィールドに浮動小数点型はない（`DateTime<Utc>`, `serde_json::Value` はすべて `Eq` 実装済み）。既存パターンとの一貫性のため両方を derive する。

### 2. テスト移行パターン

| パターン | 対象 | 方法 |
|---------|------|------|
| A: 状態変更の検証 | `with_status`, `approve`, `submitted` 等 | 変更前を `clone()` し、`from_db` で期待値を構築 → 全体比較 |
| B: ブーリアン/異常系 | `is_active()`, `is_err()` 等 | 移行しない（テストの意図が全体比較で不明確になる） |
| C: テスト統合 | 同一操作を別フィールドで検証するテスト群 | 全体比較に統合（version テスト + ステータステスト → 1テスト） |

### 3. ユースケース層の対象範囲

ユースケース層テストは `DashboardStats`（全フィールド 0 のテスト）のみ全体比較に移行する。

除外理由: task.rs / workflow.rs のユースケーステストは `Utc::now()` を使って Mock データを構築しており、テスト実行中の時刻が厳密に制御されていない。全体比較に移行するには Clock DI（`Clock` trait 等）の導入が前提。これは別 Issue のスコープ。

### 4. Phase 構成

| Phase | 内容 |
|-------|------|
| Phase 1 | `PartialEq, Eq` の derive 追加（全エンティティ + `DashboardStats`） |
| Phase 2 | ドメイン層テストの全体比較移行 |
| Phase 3 | ユースケース層テスト（`DashboardStats`）の全体比較移行 |

## Phase 1: PartialEq, Eq の derive 追加

**対象:**

| ファイル | 構造体 | 行 |
|---------|--------|-----|
| `backend/crates/domain/src/user.rs` | `User` | 194 |
| `backend/crates/domain/src/tenant.rs` | `Tenant` | 204 |
| `backend/crates/domain/src/workflow.rs` | `WorkflowDefinition` | 110 |
| `backend/crates/domain/src/workflow.rs` | `WorkflowInstance` | 333 |
| `backend/crates/domain/src/workflow.rs` | `WorkflowStep` | 716 |
| `backend/crates/domain/src/role.rs` | `Role` | 117 |
| `backend/crates/domain/src/role.rs` | `UserRole` | 250 |
| `backend/apps/core-service/src/usecase/dashboard.rs` | `DashboardStats` | 25 |

変更: `#[derive(Debug, Clone)]` → `#[derive(Debug, Clone, PartialEq, Eq)]`
（`DashboardStats` は `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`）

テストリスト:
- [ ] 既存テストが全て通ることを確認（`cargo test -p ringiflow-domain` + `cargo test -p ringiflow-core-service`）

## Phase 2: ドメイン層テストの全体比較移行

### user.rs（3テスト移行）

移行対象:

| テスト名 | 現在 | 移行後 |
|---------|------|--------|
| `test_ステータス変更で状態が更新される` | `status()`, `updated_at()` | `User::from_db(...)` で expected 構築 → 全体比較 |
| `test_削除されたユーザーのステータスは削除済み` | `status()`, `updated_at()` | 同上 |
| `test_最終ログイン日時を更新できる` | `last_login_at()`, `updated_at()` | 同上 |

移行しない:
- `test_新規ユーザーはアクティブ状態` — `is_active()` ロジックテスト
- `test_新規ユーザーはログイン可能` — `can_login()` ロジックテスト
- `test_新規ユーザーは最終ログイン日時なし` — 単一フィールド None チェック
- `test_新規ユーザーのcreated_atとupdated_atは注入された値と一致する` — 初期状態の入出力確認
- `test_非アクティブユーザーはアクティブでない` — `is_active()` ロジックテスト
- `test_削除されたユーザーはログインできない` — `can_login()` ロジックテスト
- `test_ユーザーから表示用連番を取得できる` — getter テスト
- Email 関連テスト 3 件 — バリデーションテスト

テストリスト:
- [ ] `test_ステータス変更で全フィールドが正しく更新される` — `with_status` 後に `from_db` 期待値と全体比較
- [ ] `test_削除で全フィールドが正しく更新される` — `deleted` 後に `from_db` 期待値と全体比較
- [ ] `test_最終ログイン日時更新で全フィールドが正しく更新される` — `with_last_login_updated` 後に `from_db` 期待値と全体比較

### tenant.rs（1テスト移行）

移行対象:

| テスト名 | 現在 | 移行後 |
|---------|------|--------|
| `test_from_dbでテナントを復元できる` | `id()`, `name().as_str()` | `Tenant::from_db(...)` で expected 構築 → 全体比較 |

テストリスト:
- [ ] `test_from_dbでテナントを復元できる` — `from_db` で構築した Tenant と全体比較

### workflow.rs — workflow_instance（テスト統合あり）

移行 + 統合:

| 元テスト | 統合後 |
|---------|--------|
| `test_新規作成時にversionは1` + `test_新規作成時のcreated_atとupdated_atは注入された値と一致する` | `test_新規作成の初期状態が正しい` |
| `test_承認完了でステータスが承認済みになる` + `test_承認完了でversionがインクリメントされる` | `test_承認完了で全フィールドが正しく更新される` |
| `test_却下完了でステータスが却下済みになる` + `test_却下完了でversionがインクリメントされる` | `test_却下完了で全フィールドが正しく更新される` |

単独移行:

| テスト名 | 移行後 |
|---------|--------|
| `test_申請後のsubmitted_atは注入された値と一致する` | `test_申請で全フィールドが正しく更新される` |
| `test_下書きからの取消でキャンセルになる` | `test_下書きからの取消で全フィールドが正しく更新される` |
| `test_申請済みからの取消でキャンセルになる` | `test_申請済みからの取消で全フィールドが正しく更新される` |
| `test_処理中からの取消でキャンセルになる` | `test_処理中からの取消で全フィールドが正しく更新される` |

移行しない: 異常系テスト 7 件（`is_err()` チェック）

テストリスト:
- [ ] `test_新規作成の初期状態が正しい` — `from_db(WorkflowInstanceRecord {...})` 期待値と全体比較
- [ ] `test_申請で全フィールドが正しく更新される` — `submitted` 後に全体比較
- [ ] `test_承認完了で全フィールドが正しく更新される` — `complete_with_approval` 後に全体比較
- [ ] `test_却下完了で全フィールドが正しく更新される` — `complete_with_rejection` 後に全体比較
- [ ] `test_下書きからの取消で全フィールドが正しく更新される` — `cancelled` 後に全体比較
- [ ] `test_申請済みからの取消で全フィールドが正しく更新される` — `cancelled` 後に全体比較
- [ ] `test_処理中からの取消で全フィールドが正しく更新される` — `cancelled` 後に全体比較

### workflow.rs — workflow_step（テスト統合あり）

移行 + 統合:

| 元テスト | 統合後 |
|---------|--------|
| `test_新規作成時にversionは1` + `test_新規作成時のcreated_atとupdated_atは注入された値と一致する` | `test_新規作成の初期状態が正しい` |
| `test_承認で完了と承認済みになる` + `test_承認でversionがインクリメントされる` | `test_承認で全フィールドが正しく更新される` |
| `test_却下で完了と却下済みになる` + `test_却下でversionがインクリメントされる` | `test_却下で全フィールドが正しく更新される` |

単独移行:

| テスト名 | 移行後 |
|---------|--------|
| `test_アクティブ化後のstarted_atは注入された値と一致する` | `test_アクティブ化で全フィールドが正しく更新される` |
| `test_承認でコメントが設定される` | `test_コメント付き承認で全フィールドが正しく更新される` |
| `test_差戻しで完了と差戻しになる` | `test_差戻しで全フィールドが正しく更新される` |

移行しない: 異常系 2 件 + ブーリアンテスト 2 件

テストリスト:
- [ ] `test_新規作成の初期状態が正しい` — `from_db(WorkflowStepRecord {...})` 期待値と全体比較
- [ ] `test_アクティブ化で全フィールドが正しく更新される` — `activated` 後に全体比較
- [ ] `test_承認で全フィールドが正しく更新される` — `approve(None, now)` 後に全体比較
- [ ] `test_コメント付き承認で全フィールドが正しく更新される` — `approve(Some(...), now)` 後に全体比較
- [ ] `test_却下で全フィールドが正しく更新される` — `reject(None, now)` 後に全体比較
- [ ] `test_差戻しで全フィールドが正しく更新される` — `completed(RequestChanges, ...)` 後に全体比較

### workflow.rs — workflow_definition（2テスト移行）

テストリスト:
- [ ] `test_公開で全フィールドが正しく更新される` — `published` 後に `from_db(WorkflowDefinitionRecord {...})` 期待値と全体比較
- [ ] `test_アーカイブで全フィールドが正しく更新される` — `archived` 後に全体比較

### role.rs（1テスト移行）

テストリスト:
- [ ] `test_ロールの初期状態が正しい` — `from_db` 期待値と全体比較（`test_ロールのcreated_atは注入された値と一致する` を移行）

### pretty_assertions の統一

`workflow.rs` と `tenant.rs` のテストモジュールに `use pretty_assertions::assert_eq;` を追加。
- `user.rs`: 追加済み
- `role.rs`: 追加済み

## Phase 3: ユースケース層テスト（DashboardStats）

テストリスト:
- [ ] `test_タスクがない場合はすべて0を返す` — `DashboardStats { pending_tasks: 0, my_workflows_in_progress: 0, completed_today: 0 }` と全体比較

他のダッシュボードテスト（`test_承認待ちタスク数が...` 等）は「特定条件で特定フィールドの値を検証する」意図であり、全体比較に移行しない。

## 対象外

- ユースケース層テスト（task.rs, workflow.rs）— `Utc::now()` の時刻制御問題。Clock DI 導入後に別 Issue で対応
- 値オブジェクトのバリデーションテスト
- ブーリアンロジックテスト（`is_active()`, `can_login()`, `is_overdue()` 等）
- 異常系テスト（`is_err()` 等）

## 実装時の注意事項

1. `from_db` のシグネチャが2パターンある:
   - 個別引数: `User::from_db(...)`, `Tenant::from_db(...)`, `Role::from_db(...)`, `UserRole::from_db(...)`
   - Record 構造体: `WorkflowInstance::from_db(WorkflowInstanceRecord {...})`, `WorkflowStep::from_db(WorkflowStepRecord {...})`, `WorkflowDefinition::from_db(WorkflowDefinitionRecord {...})`
2. 状態変更メソッドは `self` を consume する。変更前フィールドを参照するには先に `clone()` が必要
3. フィクスチャの `UserId::new()` 等は毎回新しい UUID を生成する。期待値構築時はフィクスチャから取得したエンティティの ID を `clone()` して使う

## 検証

```bash
cd backend && cargo test -p ringiflow-domain    # Phase 1-2
cd backend && cargo test -p ringiflow-core-service  # Phase 3
just check-all                                      # 全体
```

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → PartialEq 対応確認 | `serde_json::Value`, `DateTime<Utc>`, `Uuid` の `Eq` 実装 | 全フィールドが `PartialEq+Eq` 対応済み。コンパイルリスクなし |
| 2回目 | ユースケース層の実現可能性 | task.rs / workflow.rs の `Utc::now()` 使用箇所 | 時刻制御不可のため大部分を除外。`DashboardStats` のみ対象 |
| 3回目 | テスト統合の適切性 | version テストと状態変更テストの重複分析 | 全体比較で version も検証されるため統合が適切 |
| 4回目 | `from_db` シグネチャ差異 | User（個別引数）vs WorkflowInstance（Record 構造体）| 両パターンの使い分けを注意事項に記載 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | 7 エンティティ + DashboardStats。ドメイン 4 ファイル + ユースケース 1 ファイルの全テストを分析し、移行対象/非対象を判定 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各テストの移行対象/非対象を理由付きで判定。テスト統合の対象を明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | PartialEq vs Eq、テスト移行パターン A/B/C、テスト統合、ユースケース層除外の判断根拠を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 「対象外」セクションで除外範囲と理由を明示 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | Eq 実装確認、from_db シグネチャ差異、clone() 必要性、UUID 生成の非決定性を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #334 のスコープ 4 項目と整合。DDD エンティティ同一性と PartialEq の区別を Issue が明記済み |
