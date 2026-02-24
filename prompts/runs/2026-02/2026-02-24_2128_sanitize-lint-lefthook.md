# 改善記録のサニタイズ lint 自動検証

Issue: #845

## 概要

改善記録におけるユーザー発言の直接引用（サニタイズ漏れ）を lefthook pre-commit で構造的に防止する lint スクリプトを実装した。3 回の再発（02-09, 02-15, 02-19）により行動規範形式の対策が不十分であることが実証されたため、フロー組み込み形式に転換した。

## 実施内容

### Phase 1: lint スクリプト実装

`scripts/check/sanitize-improvements.sh` を新規作成した。

- 3 つの ERE パターンで検出:
  1. ユーザー帰属 + カギ括弧引用（`ユーザー[がのはから].*[「『][^」』]+[」』]`）
  2. カギ括弧引用 + 発話動詞（`[「『]...[」』]と指摘し` 等）
  3. カギ括弧引用 + 帰属名詞（`[「『]...[」』]という指摘` 等）
- `awk` でコードブロック内を除外
- 引数あり（pre-commit の `{staged_files}`）と引数なし（全ファイル）の 2 モード
- `stale-annotations.sh` と同じ構造（set -euo pipefail, errors 配列, exit code）

### Phase 2: 既存違反の修正

27 件の違反を 23 ファイルで修正した。直接引用を技術的な要約に言い換え、修正後に lint を実行して exit 0 を確認。

1 件の偽陽性（テスト名 `test_削除されたユーザーの...` + `「正しく」`）は `_正しく_`（イタリック）に変更して解消。

### Phase 3: lefthook + justfile + parallel.sh 統合

- `lefthook.yaml`: pre-commit に `sanitize-improvements` コマンド追加（`glob` + `{staged_files}`）
- `justfile`: `lint-improvements-sanitize` レシピ追加
- `scripts/check/parallel.sh`: Non-Rust レーンに追加

### Phase 4: ドキュメント更新

`process/improvements/README.md` にサニタイズルールセクションを追加。禁止表現と修正例のテーブル、自動検証の説明を記載。

## 判断ログ

- 設計判断: Shell vs Rust → Shell を採用。パターンが grep ベースで単純、pre-commit での高速起動が重要、`stale-annotations.sh` と同じパターン。フォーマット検証（Rust）とサニタイズ検証（Shell）は別の関心事
- 偽陽性対策: recall（真陽性の見逃し防止）を優先。エッジケースは修正時に言い換えて解消する方針
- `git ls-files` の日本語対応: `-c core.quotepath=false` が必要（UTF-8 ファイル名のエスケープ回避）

## 成果物

コミット:
- `7e9a6a0` #845 Add sanitization lint for improvement records

作成ファイル:
- `scripts/check/sanitize-improvements.sh`（新規）
- `prompts/plans/845_sanitize-lint-lefthook.md`（計画ファイル）

更新ファイル:
- `lefthook.yaml`
- `justfile`
- `scripts/check/parallel.sh`
- `process/improvements/README.md`
- 改善記録 23 ファイル（サニタイズ違反修正）

検証:
- `just check-all`: 全テスト通過
- lefthook pre-commit: `sanitize-improvements` hook 動作確認済み
- lefthook pre-push: `check-pre-push` 全通過
