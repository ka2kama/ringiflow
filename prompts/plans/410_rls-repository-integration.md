# #410 RLS リポジトリ統合: 既存リポジトリの RLS 対応

## Context

Epic #402（マルチテナント RLS 実装）の Story 4。Story 1-3 で以下が完了済み:
- Story 1 (#407): `workflow_steps` と `user_roles` に `tenant_id` カラム追加、全テーブルに RLS ポリシー設定
- Story 2 (#408): `after_release` フック + `TenantConnection` 型
- Story 3 (#409): クロステナントアクセス防止の統合テスト

本 Issue では、新しい `tenant_id` カラムを活用するよう既存リポジトリのクエリを更新する。

## Issue 精査

| 観点 | 検証結果 |
|------|---------|
| Want | マルチテナントの二重防御（アプリ層 + DB 層）を全クエリで正しく機能させる |
| How への偏り | なし。Issue のスコープは適切 |
| 完了基準の妥当性 | 6項目すべて Want に直結。E2E 基準は不要（インフラ層の変更のため） |
| 暗黙の仮定 | `workflow_steps.tenant_id` のバックフィルが正しいこと → Story 1 で完了済み |
| スコープの適切さ | 適切。リポジトリ層のクエリ更新に絞られている |

## スコープ

**対象**:
- `workflow_step_repository.rs`: SELECT クエリの JOIN 排除（直接 `tenant_id` 参照化）、UPDATE の `tenant_id` チェック追加
- `user_repository.rs`: `find_with_roles` の user_roles クエリに `tenant_id` フィルタ追加
- 上記に伴う Mock、呼び出し元、テストの更新

**対象外**:
- `WorkflowInstanceRepository::update_with_version_check` への `tenant_id` 追加（`workflow_instances` は元々 `tenant_id` カラムがあり RLS が効いている。UPDATE の二重防御追加は別 Issue）
- `UserRepository::find_by_id` / `find_by_ids` / `update_last_login` への `tenant_id` 追加（内部 API 向けでテナント ID を検証しない設計。コメントに明記済み）
- RLS ポリシー自体の変更（Story 1 で完了済み）

## 設計判断

### 1. SELECT クエリの JOIN → 直接カラム参照

| 方式 | 判定 | 理由 |
|------|------|------|
| **A: `s.tenant_id = $N` で直接参照（採用）** | ✅ | `workflow_instance_repository` と一貫、JOIN 不要でシンプル、パフォーマンス向上 |
| B: JOIN を維持 | ❌ | JOIN は `tenant_id` カラムがなかった時代のワークアラウンド。カラム追加後も維持する理由がない |

### 2. `find_with_roles` のシグネチャ

| 方式 | 判定 | 理由 |
|------|------|------|
| **A: シグネチャ変更なし、内部で `user.tenant_id()` を使用（採用）** | ✅ | 呼び出し元（auth.rs ハンドラ）に tenant_id がなく、変更不要。Mock 変更もゼロ |
| B: `tenant_id: &TenantId` を引数追加 | ❌ | auth.rs ハンドラの変更が必要。user_id からユーザーを取得済みなので冗長 |

`find_with_roles` は内部で `find_by_id(id)` → `user` を取得し、`user.tenant_id()` を使って user_roles クエリをフィルタする。

## Phase 1: WorkflowStepRepository SELECT クエリの直接参照化

### 確認事項
- 型: `WorkflowStepRepository` トレイト定義 → `workflow_step_repository.rs:27-41`（確認済み）
- パターン: `workflow_instance_repository.rs` の SELECT パターン（直接 `tenant_id` フィルタ）（確認済み）
- ライブラリ: sqlx `query!` マクロのカラム名解決 — テーブルエイリアス `s.id` も非エイリアス `id` も同じフィールド名 `id` で返される（PostgreSQL の動作）

### テストリスト

SQL の内部変更のみで外部振る舞いは変わらない。既存テスト全パスが目標。

- [ ] `test_find_by_id_でステップを取得できる` — 既存テスト green
- [ ] `test_find_by_id_存在しない場合はnoneを返す` — 既存テスト green
- [ ] `test_find_by_instance_インスタンスのステップ一覧を取得できる` — 既存テスト green
- [ ] `test_find_by_instance_別テナントのステップは取得できない` — 既存テスト green
- [ ] `test_find_by_assigned_to_担当者のタスク一覧を取得できる` — 既存テスト green
- [ ] `test_find_by_display_number_存在するdisplay_numberで検索できる` — 既存テスト green
- [ ] `test_find_by_display_number_存在しない場合はnoneを返す` — 既存テスト green
- [ ] `test_find_by_display_number_別のinstance_idでは見つからない` — 既存テスト green

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/src/repository/workflow_step_repository.rs` | 4つの SELECT クエリの JOIN 排除 + `s.tenant_id` 直接参照 |

### 具体的な SQL 変更

**find_by_id** (L172-187):
```sql
-- Before
FROM workflow_steps s
INNER JOIN workflow_instances i ON s.instance_id = i.id
WHERE s.id = $1 AND i.tenant_id = $2

-- After
FROM workflow_steps
WHERE id = $1 AND tenant_id = $2
```

**find_by_instance** (L228-243):
```sql
-- Before
FROM workflow_steps s
INNER JOIN workflow_instances i ON s.instance_id = i.id
WHERE s.instance_id = $1 AND i.tenant_id = $2

-- After
FROM workflow_steps
WHERE instance_id = $1 AND tenant_id = $2
```

**find_by_assigned_to** (L284-299):
```sql
-- Before
FROM workflow_steps s
INNER JOIN workflow_instances i ON s.instance_id = i.id
WHERE i.tenant_id = $1 AND s.assigned_to = $2

-- After
FROM workflow_steps
WHERE tenant_id = $1 AND assigned_to = $2
```

**find_by_display_number** (L341-356):
```sql
-- Before
FROM workflow_steps s
INNER JOIN workflow_instances i ON s.instance_id = i.id
WHERE s.display_number = $1 AND s.instance_id = $2 AND i.tenant_id = $3

-- After
FROM workflow_steps
WHERE display_number = $1 AND instance_id = $2 AND tenant_id = $3
```

注: テーブルエイリアス `s.` プレフィックスも不要になる（単一テーブルクエリ）。SELECT リストのカラム名もエイリアスなしに変更。sqlx の返却フィールド名はカラム名と同一なので、Rust 側のマッピングコード（`r.id` 等）は変更不要。

## Phase 2: WorkflowStepRepository `update_with_version_check` に `tenant_id` 追加

### 確認事項
- 型: `update_with_version_check` シグネチャ → `workflow_step_repository.rs:37-41`（確認済み）
- パターン: 呼び出し元 `approve_step`/`reject_step` → `command.rs:266, 363`（確認済み、両方とも `tenant_id` がスコープ内）
- Mock: 3箇所 — `command.rs:750`, `task.rs:371`, `dashboard.rs:248`（確認済み）

### テストリスト

**新規テスト (Red → Green)**:
- [ ] `test_update_with_version_check_別テナントのステップは更新できない` — tenant_id 不一致で rows_affected=0 → Conflict エラー

**既存テスト更新（`tenant_id` 引数追加）**:
- [ ] `test_update_with_version_check_バージョン一致で更新できる` — `&tenant_id` 引数追加、green
- [ ] `test_update_with_version_check_バージョン不一致でconflictエラーを返す` — `&tenant_id` 引数追加、green
- [ ] `test_ステップを完了できる` — `&tenant_id` 引数追加、green

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/src/repository/workflow_step_repository.rs` | トレイト + 実装に `tenant_id: &TenantId` 追加、SQL に `AND tenant_id = $10` |
| `backend/crates/infra/tests/workflow_step_repository_test.rs` | 既存テストに `&tenant_id` 追加 + 新規テスト |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | 呼び出し2箇所 (L266, L363) に `&tenant_id` 追加 + Mock (L750) シグネチャ更新 |
| `backend/apps/core-service/src/usecase/task.rs` | Mock (L371) シグネチャ更新 |
| `backend/apps/core-service/src/usecase/dashboard.rs` | Mock (L248) シグネチャ更新 |

### 具体的な変更

**トレイトシグネチャ** (L37-41):
```rust
// Before
async fn update_with_version_check(
    &self,
    step: &WorkflowStep,
    expected_version: Version,
) -> Result<(), InfraError>;

// After
async fn update_with_version_check(
    &self,
    step: &WorkflowStep,
    expected_version: Version,
    tenant_id: &TenantId,
) -> Result<(), InfraError>;
```

**SQL** (L132-142):
```sql
-- Before
WHERE id = $8 AND version = $9

-- After
WHERE id = $8 AND version = $9 AND tenant_id = $10
```

バインドパラメータに `tenant_id.as_uuid()` を追加。

## Phase 3: UserRepository `find_with_roles` の `tenant_id` フィルタ

### 確認事項
- 型: `find_with_roles` シグネチャ → `user_repository.rs:56`（確認済み、変更なし）
- パターン: `find_with_roles` 内部 → `find_by_id(id)` で `user` 取得後に `user.tenant_id()` 使用可能（L181-185 確認済み）
- ライブラリ: sqlx `query!` のバインドパラメータ追加 → 既存使用パターンで確認済み

### テストリスト

**新規テスト (Red → Green)**:
- [ ] `test_find_with_roles_別テナントのロール割り当ては含まれない` — user_roles に別テナントの割り当てがあっても返さない

**既存テスト (Green のまま)**:
- [ ] `test_ユーザーとロールを一緒に取得できる` — 既存テスト green

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/src/repository/user_repository.rs` | ロール取得クエリに `AND ur.tenant_id = $2` 追加 |
| `backend/crates/infra/tests/user_repository_test.rs` | 新規テスト追加 |

### 具体的な変更

**SQL** (L188-206):
```rust
// Before
sqlx::query!(
    r#"
    SELECT r.id, r.tenant_id, r.name, ...
    FROM roles r
    INNER JOIN user_roles ur ON ur.role_id = r.id
    WHERE ur.user_id = $1
    "#,
    id.as_uuid()
)

// After
sqlx::query!(
    r#"
    SELECT r.id, r.tenant_id, r.name, ...
    FROM roles r
    INNER JOIN user_roles ur ON ur.role_id = r.id
    WHERE ur.user_id = $1 AND ur.tenant_id = $2
    "#,
    id.as_uuid(),
    user.tenant_id().as_uuid()
)
```

トレイトシグネチャ変更なし。Mock・呼び出し元の変更なし。

## Phase 4: sqlx-prepare と最終確認

### 確認事項: なし（既知のパターンのみ）

### タスクリスト
- [ ] `just sqlx-prepare` でキャッシュ更新
- [ ] `just check-all` でリント + テスト + API テスト全パス
- [ ] Issue #410 の完了基準チェック

## 検証方法

1. 各 Phase のテスト: `cd backend && cargo test --package ringiflow-infra`
2. Phase 4: `just check-all`（リント + テスト + API テスト）
3. 完了基準の突合: Issue #410 の6項目をすべて確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `WorkflowInstanceRepository::update_with_version_check` も tenant_id なしだがスコープ外か | スコープ境界 | Issue #410 のスコープは `workflow_steps` と `user_roles`。`workflow_instances` は元々 tenant_id があり RLS が効いているため別 Issue。対象外に明記 |
| 2回目 | `find_with_roles` のシグネチャ変更要否。auth.rs ハンドラに tenant_id がない | 不完全なパス | 内部で `user.tenant_id()` を使うことでシグネチャ変更不要と判断。Mock・呼び出し元の変更ゼロ |
| 3回目 | SELECT のテーブルエイリアス削除時、sqlx マクロのフィールド名への影響 | 技術的前提 | PostgreSQL のカラム名解決により、エイリアス `s.id` も非エイリアス `id` も同じフィールド名で返される。マッピングコード変更不要 |
| 4回目 | Mock が3箇所（command.rs, task.rs, dashboard.rs）で、当初 dashboard.rs を見落としていた | 網羅性 | `impl WorkflowStepRepository for` で Grep し3箇所を確認。Phase 2 のファイル変更に dashboard.rs を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue #410 の完了基準が全て計画に含まれている | OK | INSERT の tenant_id（既に正しい）、全クエリの tenant_id（Phase 1-3）、既存テスト（全 Phase）、sqlx-prepare（Phase 4）、check-all（Phase 4） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の SQL 変更を具体的なコードスニペットで記載。設計判断は理由付きで確定 |
| 3 | 設計判断の完結性 | 全ての判断に理由が記載されている | OK | JOIN 排除（一貫性+シンプルさ）、find_with_roles シグネチャ不変（ハンドラ影響回避）、対象外の理由を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象/対象外セクションに明記。find_by_id 等が対象外の理由も記載 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | sqlx のカラム名解決、PostgreSQL エイリアス動作をループ3回目で確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Epic 計画 (402_multi-tenant-rls.md) の Story 4 設計判断と整合。Issue #410 の完了基準6項目すべてに対応 |
