# #632 `just check-all` の API_TEST_POSTGRES_PORT 未設定エラーの修正

## Context

PR #624 で API/E2E テストのワーカーツリー対応（動的ポート割り当て）を導入した際、`api-test-reset-db` レシピのポートをハードコード（`15433`）から環境変数（`${API_TEST_POSTGRES_PORT}`）に変更した。この変数は `.env` から供給されるが、`.env` が存在しない or 古い場合にエラーになる。

CI では `just setup-env` を明示的に実行しているため問題が発生しない。ローカルでは `.env` がない状態で `just check-all` を実行するとエラーになる。

## 根本原因

justfile の `set dotenv-load := true` は **just 起動時に1回だけ** `.env` を読み込む。以下の2シナリオで変数が未定義になる:

1. `.env` が存在しない（初回 or 削除後）
2. `.env` が #624 以前に生成され `API_TEST_*` 変数を含まない

レシピ依存で `setup-env` を追加しても、dotenv-load は起動時に完了済みのため non-shebang レシピには反映されない。

## 修正方針

3つの変更を組み合わせる:

| 変更 | 目的 | 対象シナリオ |
|------|------|------------|
| `setup-env.sh` に陳腐化検知を追加 | 古い `.env` を検知・再生成 | シナリオ2 |
| `api-test-deps` に `setup-env` 依存を追加 | テスト実行前に `.env` を保証 | シナリオ1 |
| `api-test-reset-db` を shebang 化 + `source .env` | dotenv-load に依存せず変数を取得 | 両方 |

## 対象 / 対象外

対象: `scripts/setup-env.sh`、`justfile`（`api-test-deps`、`api-test-reset-db`）
対象外: `api-test-deps` の echo 修正（`setup-env` 依存で `.env` が保証される）、`dev-deps` 等の他レシピ

## Phase 1: `setup-env.sh` 陳腐化検知

ファイル: `scripts/setup-env.sh`

L22-28 の "全ファイル存在 → スキップ" チェックに、`API_TEST_POSTGRES_PORT` の存在確認を追加する。このキーは #624 で追加されたため、sentinel として機能する。

```bash
# 全ファイルが揃っており、必須変数も存在する場合のみスキップ
if [[ -f .env && -f backend/.env && -f backend/.env.api-test ]]; then
    if grep -q '^API_TEST_POSTGRES_PORT=' .env 2>/dev/null; then
        echo "  確認: .env"
        echo "  確認: backend/.env"
        echo "  確認: backend/.env.api-test"
        echo "✓ 環境変数ファイル準備完了"
        exit 0
    fi
    echo "  ⚠ .env が古い形式です。再生成します..."
fi
```

欠落時は既存のフォールスルーロジック（L30-41: オフセット再利用 → L44-84: worktree → L85-92: main テンプレートコピー）が再生成を行う。

### 確認事項

- [ ] パターン: `setup-env.sh` の分岐ロジック → L30-92 を Read 済み
- [ ] 動作: sentinel 欠落時のフォールスルーで正しいパスが実行されること

### テストリスト

ユニットテスト（該当なし: シェルスクリプト）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 2: justfile レシピ修正

ファイル: `justfile`

### 2a: `api-test-deps` に `setup-env` 依存を追加

```just
api-test-deps: setup-env
    #!/usr/bin/env bash
    # （以降は変更なし）
```

`setup-env` は冪等（ファイル + 変数が揃っていればスキップ）。CI で2回実行されても問題ない。

### 2b: `api-test-reset-db` を shebang レシピに変換

```just
api-test-reset-db:
    #!/usr/bin/env bash
    set -euo pipefail
    # dotenv-load は just 起動時に1回のみ読み込むため、
    # .env が起動後に生成された場合にも対応するため直接 source する
    set -a
    source .env
    set +a
    echo "API テスト用データベースをリセット中..."
    cd backend && DATABASE_URL="postgres://ringiflow:ringiflow@localhost:${API_TEST_POSTGRES_PORT}/ringiflow" sqlx database reset -y
    echo "✓ API テスト用データベースリセット完了"
```

設計判断:
- `set -euo pipefail`: 元の non-shebang レシピが `sh -euo pipefail` で実行されていたのを踏襲
- `set -a` / `set +a`: `run-api-tests.sh` L28-31 の既存パターンに合わせる
- `source .env`: dotenv-load のタイミング制約を回避し、レシピ実行時に変数を取得

### 確認事項

- [ ] パターン: shebang レシピの依存宣言 → `setup: check-tools setup-env ...` (L23)
- [ ] パターン: `set -a; source .env; set +a` → `run-api-tests.sh` L28-31
- [ ] 動作: shebang レシピ内の `cd backend` が正しく動作すること

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## 検証

1. `.env` を削除 → `just setup-env` → `.env` が生成され `API_TEST_POSTGRES_PORT` を含む
2. 古い `.env`（`API_TEST_*` なし）→ `just setup-env` → 再生成される
3. `.env` を削除 → `just check-all` → `setup-env` 自動実行 → 全テスト通過
4. 正しい `.env` → `just check-all` → 既存動作に影響なし

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | dotenv-load のタイミング制約: `setup-env` を依存に追加するだけでは non-shebang レシピに反映されない | 不完全なパス | `api-test-reset-db` を shebang 化し `source .env` で直接変数を取得する方針に変更 |
| 2回目 | `api-test-deps` の echo 文も同じ問題を持つ | 競合・エッジケース | `setup-env` 依存追加で `.env` が事前保証されるため修正不要と判断 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | シナリオ1（`.env` なし）は `setup-env` 依存、シナリオ2（古い `.env`）は陳腐化検知で対応 |
| 2 | 曖昧さ排除 | OK | 変更箇所とコードスニペットを具体的に記載 |
| 3 | 設計判断の完結性 | OK | shebang 化の理由、`set -euo pipefail` の必要性、sentinel キーの選択を記載 |
| 4 | スコープ境界 | OK | 対象3箇所と対象外（`api-test-deps` echo、`dev-deps` 等）を明記 |
| 5 | 技術的前提 | OK | dotenv-load のタイミング制約を確認 |
| 6 | 既存ドキュメント整合 | OK | `docker-compose.api-test.yaml` のコメントと整合 |
