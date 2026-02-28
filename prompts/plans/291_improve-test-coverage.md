# Issue #291: テストカバレッジ改善計画

## Context

Issue #291 はテストカバレッジの現状把握と改善を目的とする。現在 203+ テストが存在するが、Domain 層のエッジケース（特に取消・無効な状態遷移）、Handler 層、API テスト（Hurl）にカバレッジの穴がある。この計画では4つの完了基準すべてに対応する。

## Issue 精査結果

| 観点 | 判断 |
|------|------|
| Want | コードの信頼性向上とリファクタリング（#290）の安全ネット構築 |
| スコープ | 計測 + 分析 + テスト追加 + 突合表。1 PR で対応可能な範囲に絞る |
| 完了基準 #3 の解釈 | 「優先度の高いテスト」= Domain 層のエッジケーステスト（ビジネスルール直結、純粋ユニットテスト、既存パターン踏襲で高効率） |

## 対象

- カバレッジ計測ツール（cargo-llvm-cov）のセットアップ
- Domain 層のエッジケーステスト追加
- API テスト ↔ OpenAPI 突合表の作成
- Handler 層の評価・文書化

## 対象外

- Usecase 層のエラーパステスト追加（→ フォローアップ Issue）
- API テスト（Hurl）の追加（→ フォローアップ Issue）
- CI へのカバレッジ計測統合（→ フォローアップ Issue）

## 設計判断

### 1. カバレッジ計測ツール: cargo-llvm-cov

| ツール | 方式 | 精度 | メンテナンス |
|--------|------|------|-------------|
| **cargo-llvm-cov（採用）** | Rust native `-C instrument-coverage` (LLVM) | 高（ソースベース） | taiki-e が活発にメンテナンス |
| cargo-tarpaulin | ptrace (Linux only) | 中（行ベース、複雑なコードで不正確） | メンテナンスは継続しているが LLVM ベースに比べ旧式 |

選定理由: Rust 公式が推奨する LLVM ソースベースカバレッジを使用。cargo-tarpaulin より精度が高く、クロスプラットフォーム対応。

### 2. Handler 層の評価

**結論: BFF Handler にユニットテストは不要。API テスト（Hurl）で十分。**

根拠:
- BFF Handler は薄いプロキシ層: セッション取得 → Core Service クライアント呼び出し → レスポンスマッピング
- ビジネスロジックは Core Service の Usecase 層に集約
- BFF auth handler は統合テスト（12 件）でカバー済み。このパターンが他の handler にも適用可能
- Hurl API テストでセッション管理・CSRF・エラーマッピングを E2E で検証可能

Core Service Handler も同様にユニットテスト不要:
- Handler は Usecase を呼び出して DTO に変換するだけ
- DTO 変換は API テストでカバーすれば十分

この判断は Issue クローズ時の振り返りコメントに記録する。

### 3. Domain エッジケーステストの優先順位

| 優先度 | 対象 | テスト数 | 根拠 |
|--------|------|---------|------|
| 高 | WorkflowInstance `cancelled()` | 6 件 | メソッドに Result 型のバリデーションあり、テストゼロ |
| 高 | WorkflowInstance `submitted()` 異常系 | 2 件 | Draft 以外からの申請（バリデーションロジック未テスト） |
| 中 | WorkflowDefinition `published()` / `archived()` | 3 件 | 状態遷移のバリデーション |
| 中 | WorkflowStep `completed()` with RequestChanges | 1 件 | 3つの StepDecision のうち RequestChanges のみ未テスト |

## Phase 構成

### Phase 1: カバレッジ計測ツールセットアップ

- `cargo-llvm-cov` を justfile に追加（`just coverage` タスク）
- `check-tools` に cargo-llvm-cov の確認を追加
- 開発環境構築手順書に追記
- ベースラインのカバレッジを計測・記録

変更ファイル:
- `justfile` — `coverage` タスク追加、`check-tools` に cargo-llvm-cov 追加
- `docs/60_手順書/01_開発参画/01_開発環境構築.md` — ツール一覧に追記

### Phase 2: Domain エッジケーステスト追加

既存テストパターン（rstest fixtures、日本語テスト名）に従い、以下のテストを追加する。

変更ファイル:
- `backend/crates/domain/src/workflow.rs` — テストモジュールにテスト追加

#### WorkflowInstance `cancelled()` テスト（6 件）

```
mod workflow_instance {
  // 既存テストに追加:
  test_下書きからの取消でキャンセルになる        (Draft → Cancelled: Ok)
  test_申請済みからの取消でキャンセルになる      (Pending → Cancelled: Ok)
  test_処理中からの取消でキャンセルになる        (InProgress → Cancelled: Ok)
  test_承認済みからの取消はエラー              (Approved → Cancelled: Err)
  test_却下済みからの取消はエラー              (Rejected → Cancelled: Err)
  test_キャンセル済みからの取消はエラー          (Cancelled → Cancelled: Err)
}
```

実装パターン（既存テスト `test_承認完了でステータスが承認済みになる` を踏襲）:

```rust
#[rstest]
fn test_下書きからの取消でキャンセルになる(
    test_instance: WorkflowInstance,
    now: DateTime<Utc>,
) {
    let result = test_instance.cancelled(now);

    assert!(result.is_ok());
    let cancelled = result.unwrap();
    assert_eq!(cancelled.status(), WorkflowInstanceStatus::Cancelled);
    assert_eq!(cancelled.completed_at(), Some(now));
}

#[rstest]
fn test_承認済みからの取消はエラー(
    test_instance: WorkflowInstance,
    now: DateTime<Utc>,
) {
    let instance = test_instance
        .submitted(now).unwrap()
        .with_current_step("step_1".to_string(), now)
        .complete_with_approval(now).unwrap();

    let result = instance.cancelled(now);

    assert!(result.is_err());
}
```

#### WorkflowInstance `submitted()` 異常系テスト（2 件）

```
mod workflow_instance {
  test_申請済みからの再申請はエラー     (Pending → submitted: Err)
  test_処理中からの申請はエラー        (InProgress → submitted: Err)
}
```

#### WorkflowDefinition テスト（3 件）

既存の WorkflowDefinition テストの有無を確認し、不足分を追加。

```
mod workflow_definition {
  test_公開でステータスが公開済みになる       (Draft → Published: Ok)
  test_公開済みの再公開はエラー             (Published → Published: Err)
  test_アーカイブでステータスがアーカイブ済みになる (Published → Archived: Ok)
}
```

#### WorkflowStep `completed()` RequestChanges テスト（1 件）

```
mod workflow_step {
  test_差戻しで完了と差戻しになる  (Active → Completed(RequestChanges): Ok)
}
```

### Phase 3: API テスト ↔ OpenAPI 突合表

変更ファイル:
- `docs/40_詳細設計書/API_テスト突合表.md` — 新規作成

内容:

| # | エンドポイント | メソッド | Hurl テスト | 状態 |
|---|---------------|---------|------------|------|
| 1 | /api/v1/auth/login | POST | auth/login.hurl | カバー済み |
| 2 | /api/v1/auth/logout | POST | auth/logout.hurl | カバー済み |
| 3 | /api/v1/auth/me | GET | auth/me.hurl, auth/me_unauthorized.hurl | カバー済み |
| 4 | /api/v1/auth/csrf | GET | auth/csrf.hurl, auth/csrf_unauthorized.hurl | カバー済み |
| 5 | /api/v1/workflow-definitions | GET | — | ギャップ |
| 6 | /api/v1/workflow-definitions/{id} | GET | — | ギャップ |
| 7 | /api/v1/workflows | GET | — | ギャップ |
| 8 | /api/v1/workflows | POST | workflow/create_workflow.hurl | カバー済み |
| 9 | /api/v1/workflows/{dn} | GET | — | ギャップ |
| 10 | /api/v1/workflows/{dn}/submit | POST | workflow/submit_workflow.hurl | カバー済み |
| 11 | /api/v1/workflows/{dn}/steps/{sdn}/approve | POST | — | ギャップ |
| 12 | /api/v1/workflows/{dn}/steps/{sdn}/reject | POST | — | ギャップ |
| 13 | /api/v1/users | GET | — | ギャップ |
| 14 | /api/v1/tasks/my | GET | task/list_my_tasks.hurl | カバー済み |
| 15 | /api/v1/workflows/{wdn}/tasks/{sdn} | GET | task/get_task_by_display_numbers.hurl | カバー済み |
| 16 | /api/v1/dashboard/stats | GET | — | ギャップ |
| 17 | /health | GET | health.hurl | カバー済み |
| 18 | /health/ready | GET | — | ギャップ |

カバレッジ: 9/18 エンドポイント（50%）

### Phase 4: フォローアップ Issue 作成

- API テスト追加 Issue（Hurl テストの 9 件のギャップ解消）
- Usecase 層エラーパステスト Issue（reject の各エラーパス、submit エラーパス等）

## 検証方法

```bash
# Phase 1: カバレッジツール動作確認
just coverage

# Phase 2: テスト実行
cd backend && cargo test --package ringiflow-domain -- workflow

# 全体チェック
just check-all
```

## ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | 初版完成 → 探索結果の統合 | 3 エージェントの探索結果を突合、テストの穴を体系的に整理 | Domain cancelled() が完全にテストゼロと判明。Handler は薄いプロキシで API テストで十分と判断 |
| 2回目 | ツール選定 | cargo-llvm-cov vs cargo-tarpaulin を比較 | LLVM ベースの方が精度高く現代的。cargo-llvm-cov を採用 |
| 3回目 | スコープ確認 | 4つの完了基準を1 PR でカバーできるか検証 | Domain テスト追加は 12 件と小規模。突合表はドキュメントのみ。1 PR で収まる |
| 4回目 | 既存テストパターン確認 | workflow.rs のテストコードを精読 | rstest fixtures（now, test_instance, test_step）、日本語テスト名、mod 構造を確認。新テストはこのパターンに完全準拠 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の4つの完了基準すべてに対応するアクションがある | OK | 計測→Phase 1、特定→探索で完了、テスト追加→Phase 2、突合表→Phase 3 |
| 2 | 曖昧さ排除 | 追加するテスト名、ファイル、パターンが明確 | OK | 全12テストケースの名前・期待動作・実装パターンを記載 |
| 3 | 設計判断の完結性 | Handler 層の判断に根拠がある | OK | BFF=薄いプロキシ、ビジネスロジックは Usecase 層に集約。API テストで E2E カバー可能 |
| 4 | スコープ境界 | 対象と対象外が明記されている | OK | 対象: 計測+Domain テスト+突合表。対象外: Usecase テスト、API テスト追加、CI 統合 |
| 5 | 技術的前提 | cargo-llvm-cov の動作環境が確認済み | OK | llvm-tools-preview が rustup component として利用可能。Linux で動作確認済みの実績多数 |
| 6 | 既存ドキュメント整合 | 既存の ADR・設計書と矛盾がない | OK | テスト追加は品質追求の理念に合致。Handler 評価は Issue の To-Be（「統合テストで十分かを検証済み」）に直接対応 |
