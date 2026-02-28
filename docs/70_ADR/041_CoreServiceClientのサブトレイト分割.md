# ADR-041: CoreServiceClient のサブトレイト分割

## ステータス

承認済み

## コンテキスト

`backend/apps/bff/src/client/core_service.rs` は 1212 行に達しており、プロジェクトのファイルサイズ閾値 500 行を大幅に超過していた。

行数超過の根本原因は、`CoreServiceClient` トレイトが 20 メソッドを持つ巨大インターフェースであること（ISP: Interface Segregation Principle 違反）。Rust では `impl Trait for Type` ブロックを分割できないため、614 行が 1 ブロックに集中していた。

この ISP 違反により 2 つの問題が発生していた:

| 問題 | 詳細 |
|------|------|
| ファイル分割の制約 | impl ブロックが分割不可のため、物理的にファイルを分割できない |
| テストスタブの肥大化 | auth テストで `CoreServiceUserClient` の 3 メソッドのみ必要だが、残り 17 メソッドの `unimplemented!()` が必須（約 300 行のボイラープレート） |

### 500 行閾値の再評価

Issue #290 の残り 5 ファイルを分析した結果、行数超過の原因はファイルごとに異なり、一律分割は不適切と判断した:

| ファイル | 行数 | 超過の原因 | 対応 |
|---------|------|-----------|------|
| `core_service.rs` | 1212 | ISP 違反 | この ADR でサブトレイト分割 |
| `auth.rs` | 1135 | ISP の症状（テストスタブ肥大化） | core_service 分割で 150 行削減 |
| `task.rs` | 1042 | 実装 225 行 + テスト 817 行（実装は高凝集） | 例外として許容 |
| `New.elm` | 1115 | TEA パターンの典型的ページ | 例外として許容（要検討） |
| `Main.elm` | 832 | SPA エントリポイント | 例外として許容 |

この分析を踏まえ、structural-review.md の閾値基準を「行数超過 → 責務分析 → 判断」に改善した。

## 検討した選択肢

### 選択肢 1: サブトレイト + スーパートレイト + ブランケット impl

`CoreServiceClient` を User/Workflow/Task の 3 つのサブトレイトに分割し、スーパートレイト + ブランケット impl で束ねる。

```rust
pub trait CoreServiceClient:
    CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient
{}

impl<T> CoreServiceClient for T
where
    T: CoreServiceUserClient + CoreServiceWorkflowClient + CoreServiceTaskClient,
{}
```

評価:

- 利点: `dyn CoreServiceClient` が引き続き使用可能、消費者は必要なサブトレイトのみに依存可能、テストスタブが最小限
- 欠点: トレイト数が増える（1 → 4）

### 選択肢 2: 単純なファイル分割（トレイトは維持）

20 メソッドの `CoreServiceClient` トレイトを維持しつつ、DTO 型やエラー型を別ファイルに移動する。

評価:

- 利点: API の変更が最小限
- 欠点: ISP 違反が解消されない、テストスタブの肥大化が続く、impl ブロックが分割不可のため効果が限定的

### 選択肢 3: メソッドごとの個別トレイト（Fine-grained ISP）

20 メソッドそれぞれを個別トレイトにする。

評価:

- 利点: 最も厳密な ISP 適用
- 欠点: トレイト数が爆発（20 個）、使用側の型境界が煩雑、過度な分割

### 比較表

| 観点 | 選択肢 1 | 選択肢 2 | 選択肢 3 |
|------|---------|---------|---------|
| ISP 解消 | 責務単位で解消 | 未解消 | 完全解消だが過剰 |
| テストスタブ削減 | 大（300 行→0 行） | なし | 大 |
| 後方互換性 | `dyn CoreServiceClient` 維持 | 完全互換 | 大幅な API 変更 |
| 複雑さ | 適度（4 トレイト） | 低 | 高（20 トレイト） |

## 決定

選択肢 1: サブトレイト + スーパートレイト + ブランケット impl パターンを採用する。

理由:

1. ISP 違反を責務単位（User/Workflow/Task）で解消し、ファイル分割と型の絞り込みを同時に実現
2. ブランケット impl により `dyn CoreServiceClient` の後方互換性を維持
3. テストスタブのボイラープレートを 303 行削減（auth.rs -150 行、統合テスト -153 行）

### サブトレイトの分類

| サブトレイト | メソッド数 | 責務 |
|------------|----------|------|
| `CoreServiceUserClient` | 3 | ユーザー関連（list_users, get_user_by_email, get_user） |
| `CoreServiceWorkflowClient` | 12 | ワークフロー関連（CRUD + 承認/却下 + display_number 版） |
| `CoreServiceTaskClient` | 4 | タスク・ダッシュボード関連 |

### 消費者への ISP 適用

`AuthState.core_service_client` の型を `Arc<dyn CoreServiceClient>` から `Arc<dyn CoreServiceUserClient>` に変更。認証ハンドラが User 系 3 メソッドのみ使用する実態を型で表現した。

`main.rs` では具象型 `Arc<CoreServiceClientImpl>` を保持し、各 State のフィールド型に合わせて unsizing coercion で注入する。Rust では `Arc<dyn TraitA>` → `Arc<dyn TraitB>` の変換が不可能なため、具象型を起点にする必要がある。

## 帰結

### 肯定的な影響

- `core_service.rs` が 1212 行 → 親モジュール約 30 行に縮小（各サブモジュールは 50-537 行）
- テストスタブの 303 行のボイラープレートを削除
- 認証ハンドラが依存する型が `CoreServiceUserClient`（3 メソッド）に限定され、変更影響範囲が明確に

### 否定的な影響・トレードオフ

- トレイト数が 1 → 4 に増加するが、各トレイトの責務は明確
- `CoreServiceClientImpl` のフィールドを `pub(super)` に変更する必要がある（サブモジュールからのアクセス用）

### 関連ドキュメント

- Issue: #290（Refactor oversized files）
- 先行 ADR: ADR-039（ワークフローモジュールの分割方針）
- 構造レビュー: `.claude/rules/structural-review.md`（閾値基準の改善）

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-10 | 初版作成 |
