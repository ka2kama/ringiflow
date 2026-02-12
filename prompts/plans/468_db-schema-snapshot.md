# 計画: #468 DBスキーマスナップショットの自動生成

## Context

マイグレーションファイル（27個の差分の積み重ね）から「現在のDBスキーマ全体像」を把握するには、DB接続して確認するしかない。`pg_dump --schema-only` でスナップショットを Git 管理することで、DB接続なしでスキーマ全体を把握でき、PRでのスキーマ差分レビューも容易になる。

Rails の `db/structure.sql` に相当する仕組み。既存の `openapi-check`（utoipa → openapi.yaml 同期チェック）と同じパターンで実装する。

## 対象

- `justfile`: 新コマンド追加 + 既存コマンド修正
- `scripts/check-parallel.sh`: Rust レーンにチェック追加
- `.github/workflows/ci.yaml`: `rust-integration` ジョブにチェック追加
- `backend/schema.sql`: 新規生成（Git 管理）
- `CLAUDE.md`: データストア操作セクションに新コマンドを追記

## 対象外

- `backend/.env` や接続情報の変更
- マイグレーションファイル自体の変更
- フロントエンド・Elm の変更

## 設計判断

### pg_dump のフラグ選定

```bash
pg_dump --schema-only --no-owner --no-privileges --no-tablespaces \
    --exclude-table=_sqlx_migrations \
    "${URL}"
```

| フラグ | 理由 |
|--------|------|
| `--schema-only` | データは不要、スキーマ定義のみ |
| `--no-owner` | 環境依存のオーナー情報を除外 |
| `--no-privileges` | 環境依存の GRANT/REVOKE を除外 |
| `--no-tablespaces` | 環境依存のテーブルスペース割当を除外 |
| `--exclude-table=_sqlx_migrations` | sqlx 内部テーブル（マイグレーション追跡用）を除外 |

### バージョン行のストリップ

`pg_dump` は出力冒頭にバージョン行を含む:
```
-- Dumped from database version 17.x
-- Dumped by pg_dump version 17.x
```

開発者のローカル環境と CI で `pg_dump` クライアントバージョンが異なる可能性があるため、これらの行を `sed` で除去し、環境間の差分を防ぐ。

### justfile の接続先

| コマンド | 接続先 | 理由 |
|---------|--------|------|
| `db-dump-schema` | `_psql_url`（justfile 変数） | ローカル開発専用。既存 DB コマンド（`db-tables` 等）と統一 |
| `schema-check` | `_psql_url`（justfile 変数） | ローカル開発専用 |
| CI のチェック | `$DATABASE_URL`（環境変数） | CI 環境のDB名（`ringiflow_test`）に対応 |

スキーマ定義自体は DB名に依存しないため、`ringiflow_dev` と `ringiflow_test` で同一の出力になる。

### CI のチェック配置

`rust-integration` ジョブに配置する。理由:
- PostgreSQL サービスコンテナが起動済み
- マイグレーション適用後にチェック可能
- `pg_dump` は Ubuntu ランナーで利用可能（`postgresql-client` パッケージ）

## Phase 1: justfile コマンド追加 + check-parallel.sh + CI 更新

### 確認事項

- [x] パターン: `openapi-check` の実装 → `justfile` L398-411, diff パターン
- [x] パターン: `setup-db` の実装 → `justfile` L88-91
- [x] パターン: `reset-db` の実装 → `justfile` L98-102
- [x] パターン: `check-parallel.sh` の Rust レーン → L26-30
- [x] パターン: CI `rust-integration` ジョブ → `ci.yaml` L141-225

### 変更内容

#### 1. justfile: `db-dump-schema` 追加（データストア操作セクション L204 後）

```just
# PostgreSQL: 現在のスキーマスナップショットを出力
db-dump-schema:
    #!/usr/bin/env bash
    set -euo pipefail
    pg_dump --schema-only --no-owner --no-privileges --no-tablespaces \
        --exclude-table=_sqlx_migrations \
        "{{ _psql_url }}" \
    | sed '/^-- Dumped from database version/d; /^-- Dumped by pg_dump version/d' \
    > backend/schema.sql
    echo "✓ backend/schema.sql を更新しました"
```

#### 2. justfile: `db-migrate` 追加（初回セットアップセクション、`setup-db` の後）

```just
# データベースマイグレーション実行 + スキーマスナップショット更新
db-migrate:
    @echo "マイグレーション実行中..."
    cd backend && sqlx migrate run
    just db-dump-schema
```

#### 3. justfile: `setup-db` 修正（スキーマダンプ追加）

```just
setup-db:
    @echo "データベースをセットアップ中..."
    @cd backend && sqlx migrate run
    @just db-dump-schema
    @echo "✓ データベースセットアップ完了"
```

#### 4. justfile: `reset-db` 修正（スキーマダンプ追加）

```just
reset-db:
    @echo "データベースをリセット中..."
    cd backend && sqlx database reset -y
    just db-dump-schema
    @echo "✓ データベースリセット完了"
```

#### 5. justfile: `schema-check` 追加（全チェックセクション、`openapi-check` の後）

```just
# スキーマスナップショットの同期チェック（pg_dump 出力と backend/schema.sql を比較）
schema-check:
    #!/usr/bin/env bash
    set -euo pipefail
    temp=$(mktemp)
    trap 'rm -f "$temp"' EXIT
    pg_dump --schema-only --no-owner --no-privileges --no-tablespaces \
        --exclude-table=_sqlx_migrations \
        "{{ _psql_url }}" \
    | sed '/^-- Dumped from database version/d; /^-- Dumped by pg_dump version/d' \
    > "$temp"
    if ! diff -q backend/schema.sql "$temp" > /dev/null 2>&1; then
        echo "ERROR: backend/schema.sql が現在の DB スキーマと同期していません"
        echo "  'just db-dump-schema' を実行して更新してください"
        diff --unified backend/schema.sql "$temp" || true
        exit 1
    fi
    echo "✓ backend/schema.sql は現在の DB スキーマと同期しています"
```

#### 6. justfile: `check-tools` に `pg_dump` 追加

```just
@which pg_dump > /dev/null || (echo "ERROR: pg_dump がインストールされていません" && exit 1)
```

#### 7. scripts/check-parallel.sh: Rust レーンに `schema-check` 追加

```bash
just lint-rust && \
just test-rust && \
just test-rust-integration && \
just sqlx-check && \
just schema-check && \
just openapi-check || rust_ok=false
```

#### 8. .github/workflows/ci.yaml: `rust-integration` ジョブにチェック追加

マイグレーション実行後、テスト前に配置:

```yaml
      - name: Check schema snapshot
        run: |
          pg_dump --schema-only --no-owner --no-privileges --no-tablespaces \
              --exclude-table=_sqlx_migrations \
              "$DATABASE_URL" \
          | sed '/^-- Dumped from database version/d; /^-- Dumped by pg_dump version/d' \
          > /tmp/schema-check.sql
          if ! diff -q backend/schema.sql /tmp/schema-check.sql > /dev/null 2>&1; then
            echo "::error::backend/schema.sql is out of sync. Run 'just db-dump-schema' to update."
            diff --unified backend/schema.sql /tmp/schema-check.sql || true
            exit 1
          fi
          echo "✓ Schema snapshot is in sync"
```

#### 9. backend/schema.sql: 初期生成

`just db-dump-schema` を実行して生成。

#### 10. CLAUDE.md: データストア操作セクション更新

```markdown
just db-dump-schema         # スキーマスナップショットを更新
just db-migrate             # マイグレーション + スナップショット更新
```

### テストリスト

ユニットテスト（該当なし）: シェルスクリプト/justfile レシピのため

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動検証:
- [ ] `just db-dump-schema` で `backend/schema.sql` が生成される
- [ ] 生成されたファイルにバージョン行（`Dumped from/by`）が含まれない
- [ ] `_sqlx_migrations` テーブルが含まれない
- [ ] RLS ポリシー、インデックス、制約が含まれる
- [ ] `just schema-check` が成功する（同期状態）
- [ ] `just db-dump-schema` を2回実行して差分がない（冪等性）
- [ ] `just schema-check` が失敗する（schema.sql を手動で変更後）
- [ ] `just db-migrate` でマイグレーション + スナップショット更新が実行される
- [ ] `just check` でスキーマチェックが含まれる

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `_psql_url` と CI の `DATABASE_URL` が異なる（DB名: dev vs test） | 不完全なパス | ローカルは `_psql_url`、CI は `$DATABASE_URL` を使用。スキーマ定義は DB名に依存しないため出力は同一 |
| 2回目 | `pg_dump` クライアントバージョン差異で出力が変わる可能性 | 競合・エッジケース | バージョン行を `sed` で除去し、環境間差分を防止 |
| 3回目 | `check-tools` に `pg_dump` がない | 未定義 | `check-tools` に `pg_dump` チェックを追加 |
| 4回目 | `db-migrate` コマンドが Issue で要求されているが未存在 | 未定義 | `db-migrate` レシピを新規追加（`sqlx migrate run` + `db-dump-schema`） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の完了条件4項目がすべて計画に含まれている | OK | ① db-dump-schema → 変更1, ② db-migrate → 変更2-4, ③ Git管理 → 変更9, ④ CI検証 → 変更7,8 |
| 2 | 曖昧さ排除 | pg_dump フラグ、sed パターン、配置先がすべて具体的 | OK | コードスニペットで一意に確定 |
| 3 | 設計判断の完結性 | 接続先、バージョン差異、除外テーブルの判断が記載 | OK | 設計判断セクションで3点を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象5ファイル、対象外3項目を明記 |
| 5 | 技術的前提 | pg_dump の動作、CI 環境の制約が考慮 | OK | Ubuntu ランナーの postgresql-client、DB名非依存性を確認 |
| 6 | 既存ドキュメント整合 | Issue の要件と矛盾なし | OK | 完了条件4項目と1:1で対応 |
