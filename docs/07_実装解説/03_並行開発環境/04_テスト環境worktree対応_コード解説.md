# テスト環境 worktree 対応 - コード解説

対応 PR: #624
対応 Issue: #617

## 主要な型・関数

| 型/関数 | ファイル | 責務 |
|--------|---------|------|
| `generate-env.sh` | [`scripts/generate-env.sh`](../../../scripts/generate-env.sh) | 開発環境 + テスト環境のポート計算・`.env` 一括生成 |
| `setup-env.sh` | [`scripts/setup-env.sh`](../../../scripts/setup-env.sh) | テンプレートからのフォールバックコピーを含む環境セットアップ |
| `run-api-tests.sh` | [`scripts/run-api-tests.sh`](../../../scripts/run-api-tests.sh) | API テスト実行（環境変数 source → ビルド → 起動 → hurl） |
| `run-e2e-tests.sh` | [`scripts/run-e2e-tests.sh`](../../../scripts/run-e2e-tests.sh) | E2E テスト実行（環境変数 source → ビルド → 起動 → Vite → Playwright） |
| `docker-compose.api-test.yaml` | [`infra/docker/docker-compose.api-test.yaml`](../../../infra/docker/docker-compose.api-test.yaml) | テスト用 Docker コンテナ定義（環境変数でポート参照） |
| `api-test-deps` | [`justfile`](../../../justfile) | テスト用 Docker コンテナ起動（プロジェクト名分離） |

## コードフロー

コードをライフサイクル順に追う。各ステップの構造を図で示した後、対応するコードを解説する。

```mermaid
flowchart LR
    A["1. ポート生成<br/>（generate-env.sh）"] --> B["2. Docker 起動<br/>（api-test-deps）"]
    B --> C["3. 環境変数読み込み<br/>（source .env.api-test）"]
    C --> D["4. サービス起動<br/>（cargo run）"]
    D --> E["5. テスト実行<br/>（hurl / Playwright）"]
```

### 1. ポート生成（generate-env.sh による一括生成）

`generate-env.sh` がオフセットを受け取り、開発環境とテスト環境の全ポートを計算して 3 つの `.env` ファイルを生成する。

```mermaid
flowchart TB
    Input["PORT_OFFSET（0-9）"] --> Calc["OFFSET = PORT_OFFSET × 100"]
    Calc --> Dev["開発環境ポート<br/>PG: 15432 + OFFSET<br/>BFF: 13000 + OFFSET"]
    Calc --> Test["テスト環境ポート<br/>PG: 15433 + OFFSET<br/>BFF: 14000 + OFFSET"]
    Dev --> RootEnv[".env<br/>（Docker Compose 用）"]
    Dev --> BackendEnv["backend/.env<br/>（Rust サービス用）"]
    Test --> RootEnv
    Test --> ApiTestEnv["backend/.env.api-test<br/>（テストスクリプト用）"]
```

```bash
# scripts/generate-env.sh:39-67
# 基準ポート（メインworktree用）— API テスト環境
BASE_API_TEST_POSTGRES_PORT=15433                                # ①
BASE_API_TEST_REDIS_PORT=16380
BASE_API_TEST_DYNAMODB_PORT=18001
BASE_API_TEST_BFF_PORT=14000                                     # ②
BASE_API_TEST_CORE_PORT=14001
BASE_API_TEST_AUTH_PORT=14002
BASE_API_TEST_VITE_PORT=15174

# オフセット計算（100 単位）
OFFSET=$((PORT_OFFSET * 100))

# API テスト環境ポート
API_TEST_POSTGRES_PORT=$((BASE_API_TEST_POSTGRES_PORT + OFFSET)) # ③
API_TEST_REDIS_PORT=$((BASE_API_TEST_REDIS_PORT + OFFSET))
API_TEST_DYNAMODB_PORT=$((BASE_API_TEST_DYNAMODB_PORT + OFFSET))
API_TEST_BFF_PORT=$((BASE_API_TEST_BFF_PORT + OFFSET))
API_TEST_CORE_PORT=$((BASE_API_TEST_CORE_PORT + OFFSET))
API_TEST_AUTH_PORT=$((BASE_API_TEST_AUTH_PORT + OFFSET))
API_TEST_VITE_PORT=$((BASE_API_TEST_VITE_PORT + OFFSET))
```

注目ポイント:

- ① インフラの基準ポートは開発環境の +1（15432 → 15433）
- ② サービスの基準ポートは開発環境の +1000（13000 → 14000）
- ③ 開発環境と同じ 100 単位のオフセットを適用

生成される `backend/.env.api-test` は、Rust サービスが読み込む全設定（`DATABASE_URL`, `REDIS_URL`, ポート等）を含む:

```bash
# scripts/generate-env.sh:162-225（backend/.env.api-test 生成部分、抜粋）
cat > "$PROJECT_ROOT/backend/.env.api-test" << EOF
DATABASE_URL=postgres://ringiflow:ringiflow@localhost:$API_TEST_POSTGRES_PORT/ringiflow  # ①
REDIS_URL=redis://localhost:$API_TEST_REDIS_PORT
BFF_PORT=$API_TEST_BFF_PORT                                                              # ②
CORE_PORT=$API_TEST_CORE_PORT
AUTH_PORT=$API_TEST_AUTH_PORT
CORE_URL=http://localhost:$API_TEST_CORE_PORT
AUTH_URL=http://localhost:$API_TEST_AUTH_PORT
E2E_VITE_PORT=$API_TEST_VITE_PORT                                                       # ③
ENVIRONMENT=test
EOF
```

注目ポイント:

- ① `DATABASE_URL` にテスト用ポートが埋め込まれ、テスト専用 DB に接続する
- ② `BFF_PORT` 等のサービスポートはテストスクリプトのヘルスチェックでも参照される
- ③ `E2E_VITE_PORT` は E2E テストスクリプトが Vite 起動・Playwright 接続に使用

### 2. Docker 起動（プロジェクト名 + 環境変数でポート分離）

`justfile` の `api-test-deps` がディレクトリ名からプロジェクト名を取得し、`.env` の環境変数でポートを動的に設定する。

```mermaid
flowchart LR
    PWD["basename $(pwd)<br/>→ ringiflow-auth"] --> PName["プロジェクト名:<br/>ringiflow-auth-api-test"]
    PName --> Compose["docker compose<br/>-p ringiflow-auth-api-test<br/>--env-file ../../.env"]
    Compose --> PG["postgres<br/>:15533"]
    Compose --> Redis["redis<br/>:16480"]
    Compose --> DDB["dynamodb<br/>:18101"]
```

`docker-compose.api-test.yaml` はポートをハードコードせず環境変数で参照する:

```yaml
# infra/docker/docker-compose.api-test.yaml:25-26
    ports:
      - "${API_TEST_POSTGRES_PORT}:5432"    # ① ルート .env から読み込み
```

```yaml
# infra/docker/docker-compose.api-test.yaml:42-43
    ports:
      - "${API_TEST_REDIS_PORT}:6379"       # ② 同様に環境変数参照
```

```yaml
# infra/docker/docker-compose.api-test.yaml:57-58
    ports:
      - "${API_TEST_DYNAMODB_PORT}:8000"    # ③
```

注目ポイント:

- ① Docker Compose はルートの `.env` を自動読み込みし、`${API_TEST_POSTGRES_PORT}` を展開する
- ② ③ Redis・DynamoDB も同じ方式。ステップ 1 で生成した `.env` の値が反映される

### 3. 環境変数読み込み（`set -a && source && set +a` パターン）

テストスクリプトは `backend/.env.api-test` を `source` して全環境変数をエクスポートする。これにより `cargo run` で起動される Rust サービスがテスト用ポート・DB に接続する。

```mermaid
flowchart TB
    Source["source backend/.env.api-test"] --> Export["set -a で全変数を export"]
    Export --> Cargo["cargo run が環境変数を継承"]
    Export --> Health["ヘルスチェックが $BFF_PORT 等を参照"]
    Export --> Test["hurl / Playwright がポートを参照"]
```

```bash
# scripts/run-api-tests.sh:26-31
# API テスト環境変数を読み込み
cd "$PROJECT_ROOT/backend"
set -a                      # ① 以降の変数定義を自動 export
# shellcheck disable=SC1091
source .env.api-test         # ② ファイル内の KEY=VALUE が環境変数になる
set +a                      # ③ 自動 export を解除
```

注目ポイント:

- ① `set -a` は Bash のオプションで、`source` で読み込んだ変数を自動的に `export` する
- ② `.env.api-test` には `BFF_PORT`, `CORE_PORT`, `AUTH_PORT`, `DATABASE_URL` 等が含まれる
- ③ `set +a` で以降の変数は通常の動作に戻る

このパターンは `run-api-tests.sh`（L26-31）と `run-e2e-tests.sh`（L30-35）の両方で同一。`cargo run` で起動されるサービスは `dotenv` でなく、シェルの環境変数からポート番号を取得する（`source` によりシェル環境に設定されるため、`.env` ファイルの `dotenv` 読み込みより優先される）。

### 4. サービス起動とヘルスチェック

ビルド済みバイナリをバックグラウンドで起動し、ヘルスチェックで全サービスの準備完了を待つ。

```mermaid
sequenceDiagram
    participant Script as テストスクリプト
    participant BFF as BFF :$BFF_PORT
    participant Core as Core :$CORE_PORT
    participant Auth as Auth :$AUTH_PORT

    Script->>Script: cargo build（3 サービス一括）
    Script->>BFF: cargo run -p ringiflow-bff &
    Script->>Core: cargo run -p ringiflow-core-service &
    Script->>Auth: cargo run -p ringiflow-auth-service &
    loop 最大 30 回（API テスト）/ 60 回（E2E テスト）
        Script->>BFF: curl /health
        Script->>Core: curl /health
        Script->>Auth: curl /health
    end
    Note over Script: 全サービス起動完了
```

```bash
# scripts/run-api-tests.sh:57-69
for i in {1..30}; do
    if curl -sf "http://localhost:$BFF_PORT/health" > /dev/null 2>&1 && \  # ①
       curl -sf "http://localhost:$CORE_PORT/health" > /dev/null 2>&1 && \
       curl -sf "http://localhost:$AUTH_PORT/health" > /dev/null 2>&1; then
        echo "✓ 全サービス起動完了"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "エラー: サービス起動タイムアウト" >&2
        exit 1
    fi
    sleep 1
done
```

注目ポイント:

- ① `$BFF_PORT`, `$CORE_PORT`, `$AUTH_PORT` はステップ 3 で `source` した環境変数。ハードコードされたポートは一切ない

E2E テストスクリプトはバックエンドのヘルスチェック（60 秒タイムアウト）に加え、Vite 開発サーバーの起動も待つ:

```bash
# scripts/run-e2e-tests.sh:75-92
# Vite 開発サーバーを起動（BFF_PORT でプロキシ先を API テスト BFF に向ける）
cd "$PROJECT_ROOT/frontend"
VITE_PORT=$E2E_VITE_PORT pnpm run dev &                         # ①

# Vite 開発サーバーの起動を待機
cd "$PROJECT_ROOT"
for i in {1..30}; do
    if curl -sf "http://localhost:$E2E_VITE_PORT" > /dev/null 2>&1; then  # ②
        echo "✓ Vite 開発サーバー起動完了"
        break
    fi
    ...
done
```

注目ポイント:

- ① `VITE_PORT` 環境変数で Vite のリッスンポートを指定。`$E2E_VITE_PORT` はステップ 3 で読み込み済み
- ② ヘルスチェックも同じ `$E2E_VITE_PORT` を参照

### 5. テスト実行（hurl / Playwright）

API テストと E2E テストで、動的ポートの渡し方が異なる。

```mermaid
flowchart TB
    subgraph API["API テスト（hurl）"]
        HurlVar["--variable bff_url=http://localhost:$BFF_PORT"]
        VarsEnv["--variables-file vars.env<br/>（静的テストデータのみ）"]
        HurlVar --> Hurl["hurl --test"]
        VarsEnv --> Hurl
    end

    subgraph E2E["E2E テスト（Playwright）"]
        BaseURL["E2E_BASE_URL=http://localhost:$E2E_VITE_PORT"]
        BaseURL --> PW["npx playwright test"]
    end
```

#### API テスト（hurl）

```bash
# scripts/run-api-tests.sh:72-76
# API テスト実行
echo "API テストを実行中..."
hurl --test --jobs 1 \
    --variable "bff_url=http://localhost:$BFF_PORT" \   # ① 動的ポートを注入
    --variables-file tests/api/hurl/vars.env \          # ② 静的テストデータ
    tests/api/hurl/**/*.hurl
```

注目ポイント:

- ① `bff_url` を `--variable` で動的に渡す。`vars.env` には `bff_url` を含めない（hurl は重複変数をエラーにするため）
- ② `vars.env` にはテナント ID やユーザー情報等の静的なテストデータのみを保持

`vars.env` の先頭コメントがこの設計を説明する:

```bash
# tests/api/hurl/vars.env:1-2
# API テスト用変数
# bff_url は run-api-tests.sh / CI から --variable で動的に渡される（worktree 対応）
```

#### E2E テスト（Playwright）

```bash
# scripts/run-e2e-tests.sh:94-97
# E2E テスト実行
echo "E2E テストを実行中..."
cd "$PROJECT_ROOT/tests/e2e"
E2E_BASE_URL="http://localhost:$E2E_VITE_PORT" npx playwright test  # ①
```

注目ポイント:

- ① `E2E_BASE_URL` 環境変数で Playwright のベース URL を指定。Playwright の設定ファイルがこの環境変数を参照する

## テスト

各テストがライフサイクルのどのステップを検証しているかを示す。

| テスト | 検証対象のステップ | 検証内容 |
|-------|------------------|---------:|
| `just test-api`（24 ファイル / 148 リクエスト） | 1-5 全体 | API エンドポイントの正常系・異常系を動的ポートで検証 |
| `just test-e2e`（8 テスト） | 1-5 全体 | ブラウザ操作による E2E フロー検証 |

テスト自体は既存のもの。本 PR ではテストの内容は変更せず、実行基盤のポート動的化のみを行った。

### 実行方法

```bash
# API テスト
just test-api

# E2E テスト
just test-e2e

# 全チェック（lint + unit test + API test + E2E test）
just check-all
```

## 設計解説

コード実装レベルの判断を記載する。機能・仕組みレベルの判断は[機能解説](./03_テスト環境worktree対応_機能解説.md#設計判断)を参照。

### 1. `set -a && source` パターンの採用

場所: `scripts/run-api-tests.sh:28-31`, `scripts/run-e2e-tests.sh:32-35`

```bash
set -a
source .env.api-test
set +a
```

なぜこの実装か:

`source` だけではシェル変数としてのみ定義され、子プロセス（`cargo run`）に渡らない。`set -a` で自動 export を有効にすることで、`source` で読み込んだ全変数が環境変数として子プロセスに継承される。

代替案:

| 案 | メリット | デメリット | 判断 |
|----|---------|-----------|------|
| `set -a` + `source`（採用） | シンプル、全変数を一括 export | `set +a` を忘れるリスク | 採用 |
| `export $(cat .env \| xargs)` | 1 行で完結 | コメント行やスペースの処理が不安定 | 見送り |
| `env $(cat .env)` コマンドプレフィックス | 明示的 | 複数コマンドに適用しにくい | 見送り |
| `dotenv` ツール | 堅牢 | 追加依存が必要 | 見送り |

### 2. `--variable` と `--variables-file` の分離

場所: `scripts/run-api-tests.sh:73-75`

```bash
hurl --test --jobs 1 \
    --variable "bff_url=http://localhost:$BFF_PORT" \
    --variables-file tests/api/hurl/vars.env \
```

なぜこの実装か:

hurl は `--variable` で指定した変数と `--variables-file` の変数が重複するとエラーになる（オーバーライドではない）。そのため、動的に変わる `bff_url` は `vars.env` から削除し、`--variable` でのみ渡す方式とした。

代替案:

| 案 | メリット | デメリット | 判断 |
|----|---------|-----------|------|
| `vars.env` から削除 + `--variable`（採用） | hurl 仕様と整合 | `bff_url` の定義が分散 | 採用 |
| `vars.env` をテンプレート化して `envsubst` | 一元管理 | ビルドステップが増加 | 見送り |
| `vars.env` を動的生成 | 一元管理 | 静的データまで動的化される | 見送り |

### 3. Docker Compose の `--env-file` によるポート注入

場所: `justfile` の `api-test-deps` レシピ

```bash
docker compose --env-file "$ENV_FILE" \
    -p "$PROJECT_NAME-api-test" \
    -f infra/docker/docker-compose.api-test.yaml \
    up -d --wait
```

なぜこの実装か:

Docker Compose は YAML 内の `${VARIABLE}` をファイル参照時に展開する。`--env-file` でルートの `.env`（`generate-env.sh` が生成）を指定することで、`${API_TEST_POSTGRES_PORT}` 等がオフセット適用済みの値に展開される。

代替案:

| 案 | メリット | デメリット | 判断 |
|----|---------|-----------|------|
| `--env-file`（採用） | Docker Compose 標準機能 | `.env` の配置に依存 | 採用 |
| `environment:` セクションで個別指定 | 明示的 | 設定が冗長 | 見送り |
| `docker compose` の `--build-arg` | ビルド時に注入 | ランタイムに影響しない | 見送り |

## 関連ドキュメント

- [機能解説](./03_テスト環境worktree対応_機能解説.md)
- [並行開発環境 - コード解説](./02_並行開発環境_コード解説.md)（PR #96、開発環境の worktree 対応）
- 手順書: [並行開発（Worktree）](../../04_手順書/04_開発フロー/04_並行開発（Worktree）.md)
