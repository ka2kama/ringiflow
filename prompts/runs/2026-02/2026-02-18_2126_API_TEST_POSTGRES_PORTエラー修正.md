# API_TEST_POSTGRES_PORT 未設定エラーの修正

Issue: #632
PR: #635
ブランチ: `fix/632-api-test-env-port`

## 概要

`just check-all` を `.env` なし、または PR #624 以前の古い `.env` がある状態で実行すると、`api-test-reset-db` レシピで `API_TEST_POSTGRES_PORT` が未定義となりエラーになる問題を修正した。

## 実施内容

### 根本原因の特定

justfile の `set dotenv-load := true` は just プロセス起動時に1回だけ `.env` を読み込む。レシピの依存関係で `setup-env` を追加しても、non-shebang レシピには起動後に生成・更新された `.env` の変数が反映されない。

2つの障害シナリオを特定:
1. `.env` が存在しない（初回 or 削除後）
2. `.env` が #624 以前に生成され `API_TEST_*` 変数を含まない

### 修正

3つの変更を組み合わせた:

1. `scripts/setup-env.sh`: `API_TEST_POSTGRES_PORT` を sentinel キーとして陳腐化を検知。欠落時は既存のフォールスルーロジックで再生成
2. `justfile` `api-test-deps`: `setup-env` 依存を追加し、テスト実行前に `.env` を保証
3. `justfile` `api-test-reset-db`: shebang レシピに変換し、`set -a; source .env; set +a` で dotenv-load のタイミング制約を回避

## 判断ログ

- `api-test-reset-db` を shebang 化する判断: dotenv-load は just 起動時に1回のみ読み込むため、`setup-env` 依存の追加だけでは non-shebang レシピに反映されない。`source .env` で実行時に変数を取得する方式を採用
- `set -a; source .env; set +a` パターン: `scripts/run-api-tests.sh` L28-31 の既存パターンに合わせた

## 成果物

コミット:
- `4efe30e` #632 Fix API_TEST_POSTGRES_PORT unset error in check-all

変更ファイル:
- `justfile`: `api-test-deps` に `setup-env` 依存追加、`api-test-reset-db` を shebang 化
- `scripts/setup-env.sh`: sentinel キーによる陳腐化検知を追加
- `prompts/plans/lovely-tumbling-acorn.md`: 計画ファイル
