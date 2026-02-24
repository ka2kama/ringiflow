# #843 知識-実行乖離の構造的対策: lint 自動検証の横展開

## Context

知識-実行乖離が 3 期連続でカテゴリ最多（31%→40%→45%）に加速している。#735 で「計画ファイルの確認事項チェック漏れ」の lint（`scripts/check/plan-confirmations.sh`）が成功したため、同様のアプローチを他の知識-実行乖離パターンに横展開する。

Issue の候補 3 つのうち、自動検証可能な 2 つを実装する:
1. OpenAPI ハンドラ登録照合 — 高い検出精度、偽陽性リスク低
2. テスト層網羅確認 — `plan-confirmations.sh` と同じ枠組みで実装可能

## スコープ

### 対象

- OpenAPI ハンドラ登録照合の lint スクリプト
- テスト層網羅確認の lint スクリプト
- justfile・parallel.sh への統合
- `health::health_check` の openapi.rs 未登録バグ修正（lint が検出する既存バグ）

### 対象外

- チェックボックス検証（GitHub API が必要で lefthook 不可。技術的制約で後回し）
- 改善記録サニタイズ（#845 で対応）

## 実装計画

### Phase 1: OpenAPI ハンドラ登録照合

目的: `#[utoipa::path]` が付いた関数が `openapi.rs` の `paths()` に登録されているか検証する lint スクリプトを追加する。

#### 確認事項

- [x] パターン: `plan-confirmations.sh` のシェルスクリプト構造 → `scripts/check/plan-confirmations.sh`
- [x] パターン: justfile の lint ターゲット命名 → `lint-plans`, `lint-rules` 等
- [x] パターン: `parallel.sh` の Non-Rust レーンへの追加 → `scripts/check/parallel.sh` L30-48
- [x] 型: `openapi.rs` の `paths()` セクション → `backend/apps/bff/src/openapi.rs` L31-78
- [x] 型: ハンドラのディレクトリ構造 → `backend/apps/bff/src/handler/`（`auth/`, `workflow/` はサブディレクトリ、他は単一ファイル）

#### 変更ファイル

1. `scripts/check/openapi-handler-registration.sh` — 新規作成
2. `justfile` — `lint-openapi-handlers` ターゲット追加
3. `scripts/check/parallel.sh` — Non-Rust レーンに追加
4. `backend/apps/bff/src/openapi.rs` — `health::health_check` を `paths()` に追加

#### 検出ロジック

1. `openapi.rs` の `paths()` セクションから登録済みエントリを抽出
2. `backend/apps/bff/src/handler/` 配下で `#[utoipa::path]` の後の `pub async fn <name>` を抽出
3. ファイルパスからモジュール名を特定（`user.rs` → `user`、`auth/login.rs` → `auth`）
4. `<module>::<function>` が `paths()` に存在するか照合
5. 不一致をエラー報告

エッジケース:
- `mod.rs` と `workflow.rs`（モジュール定義ファイル）: `#[utoipa::path]` がないため自動的にスキップ
- `#[tracing::instrument]` が間に挟まる場合: `#[utoipa::path]` を検出後、`pub async fn` 行まで待つ
- 意図的に除外したい関数: `#[utoipa::path]` を削除する方針（lint ルール: アノテーションがあるなら登録必須）

#### 操作パス

操作パス: 該当なし（lint ツール）

#### テストリスト

ユニットテスト（該当なし — Bash スクリプト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動テスト:
- `./scripts/check/openapi-handler-registration.sh` → 成功すること（`health_check` 修正後）
- `shellcheck scripts/check/openapi-handler-registration.sh` → エラーなし
- `just lint-openapi-handlers` → 同じ結果

---

### Phase 2: テスト層網羅確認

目的: 計画ファイルの `#### テストリスト` セクションで 4 つのテスト層（ユニットテスト / ハンドラテスト / API テスト / E2E テスト）が全て明記されているか検証する lint スクリプトを追加する。

#### 確認事項

- [x] パターン: `plan-confirmations.sh` の diff 対象ファイル検出ロジック → `scripts/check/plan-confirmations.sh` L12-35
- [x] ルール: zoom-rhythm.md のテストリスト仕様 → `.claude/rules/zoom-rhythm.md`（4層を明記、該当しない層は「該当なし」）
- [x] パターン: テスト層の記載バリエーション → `prompts/plans/860_dialog-state-approach-review.md`（正式形式）、`prompts/plans/855_type-safe-state-machine-rename.md`（省略形式）

#### 変更ファイル

1. `scripts/check/plan-test-layers.sh` — 新規作成
2. `justfile` — `lint-plan-test-layers` ターゲット追加
3. `scripts/check/parallel.sh` — Non-Rust レーンに追加

#### 検出ロジック

1. main との差分で変更された `prompts/plans/*.md` を検出
2. 各ファイルで `#### テストリスト` セクションを検索
3. セクション内に 4 層（ユニットテスト / ハンドラテスト / API テスト / E2E テスト）が全て存在するか確認
4. 存在しない層がある場合エラー

免除条件:
- `#### テストリスト` ヘッダー行に「該当なし」を含む場合（Phase 全体がテスト不要）
- main ブランチでの実行はスキップ（`plan-confirmations.sh` と同じ）

#### 操作パス

操作パス: 該当なし（lint ツール）

#### テストリスト

ユニットテスト（該当なし — Bash スクリプト）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動テスト:
- テスト層が欠けた計画ファイルを用意 → 検出されること
- `テストリスト: 該当なし` 形式 → 免除されること
- 全層を記載した正しい計画 → 成功すること
- `shellcheck scripts/check/plan-test-layers.sh` → エラーなし
- `just lint-plan-test-layers` → 同じ結果

---

## 統合方法

### justfile

```just
# OpenAPI ハンドラ登録照合
lint-openapi-handlers:
    ./scripts/check/openapi-handler-registration.sh

# 計画ファイルのテスト層網羅確認
lint-plan-test-layers:
    ./scripts/check/plan-test-layers.sh
```

### parallel.sh

Non-Rust レーンに追加（配置: 同カテゴリの lint の近くに）:

```
just lint-plans
just lint-plan-test-layers    # ← 追加（lint-plans の直後）
just lint-rules
just lint-openapi-handlers    # ← 追加（lint-rules の直後）
```

## 検証方法

1. 各スクリプトの単体実行で正常動作を確認
2. `shellcheck` で両スクリプトのエラーなしを確認
3. `just check-all` で全体パスを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `auth/` と `workflow/` がサブディレクトリ構造 | 未定義 | Phase 1 のスクリプトにサブディレクトリ対応ロジック追加 |
| 2回目 | `#[utoipa::path]` と `pub async fn` の間に `#[tracing::instrument]` が挟まる | エッジケース | フラグで `pub async fn` 行まで待つ方式を採用 |
| 3回目 | `health_check` が実際に未登録（既存バグ） | 競合・エッジケース | Phase 1 に修正を含める |
| 4回目 | テスト層の記載バリエーションが複数（正式形式、省略形式） | 曖昧 | Phase 2 の検出ロジックに免除条件を網羅 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の候補のうち実装可能なものが全て含まれている | OK | OpenAPI 登録照合 + テスト層網羅の 2 Phase。チェックボックス検証は技術制約で除外、サニタイズは #845 で対応 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 検出ロジック・エッジケース・免除条件が具体化済み |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | ディレクトリ構造差異、テスト層バリエーション、`health_check` 対応方針が記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象: 2 lint スクリプト + 統合。対象外: チェックボックス検証、#845 |
| 5 | 技術的前提 | 前提が考慮されている | OK | ShellCheck パス、Bash 正規表現、`git diff --name-only` の挙動 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | zoom-rhythm.md テスト層仕様、plan-confirmations.sh パターンと整合 |
