# #696 Observability の構造的担保（CI チェック + ルールファイル）

## Context

Epic #648 で Observability 基盤を実装し `#[tracing::instrument]` が 154 箇所に適用済みだが、新規コードがこのパターンに従うことを構造的に担保する仕組みがない。AI エージェントへのルールファイル、CI チェックスクリプト、CI ワークフロー統合の 3 層で担保する。

## 対象

- `.claude/rules/observability.md` — 新規作成
- `scripts/check/instrumentation.sh` — 新規作成
- `justfile` — `check-instrumentation` レシピ追加
- `scripts/check/parallel.sh` — Non-Rust レーンに追加
- `.github/workflows/ci.yaml` — code-quality ジョブに step 追加

## 対象外

- 既存の計装パターンの変更
- BFF HTTP クライアントの CI チェック（ルールファイルの `paths:` でカバー、CI スクリプトには含めない）
- ユースケース層（設計上、計装対象外）

---

## Phase 1: `.claude/rules/observability.md` の作成

### 確認事項
- [x] 既存ルールファイルの frontmatter → `api.md`: BFF handler + openapi、`repository.md`: `**/repository/**/*.rs` + `**/infra/**/*.rs`
- [x] `rule-skill-writing.md` の文体規約 → 指示的・簡潔、背景/理由の散文を含めない

### 成果物

frontmatter に `paths:` を指定し、ハンドラ・リポジトリ・クライアント・セッションの `.rs` ファイルを対象とする。

```yaml
---
paths:
  - "backend/apps/*/src/handler/**/*.rs"
  - "backend/crates/infra/src/repository/**/*.rs"
  - "backend/apps/bff/src/client/**/*.rs"
  - "backend/crates/infra/src/session.rs"
---
```

内容:

1. 計装パターンテーブル（レイヤー別の level / skip_all / fields）
2. PII ルール（skip_all デフォルト、安全なフィールドのみ fields() で明示）
3. 除外対象（health_check、ユースケース層）
4. 属性の配置順（`#[utoipa::path]` → `#[tracing::instrument]` → `pub async fn`）
5. Observability 設計書への参照リンク

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [x] `just lint-rules` が PASS（`rule-files.sh` が `paths:` を検出）

---

## Phase 2: `scripts/check/instrumentation.sh` の作成

### 確認事項
- [x] 既存スクリプトパターン → `impl-docs.sh`: ERRORS 配列、`set -euo pipefail`、exit 1 でブロック
- [x] ハンドラの `pub async fn` パターン → 3 サービスの handler ディレクトリ、health_check が 3 箇所（BFF/Core/Auth）
- [x] リポジトリの trait 定義 vs impl メソッドの判別 → `;` で終了 = trait 署名、`{` で終了 = impl メソッド

### 設計

**チェック対象:**

| 対象 | ファイルパターン | 検出パターン | 除外 |
|------|--------------|------------|------|
| ハンドラ | `backend/apps/*/src/handler/**/*.rs` | `pub async fn` | `health_check`、テストファイル |
| リポジトリ impl | `backend/crates/infra/src/repository/**/*.rs` | `async fn`（impl メソッドのみ） | trait 署名 |

**検出アルゴリズム:**

1. ハンドラ: `pub async fn` 行の上方 10 行以内に `#[tracing::instrument` があるか
2. リポジトリ: `async fn` 行から前方スキャンし、`;` より先に `{` が見つかれば impl メソッド → 上方 10 行チェック

**スクリプト構造:**

```bash
#!/usr/bin/env bash
set -euo pipefail

ERRORS=()
EXCLUDE_FUNCTIONS=("health_check")

check_handlers() { ... }
check_repository_impls() { ... }

check_handlers
check_repository_impls

# エラー集計と exit code
```

exit 1 でブロック（計装漏れは許容しない）。

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [x] 既存コードで `./scripts/check/instrumentation.sh` が exit 0
- [x] `health_check` が除外されること（3 サービス x 1 = 3 箇所）
- [x] trait 署名の `async fn` が除外されること
- [x] `just lint-shell` が PASS（ShellCheck）

---

## Phase 3: justfile + parallel.sh + CI 統合

### 確認事項
- [x] justfile 構造品質チェックセクション → L407-411、check-impl-docs の後に追加
- [x] parallel.sh Non-Rust レーン → L41 の check-impl-docs の後に追加
- [x] ci.yaml code-quality ジョブ → L735-736、Check code duplicates の後に追加
- [x] 新しい Action 追加なし → `just` コマンドのみ、Action 許可設定の更新は不要

### 成果物

**justfile** — 構造品質チェックセクション（`check-impl-docs` の後）に追加:

```just
# 計装（tracing::instrument）の漏れを検出
check-instrumentation:
    ./scripts/check/instrumentation.sh
```

**parallel.sh** — Non-Rust レーン内の `just check-impl-docs` の後に追加:

```bash
just check-instrumentation
```

**ci.yaml** — code-quality ジョブの `Check code duplicates` の後に step 追加:

```yaml
      - name: Check instrumentation
        run: just check-instrumentation
```

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [x] `just check-instrumentation` が正常動作
- [x] `just lint-ci` が PASS（actionlint）
- [x] `just check-all` が PASS

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `backend/apps/core-service/src/handler/auth/tests.rs` がハンドラディレクトリに存在 | 不完全なパス | テストファイルをファイル名ベースで除外（ただし `pub async fn` は含まないため実害なし。安全策として除外） |
| 2回目 | BFF HTTP クライアントの CI チェック対象可否 | スコープ境界 | Issue のスコープがハンドラ+リポジトリ。クライアントはルールファイルの `paths:` で AI ガード、CI チェックは対象外 |
| 3回目 | ci.yaml の code-quality ジョブに step を追加する際、新しい Action は不要か | 技術的前提 | `just` コマンドのみで Action 追加なし。`scripts.md` の Action 許可設定の更新は不要 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の 3 タスクが全て含まれている | OK | Phase 1 = タスク 1、Phase 2 = タスク 2、Phase 3 = タスク 3 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | ファイルパス、検出パターン、除外リスト、配置場所がすべて確定 |
| 3 | 設計判断の完結性 | 全バリエーションに判断あり | OK | trait/impl 判別方式、テストファイル除外、BFF クライアントのスコープ外判断を記録 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象（ハンドラ + リポジトリ impl）、対象外（BFF クライアント CI チェック、ユースケース層）を明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | parallel.sh の `set -e` 動作、rule-files.sh の frontmatter 検出を確認済み |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Observability 設計書の計装パターン・除外対象と一致。スクリプト配置は `scripts.md` 準拠 |

## 検証計画

1. Phase 1 完了後: `just lint-rules` で rule-files.sh の frontmatter 検出を確認
2. Phase 2 完了後: `./scripts/check/instrumentation.sh` を実行し exit 0 を確認、`just lint-shell` で ShellCheck 通過
3. Phase 3 完了後: `just check-all` で全体通過を確認
