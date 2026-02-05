# derive_more/strum でボイラープレート削減

## 概要

Issue #233 に基づき、`derive_more` と `strum` クレートを活用して Rust コードのボイラープレートを削減した。約 100 行の手動実装を derive マクロに置き換え、`itertools::unique()` で重複排除も簡略化した。

## 背景と目的

Issue #231 で `derive_more`、`strum`、`itertools`、`maplit` を workspace に追加した。これらのクレートを活用して既存のボイラープレートコードを削減し、コードの保守性を向上させる。

## 実施内容

### 1. derive_more::Display で ID 型の Display 実装を自動生成

7 つの ID 型（Newtype パターン）の手動 `impl Display` を `#[derive(Display)]` + `#[display("{_0}")]` に置き換えた。

対象:
- `UserId`
- `TenantId`
- `RoleId`
- `Permission`
- `WorkflowDefinitionId`
- `WorkflowInstanceId`
- `WorkflowStepId`

### 2. strum で enum の文字列変換を自動生成

5 つの enum の `as_str()` メソッドと `impl Display` を `#[derive(IntoStaticStr, strum::Display)]` に置き換えた。

対象:
- `WorkflowDefinitionStatus`
- `WorkflowInstanceStatus`
- `WorkflowStepStatus`
- `StepDecision`
- `UserStatus`
- `DisplayIdEntityType`

### 3. itertools::unique() で重複排除を簡略化

2 箇所の `HashSet` + `for` ループによる重複排除を、イテレータチェーンに置き換えた。

対象:
- `workflow.rs`: `collect_user_ids_from_workflow()`
- `task.rs`: `list_my_tasks()` 内のユーザー ID 収集

### 4. rust.md にガイドラインを追加

`.claude/rules/rust.md` に derive_more、strum、itertools、maplit の使い方ガイドラインを追加した。

## 設計上の判断

### IntoStaticStr vs AsRefStr

| 選択肢 | 特徴 | 採用 |
|--------|------|------|
| `AsRefStr` | `&self` から借用。`as_ref()` メソッドを提供 | ❌ |
| `IntoStaticStr` | `&'static str` を返す。`Into` トレイトを実装 | ✅ |

`AsRefStr` を使うと、一時オブジェクトを sqlx マクロに渡す場合にライフタイムエラーが発生する。`IntoStaticStr` は `&'static str` を返すため、この問題を回避できる。

### FromStr の手動実装を維持

`strum::EnumString` を使うとパースエラーが `strum::ParseError` になる。プロジェクトでは `DomainError` を返したいため、`FromStr` は手動実装を維持した。

## 成果物

### コミット

- `8515f80` #233 Reduce boilerplate with derive_more and strum
- `291bb84` #233 Fix derive_more::Display example in rust.md

### 作成/更新ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.claude/rules/rust.md` | 推奨クレートのガイドライン追加 |
| `backend/crates/domain/Cargo.toml` | derive_more, strum 依存追加 |
| `backend/crates/domain/src/*.rs` | derive マクロ適用、手動実装削除 |
| `backend/crates/infra/src/repository/*.rs` | `as_str()` → `into()` に変更 |
| `backend/apps/core-service/Cargo.toml` | itertools 依存追加 |
| `backend/apps/core-service/src/**/*.rs` | itertools::unique() 適用 |

### PR

- #234: https://github.com/ka2kama/ringiflow/pull/234

## 議論の経緯

### 自己検証の実施タイミング

PR 作成後にユーザーから「自己検証終了後に PR 作成」との指摘を受けた。事後的に自己検証を実施したところ、rust.md の例に `#[display("{_0}")]` 属性が欠落していることを発見し、修正した。

## 学んだこと

1. **IntoStaticStr のライフタイム特性**: `AsRefStr` は `&self` から借用するため、一時オブジェクトでは使えない。`IntoStaticStr` は `&'static str` を返すため安全。

2. **derive_more::Display の Newtype パターン**: 単に `#[derive(Display)]` だけでは不十分で、`#[display("{_0}")]` 属性で内部フィールドの表示形式を指定する必要がある。

3. **strum と Into トレイト**: `IntoStaticStr` は `Into<&'static str>` を実装するため、型推論が効かない場合は `let s: &str = value.into();` のように明示的な型アノテーションが必要。

## 発見した問題

PR 作成前に自己検証を実施しなかった。これは [自己検証ループの自動実行欠如](../improvements/2026-02/2026-02-05_2100_自己検証ループの自動実行欠如.md) と同じ構造の問題である。

事後検証で問題を発見できたが、本来は PR 作成前に実施すべきだった。

## 次のステップ

- #234 のレビュー・マージ
