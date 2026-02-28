# 計画: #617 API テスト・E2E テストを worktree 並行開発に対応させる

## コンテキスト

worktree 並行開発において、開発環境（`dev-deps`）は `generate-env.sh` による動的プロジェクト名＋ポートオフセットで完全分離されているが、テスト環境（`api-test-deps`）は全てハードコードされたポート・プロジェクト名を使用している。2 つの worktree から同時に `just test-api` / `just test-e2e` を実行するとポート競合・DB 汚染が発生する。

dev-deps の動的化パターンを api-test-deps に横展開する。

## 対象

- `scripts/generate-env.sh` — API テスト用ポート生成を追加
- `scripts/setup-env.sh` — `.env.api-test.template` コピーを追加
- `.env.template` — API テスト Docker ポートを追加
- `backend/.env.api-test` → `backend/.env.api-test.template` に改名
- `.gitignore` — `backend/.env.api-test` を追加
- `infra/docker/docker-compose.api-test.yaml` — 環境変数参照に変更
- `justfile` — `api-test-*` ターゲットを動的化
- `scripts/run-api-tests.sh` — ポートを環境変数から取得
- `scripts/run-e2e-tests.sh` — ポートを環境変数から取得
- `tests/api/hurl/vars.env` — `bff_url` を `--variable` で動的上書き
- `.github/workflows/ci.yaml` — `setup-env` 追加＋ハードコード排除
- `docs/60_手順書/04_開発フロー/04_並行開発（Worktree）.md` — テスト実行の注意事項追記

## 対象外

- `tests/e2e/playwright.config.ts` — 既に `E2E_BASE_URL` 環境変数対応済み。変更不要
- `tests/api/hurl/vars.env` の構造変更 — テストデータ部分はそのまま。`bff_url` もデフォルト値として残す

## 設計判断

### ポートスキーム

API テスト用基準ポートを独立定義し、開発環境と同じ 100 単位のオフセットを適用する。

| サービス | 開発基準 | テスト基準 | 差分 |
|---------|---------|---------|------|
| PostgreSQL | 15432 | 15433 | +1 |
| Redis | 16379 | 16380 | +1 |
| DynamoDB | 18000 | 18001 | +1 |
| BFF | 13000 | 14000 | +1000 |
| Core | 13001 | 14001 | +1000 |
| Auth | 13002 | 14002 | +1000 |
| Vite | 15173 | 15174 | +1 |

オフセット適用例:

| | Offset 0 (main) | Offset 1 (wt 1) |
|---|---|---|
| Dev PostgreSQL | 15432 | 15532 |
| Test PostgreSQL | 15433 | 15533 |
| Dev BFF | 13000 | 13100 |
| Test BFF | 14000 | 14100 |
| Dev Vite | 15173 | 15273 |
| Test Vite | 15174 | 15274 |

理由: 既存のオフセット体系（100 単位）に自然に統合でき、開発ポートとテストポートが近い番号で配置されるため管理しやすい。

### `backend/.env.api-test` の動的生成化

現在は git 追跡されたファイル。これを `generate-env.sh` で動的生成するように変更する。

- `backend/.env.api-test` → `backend/.env.api-test.template`（git 追跡、テンプレート）
- `backend/.env.api-test`（git 無視、動的生成）

理由: `backend/.env`（開発用）と同じパターン。worktree ごとに異なるポートが必要なため、追跡ファイルでは対応できない。

### `vars.env`（hurl 変数ファイル）の扱い

`tests/api/hurl/vars.env` は git 追跡のまま維持し、`bff_url` のデフォルト値（`http://localhost:14000`）を残す。`run-api-tests.sh` と CI で `--variable bff_url=http://localhost:$BFF_PORT` を指定し、動的値で上書きする。

理由: hurl の `--variable` は `--variables-file` より優先される（公式仕様）。ファイル構造を変更せずに動的化できる。テストデータ（ユーザー ID 等）は静的なので分離不要。

### テストスクリプトのリファクタリング

`run-api-tests.sh` / `run-e2e-tests.sh` で `.env.api-test` を `source` する方式に変更する。

現在: `env_vars=$(grep ... .env.api-test | xargs)` → `env $env_vars cargo run ...`
変更後: `set -a && source .env.api-test && set +a` → `cargo run ...`

理由: `source` により全変数がスクリプト環境に読み込まれ、ヘルスチェック等で直接 `$BFF_PORT` を参照できる。CI と同じパターン。

### Docker Compose プロジェクト名

`justfile` の `api-test-deps` で `$(basename "$(pwd)")-api-test` を使用する。

- Main: `ringiflow-api-test`
- Worktree: `ringiflow-auth-api-test`

理由: `dev-deps` の `$(basename "$(pwd)")` パターンに `-api-test` サフィックスを付加。

### `E2E_VITE_PORT` の配置

`backend/.env.api-test` に `E2E_VITE_PORT` を追加する。

理由: `run-e2e-tests.sh` は `backend/.env.api-test` から全テスト環境設定を読み込む。E2E Vite ポートもテスト環境の一部。スクリプト内で `VITE_PORT=$E2E_VITE_PORT pnpm run dev` として Vite に渡す。

---

## Phase 1: 環境変数ファイル体系の変更

### 確認事項
- [x] 既存パターン: `generate-env.sh` のポート計算ロジック → L30-48, BASE + OFFSET 方式
- [x] 既存パターン: `setup-env.sh` のテンプレートコピー → L72-76, `cp .env.template .env` 方式
- [x] 既存パターン: `.gitignore` の `.env` パターン → L147, `.env` は全ディレクトリで無視されるが `.env.api-test` は別途追加が必要

### 変更内容

1. `git mv backend/.env.api-test backend/.env.api-test.template`
2. `.gitignore` に `backend/.env.api-test` を追加（L147 の `.env` の下）
3. `backend/.env.api-test.template` に `E2E_VITE_PORT=15174` を追加
4. `.env.template` に API テスト Docker ポートを追加:
   ```
   API_TEST_POSTGRES_PORT=15433
   API_TEST_REDIS_PORT=16380
   API_TEST_DYNAMODB_PORT=18001
   ```
5. `generate-env.sh` を拡張:
   - API テスト基準ポートの定義（L31-37 の下）
   - API テストポートのオフセット計算
   - ルート `.env` に API テスト Docker ポートを追記
   - `backend/.env.api-test` の動的生成（`backend/.env` と同構造）
   - 出力メッセージに API テストポートを追加
6. `setup-env.sh` を拡張:
   - メイン worktree 分岐（L72-76）に `cp backend/.env.api-test.template backend/.env.api-test` を追加

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `./scripts/generate-env.sh 0` → `backend/.env.api-test` が offset 0 で生成される
- [ ] `./scripts/generate-env.sh 1` → `backend/.env.api-test` が offset 1（+100）で生成される
- [ ] ルート `.env` に `API_TEST_POSTGRES_PORT` 等が含まれる

---

## Phase 2: Docker Compose & justfile

### 確認事項
- [x] 既存パターン: `docker-compose.yaml` の環境変数参照 → `"${POSTGRES_PORT}:5432"` 方式
- [x] 既存パターン: `justfile` の `dev-deps` → `docker compose --env-file .env -p "$PROJECT_NAME"` 方式

### 変更内容

1. `infra/docker/docker-compose.api-test.yaml`:
   - `"15433:5432"` → `"${API_TEST_POSTGRES_PORT}:5432"`
   - `"16380:6379"` → `"${API_TEST_REDIS_PORT}:6379"`
   - `"18001:8000"` → `"${API_TEST_DYNAMODB_PORT}:8000"`
   - コメントのポート番号を更新

2. `justfile` の `api-test-deps`:
   ```bash
   #!/usr/bin/env bash
   PROJECT_NAME="$(basename "$(pwd)")-api-test"
   docker compose --env-file .env -p "$PROJECT_NAME" -f infra/docker/docker-compose.api-test.yaml up -d --wait
   echo "API テスト環境:"
   echo "  PostgreSQL: localhost:${API_TEST_POSTGRES_PORT}"
   echo "  Redis: localhost:${API_TEST_REDIS_PORT}"
   echo "  DynamoDB: localhost:${API_TEST_DYNAMODB_PORT}"
   echo "  プロジェクト名: $PROJECT_NAME"
   ```

3. `justfile` の `api-test-reset-db`:
   ```
   cd backend && DATABASE_URL=postgres://ringiflow:ringiflow@localhost:${API_TEST_POSTGRES_PORT}/ringiflow sqlx database reset -y
   ```

4. `justfile` の `api-test-stop`, `api-test-clean`:
   ```bash
   #!/usr/bin/env bash
   PROJECT_NAME="$(basename "$(pwd)")-api-test"
   docker compose -p "$PROJECT_NAME" -f infra/docker/docker-compose.api-test.yaml down [-v]
   ```

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `just api-test-deps` → 正しいポートで Docker コンテナ起動
- [ ] `just api-test-reset-db` → 正しいポートで DB リセット
- [ ] `just api-test-stop` → コンテナ停止
- [ ] `just api-test-clean` → コンテナ＋ボリューム削除

---

## Phase 3: テストスクリプト

### 確認事項
- [x] ライブラリ: hurl `--variable` が `--variables-file` より優先されるか → 公式仕様で確認要（Grep で既存の `--variable` 使用を検索）
- [x] パターン: CI の `.env.api-test` 読み込み方式 → `set -a && source .env.api-test && set +a`（ci.yaml L487, L617）

### 変更内容

1. `scripts/run-api-tests.sh`:
   - `env_vars=$(grep ...)` → `set -a && source "$PROJECT_ROOT/backend/.env.api-test" && set +a`
   - `env $env_vars cargo run ...` → `cargo run ...`（環境変数は source で設定済み）
   - ヘルスチェック: `http://localhost:14000` → `http://localhost:$BFF_PORT`（同様に CORE_PORT, AUTH_PORT）
   - hurl コマンド: `--variable bff_url=http://localhost:$BFF_PORT` を追加

2. `scripts/run-e2e-tests.sh`:
   - 同様に `source` 方式に変更
   - ヘルスチェック: 動的ポート参照
   - Vite 起動: `VITE_PORT=15173 BFF_PORT=14000` → `VITE_PORT=$E2E_VITE_PORT`（`BFF_PORT` は source 済み）
   - Vite ヘルスチェック: `http://localhost:15173` → `http://localhost:$E2E_VITE_PORT`
   - Playwright: `E2E_BASE_URL=http://localhost:15173` → `E2E_BASE_URL=http://localhost:$E2E_VITE_PORT`

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）

API テスト:
- [ ] `just test-api` が通る

E2E テスト:
- [ ] `just test-e2e` が通る

---

## Phase 4: CI ワークフロー

### 確認事項
- [ ] CI の `just` コマンド実行前に `.env` が必要か → `dotenv-load` は `.env` 不在でもエラーにならない（just の仕様）。ただし `api-test-deps` が `${API_TEST_POSTGRES_PORT}` を参照するため、`.env` が必要

### 変更内容

1. `api-test` ジョブ（L404-516）:
   - `Start dependencies` の前に `Setup environment` ステップを追加: `just setup-env`
   - `Run API tests` ステップ内:
     - `source .env.api-test` は既存のまま（ポートが動的になるだけ）
     - ヘルスチェック: `http://localhost:14000` → `http://localhost:$BFF_PORT`（同様に CORE_PORT, AUTH_PORT）
     - hurl コマンド: `--variable bff_url=http://localhost:$BFF_PORT` を追加

2. `e2e-test` ジョブ（L518-674）:
   - `Start dependencies` の前に `Setup environment` ステップを追加: `just setup-env`
   - `Run E2E tests` ステップ内:
     - ヘルスチェック: 動的ポート参照
     - Vite 起動: `VITE_PORT=15173 BFF_PORT=14000` → `VITE_PORT=$E2E_VITE_PORT BFF_PORT=$BFF_PORT`
     - Vite ヘルスチェック: `http://localhost:15173` → `http://localhost:$E2E_VITE_PORT`
     - Playwright: `E2E_BASE_URL=http://localhost:15173` → `E2E_BASE_URL=http://localhost:$E2E_VITE_PORT`

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `just check-all` がローカルで通る（CI 相当の全テスト）

---

## Phase 5: ドキュメント

### 確認事項: なし（既知のパターンのみ）

### 変更内容

1. `docs/60_手順書/04_開発フロー/04_並行開発（Worktree）.md`:
   - 「テスト実行」セクションを追加
   - ポートオフセット表に API テスト用ポートを追加
   - `just test-api` / `just test-e2e` が worktree 間で独立して実行できることを説明

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `backend/.env.api-test` が git 追跡ファイル。動的生成化するとテンプレート管理が必要 | 未定義 | テンプレートパターン（`.env.api-test.template`）を設計に追加。`setup-env.sh` にコピーロジックを追加 |
| 2回目 | E2E テストの Vite ポートが未考慮。現在は開発環境と同じ 15173 で、worktree 間で競合する | 不完全なパス | `E2E_VITE_PORT` を `.env.api-test` に追加。テスト基準ポート 15174（dev+1）を設計に追加 |
| 3回目 | CI ワークフローも同じハードコードポートを使用。ローカルスクリプトだけ変更しても CI が壊れる | 未定義 | Phase 4 として CI ワークフロー変更を計画に追加。`just setup-env` ステップの挿入 |
| 4回目 | `hurl vars.env` の `bff_url` — ファイルを動的生成するか、`--variable` で上書きするか | シンプルさ | `--variable` 上書き方式を選択。ファイル構造変更なし、hurl 公式仕様で優先順位が保証 |
| 5回目 | `worktree-add.sh` L125 の環境変数クリアリスト — API テスト用変数の追加が必要か | 競合・エッジケース | `setup-worktree` は `dev-deps` のみ依存し `api-test-deps` は含まない。API テスト Docker ポート変数は `justfile` の `api-test-*` 実行時に `.env` から読まれるため、クリア不要 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue 記載の8ファイル＋CI＋ドキュメントが全て計画に含まれている | OK | Issue 変更対象8ファイル: generate-env.sh, docker-compose.api-test.yaml, run-api-tests.sh, run-e2e-tests.sh, vars.env, playwright.config.ts, justfile, 手順書。追加: setup-env.sh, .env.template, .gitignore, ci.yaml。playwright.config.ts は変更不要（根拠: 既に E2E_BASE_URL 対応済み） |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の変更内容にコード片を含め、変更箇所が一意に特定可能 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | ポートスキーム、ファイル管理方式、vars.env の扱い、スクリプトリファクタリング、プロジェクト名命名、E2E_VITE_PORT 配置 — 6つの設計判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象: 12ファイル。対象外: playwright.config.ts（理由付き）、vars.env 構造変更（理由付き） |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | hurl `--variable` 優先順位（公式仕様）、just `dotenv-load` の `.env` 不在時挙動、`source` のコメント行処理 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | 並行開発手順書の既存内容（ポートオフセット表、worktree-add フロー）と整合。ADR 確認: 関連する ADR なし |
