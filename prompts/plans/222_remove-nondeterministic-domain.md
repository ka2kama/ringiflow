# Issue #222: ドメインモデルから非決定的な値の生成を排除する

## 概要

ドメインモデル内の `Utc::now()` / `Uuid::now_v7()` を排除し、呼び出し元から注入する形に変更する。
Functional Core, Imperative Shell パターンの適用。

API 変更なし・機能変更なし（純粋なリファクタリング）。

## ブランチ

`feature/222-remove-non-deterministic-from-domain`

## Phase 構成

2 Phase に分割（エンティティ単位）:

| Phase | 対象 | 規模 |
|-------|------|------|
| Phase 1 | User + Role | 小（パターン確立） |
| Phase 2 | Workflow（Definition + Instance + Step） | 大（本丸） |

## Phase 1: User + Role

### 変更対象

**`backend/crates/domain/src/user.rs`**

| メソッド | 追加パラメータ |
|---------|--------------|
| `User::new()` | `id: UserId`, `now: DateTime<Utc>` |
| `User::with_last_login_updated()` | `now: DateTime<Utc>` |
| `User::with_status()` | `now: DateTime<Utc>` |
| `User::deleted()` | `now: DateTime<Utc>` |

+ doc example (L20-36) 更新

**`backend/crates/domain/src/role.rs`**

| メソッド | 追加パラメータ |
|---------|--------------|
| `Role::new_system()` | `id: RoleId`, `now: DateTime<Utc>` |
| `Role::new_tenant()` | `id: RoleId`, `now: DateTime<Utc>` |
| `UserRole::new()` | `id: Uuid`, `now: DateTime<Utc>` |

+ doc example (L27-37) 更新

### テストリスト（Phase 1）

既存テスト更新 + タイムスタンプ検証追加:

- [ ] `test_新規ユーザーはアクティブ状態` — フィクスチャに id, now を渡す
- [ ] `test_新規ユーザーはログイン可能` — 同上
- [ ] `test_新規ユーザーは最終ログイン日時なし` — 同上
- [ ] `test_ステータス変更で状態が更新される` — now を渡し updated_at == now を検証
- [ ] `test_非アクティブユーザーはアクティブでない` — now を渡す
- [ ] `test_削除されたユーザーのステータスは削除済み` — now を渡し updated_at == now を検証
- [ ] `test_削除されたユーザーはログインできない` — now を渡す
- [ ] `test_最終ログイン日時を更新できる` — now を渡し last_login_at == Some(now) を検証
- [ ] `test_新規ユーザーのcreated_atとupdated_atは注入された値と一致する` — 新規
- [ ] `test_システムロール/テナントロール` — フィクスチャに id, now を渡す
- [ ] `test_ロールのcreated_atは注入された値と一致する` — 新規

### 呼び出し元の更新（Phase 1）

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/core-service/src/handler/auth.rs` テスト (L335) | `Role::new_system()` に id, now を追加 |

注: `user_repository_test.rs` は `User::new()` を使用していない（raw SQL でデータ挿入）ため変更不要。

### コミット

`Inject id and now into User and Role constructors`

## Phase 2: Workflow

### 変更対象

**`backend/crates/domain/src/workflow.rs`**

WorkflowDefinition:

| メソッド | 追加パラメータ |
|---------|--------------|
| `WorkflowDefinition::new()` | `id: WorkflowDefinitionId`, `now: DateTime<Utc>` |
| `WorkflowDefinition::published()` | `now: DateTime<Utc>` |
| `WorkflowDefinition::archived()` | `now: DateTime<Utc>` |

WorkflowInstance:

| メソッド | 追加パラメータ |
|---------|--------------|
| `WorkflowInstance::new()` | `id: WorkflowInstanceId`, `now: DateTime<Utc>` |
| `WorkflowInstance::submitted()` | `now: DateTime<Utc>` |
| `WorkflowInstance::approved()` | `now: DateTime<Utc>` |
| `WorkflowInstance::rejected()` | `now: DateTime<Utc>` |
| `WorkflowInstance::cancelled()` | `now: DateTime<Utc>` |
| `WorkflowInstance::with_current_step()` | `now: DateTime<Utc>` |
| `WorkflowInstance::complete_with_approval()` | `now: DateTime<Utc>` |
| `WorkflowInstance::complete_with_rejection()` | `now: DateTime<Utc>` |

WorkflowStep:

| メソッド | 追加パラメータ |
|---------|--------------|
| `WorkflowStep::new()` | `id: WorkflowStepId`, `now: DateTime<Utc>` |
| `WorkflowStep::activated()` | `now: DateTime<Utc>` |
| `WorkflowStep::completed()` | `now: DateTime<Utc>` |
| `WorkflowStep::skipped()` | `now: DateTime<Utc>` |
| `WorkflowStep::approve()` | `now: DateTime<Utc>` |
| `WorkflowStep::reject()` | `now: DateTime<Utc>` |
| `WorkflowStep::is_overdue()` | `now: DateTime<Utc>` |

+ doc example (L12-29) 更新

### テストリスト（Phase 2）

WorkflowInstance テスト更新:

- [ ] `test_新規作成時にversionは1` — id, now を渡す
- [ ] `test_承認完了でステータスがApprovedになる` — now を渡し completed_at == Some(now) を検証
- [ ] `test_承認完了でversionがインクリメントされる` — now を渡す
- [ ] `test_却下完了でステータスがRejectedになる` — now を渡し completed_at を検証
- [ ] `test_却下完了でversionがインクリメントされる` — now を渡す
- [ ] `test_InProgress以外で承認完了するとエラー` — now を渡す
- [ ] `test_InProgress以外で却下完了するとエラー` — now を渡す
- [ ] `test_submitted後のsubmitted_atは注入された値と一致する` — 新規
- [ ] `test_新規作成時のidは注入された値と一致する` — 新規

WorkflowStep テスト更新:

- [ ] `test_新規作成時にversionは1` — id, now を渡す
- [ ] `test_approveでCompletedとApprovedになる` — now を渡し completed_at == Some(now) を検証
- [ ] `test_approveでversionがインクリメントされる` — now を渡す
- [ ] `test_approveでコメントが設定される` — now を渡す
- [ ] `test_rejectでCompletedとRejectedになる` — now を渡す
- [ ] `test_rejectでversionがインクリメントされる` — now を渡す
- [ ] `test_Active以外でapproveするとエラー` — now を渡す
- [ ] `test_Active以外でrejectするとエラー` — now を渡す
- [ ] `test_activated後のstarted_atは注入された値と一致する` — 新規
- [ ] `test_is_overdue_期限切れの場合trueを返す` — 新規
- [ ] `test_is_overdue_期限内の場合falseを返す` — 新規

### 呼び出し元の更新（Phase 2）

ユースケース層（プロダクションコード）:

| ファイル | パターン |
|---------|---------|
| `backend/apps/core-service/src/usecase/workflow.rs` | 各メソッド冒頭で `let now = Utc::now();` を1回取得し、全ドメイン操作に渡す |

テストコード:

| ファイル | `new()` 呼び出し | 状態遷移呼び出し |
|---------|-----------------|----------------|
| `backend/apps/core-service/src/usecase/workflow.rs` テスト | Instance 6 + Step 5 + Definition 2 | `submitted` 5, `with_current_step` 5, `activated` 4, `published` 2 |
| `backend/apps/core-service/src/usecase/task.rs` テスト | Instance 5 + Step 5 | `submitted` 5, `with_current_step` 5, `activated` 5 |
| `backend/apps/core-service/src/usecase/dashboard.rs` テスト | Instance 6 + Step 4 | `submitted` 5, `with_current_step` 4, `activated` 3, `approved` 1, `approve` 1 |
| `backend/crates/infra/tests/workflow_instance_repository_test.rs` | Instance 11 | `submitted` 2 |
| `backend/crates/infra/tests/workflow_step_repository_test.rs` | Instance 7 + Step 8 | `activated` 3, `completed` 1 |

### コミット戦略（Phase 2）

エンティティ単位で分割（シグネチャ変更は全呼び出し元を同時更新する必要あり）:

1. `Inject id and now into WorkflowDefinition`
2. `Inject id and now into WorkflowInstance`
3. `Inject id and now into WorkflowStep`

## 設計判断

| 判断 | 結論 | 理由 |
|------|------|------|
| `is_overdue()` | `now` を引数で受ける | テスタビリティ + Functional Core の一貫性 |
| ID 型の `new()` | 維持 | 便利メソッドとして残す。エンティティコンストラクタが外から受け取るだけ |
| `UserRole.id`（bare `Uuid`） | `id: Uuid` を引数追加 | newtype 化は別スコープ |

## 対象外

- `XxxId::new()` メソッド自体の削除（便利メソッドとして残す）
- テストフィクスチャ内の `Uuid::now_v7()`（テスト専用コード）
- セキュリティトークン（CSRF、セッション ID）

## 検証

```bash
just check-all   # lint + test + API test
```

各 Phase のコミット後に `just check-all` が通ることを確認。

## 自己検証（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | domain 内の `Utc::now()` 27箇所、`Uuid::now_v7()` 7箇所を全数確認。全呼び出し元を grep で実数確認（WorkflowInstance::new 37箇所、WorkflowStep::new 24箇所、状態遷移 57箇所）。対象外は理由付きで記載 |
| 2 | 曖昧さ排除 | OK | 初版にあった「あれば」を排除（user_repository_test.rs は User::new() 使用なしと確定）。パス誤り（bff → core-service）を修正。概算値を実数に置換 |
| 3 | 設計判断の完結性 | OK | `is_overdue()` の `now` 引数化、ID 型 `new()` 維持、`UserRole.id` の bare Uuid 対応、すべて判断と理由を記載 |
| 4 | スコープ境界 | OK | 対象（ドメインコンストラクタ + 状態遷移）と対象外（ID 型 `new()`, テストフィクスチャ, セキュリティトークン）を明記 |
| 5 | 技術的前提 | OK | Rust シグネチャ変更の連鎖（全呼び出し元の同時更新が必要）、doc test の更新を考慮 |
| 6 | 既存ドキュメント整合 | OK | API 変更なし（OpenAPI 更新不要）。設計書・ADR との矛盾なし |
