# 計画ファイルのランダム名を lefthook pre-commit で検出・ブロックする

## Context

`prompts/plans/` 配下の計画ファイルが命名規則に従わないランダム名（例: `golden-booping-metcalfe.md`）のままコミットされる問題が繰り返し発生。`/wrap-up` Step 4 の手動チェックに依存しているため、lefthook pre-commit フックで構造的にブロックする。

→ 改善記録: `process/improvements/2026-02/2026-02-27_2118_計画ファイルのランダム名リネーム漏れが構造的に防止されていない.md`

## 対象

- `scripts/check/check-plan-filenames.sh`（新規作成）
- `lefthook.yaml`（pre-commit にフック追加）
- `justfile`（`lint-plan-filenames` レシピ追加）
- `scripts/check/parallel.sh`（Non-Rust レーンに追加）
- `prompts/plans/golden-booping-metcalfe.md` → `prompts/plans/944_procedure-docs-education-focus.md`（リネーム）

## 対象外

- 命名規則自体の変更（`prompts/plans/README.md` は変更しない）
- 既存の計画チェック（`plan-confirmations.sh`, `plan-test-layers.sh`）の変更

## Phase 1: スクリプト作成 + lefthook / just check 組み込み

### 確認事項

- パターン: `sanitize-improvements.sh` → `scripts/check/sanitize-improvements.sh`（引数あり/なしの分岐、エラー出力形式）
- パターン: lefthook の `glob` + `{staged_files}` の連携 → `lefthook.yaml` L33-36
- パターン: `parallel.sh` の Non-Rust レーン追加位置 → `scripts/check/parallel.sh` L39-42 付近

### 設計判断

検出ロジック: `basename` が数字で始まる or `README` で始まる → OK、それ以外 → NG

```bash
filename=$(basename "$file")
if [[ ! "$filename" =~ ^[0-9] ]] && [[ "$filename" != README* ]]; then
    # エラー
fi
```

理由: `prompts/plans/README.md` の命名規則に基づく。Issue 番号（`944_xxx.md`）も日付（`20260207_xxx.md`）も数字で始まる。

引数の分岐（`sanitize-improvements.sh` パターン踏襲）:
- 引数あり（lefthook pre-commit）: 渡されたファイルのみチェック
- 引数なし（`just lint-plan-filenames`）: `git ls-files --cached "prompts/plans/*.md"` で全ファイルチェック

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | ランダム名の計画ファイルをステージしてコミット → ブロック | 正常系（検出） | 手動検証 |
| 2 | 命名規則に従ったファイルをステージしてコミット → 通過 | 正常系（通過） | 手動検証 |
| 3 | `just check` 実行 → ランダム名ファイルがエラー | 正常系（CI検出） | 手動検証 |

### テストリスト

ユニットテスト（該当なし）: シェルスクリプトのため
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] ランダム名ファイルをステージ → `git commit` でブロックされること
- [ ] 正規名ファイルをステージ → `git commit` が通ること
- [ ] `just lint-plan-filenames` でランダム名ファイルが検出されること
- [ ] `just check` が通ること（`golden-booping-metcalfe.md` リネーム後）

### 実装内容

1. `scripts/check/check-plan-filenames.sh` を作成
   - `sanitize-improvements.sh` の引数分岐パターンを踏襲
   - 対象: `prompts/plans/*.md`（README.md は除外）
   - 検証: ファイル名が数字で始まるか `README` で始まるか

2. `lefthook.yaml` に追加（`sanitize-improvements` の下）
   ```yaml
   check-plan-filenames:
     glob: "prompts/plans/**/*.md"
     run: ./scripts/check/check-plan-filenames.sh {staged_files}
   ```

3. `justfile` に `lint-plan-filenames` レシピ追加（`lint-plan-test-layers` の下）
   ```just
   lint-plan-filenames:
       ./scripts/check/check-plan-filenames.sh
   ```

4. `scripts/check/parallel.sh` の Non-Rust レーンに `just lint-plan-filenames` 追加（`just lint-plan-test-layers` の下）

## Phase 2: 既存ファイルのリネーム

### 確認事項: なし（既知のパターンのみ）

### 操作パス: 該当なし（操作パスが存在しない）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `git mv` でリネーム → `just check` 通過

### 実装内容

`golden-booping-metcalfe.md` を `944_procedure-docs-education-focus.md` にリネーム（Issue #944 の計画ファイル、内容: 人間向け手順書の教育資料特化）。

## 検証

1. `just lint-plan-filenames` → リネーム前はエラー、リネーム後は成功
2. `just check` → 全体通過
3. lefthook 動作確認: ランダム名ファイルをステージして `git commit` → ブロック

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `just check` への組み込み経路が未定義 | 不完全なパス | justfile レシピ + parallel.sh の Non-Rust レーンに追加を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Issue 完了条件 4 項目すべてカバー |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 検出ロジック、ファイル名、配置先が確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 検出ロジック（数字 or README 先頭）を明記 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象/対象外セクションあり |
| 5 | 技術的前提 | 前提が考慮されている | OK | lefthook glob + staged_files の動作を既存パターンで確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | `prompts/plans/README.md` の命名規則と整合 |
