# Issue #548: すべての環境のDB名を ringiflow に統一する

## Context

### 背景と目的

現在、環境ごとに異なるDB名を使用している:
- 開発環境: `ringiflow_dev`
- APIテスト環境: `ringiflow_api_test`
- CI環境: `ringiflow_test`
- 本番環境: `ringiflow_prod`

しかし、現在の分離戦略は「PostgreSQLインスタンスごと分離」（別ポート、別コンテナ、別AWS環境）であり、DB名での分離は冗長である。DB名を統一することで、シンプルさと認知負荷の軽減が期待できる（KISS原則）。

### Want

プロジェクト理念「品質の追求」に基づくシンプルさの維持（KISS原則）、認知負荷の軽減

### To-Be

すべての環境でDB名が `ringiflow` に統一されている。環境識別はポート番号・ホスト名で明確に行われる。

### As-Is

探索により、DB名参照が **18ファイル、24箇所** で確認された。以下の環境で異なるDB名を使用:
- 開発環境: 4ファイル
- APIテスト環境: 4ファイル
- CI環境: 2ファイル
- 本番環境: 4ファイル
- ドキュメント: 4ファイル

## 実装計画

### Phase 1: Docker Compose ファイル更新

Docker Compose の PostgreSQL 環境変数と healthcheck を更新する。

#### 確認事項

なし（既知のパターンのみ）

#### 実装内容

**1. `infra/docker/docker-compose.yaml`**:
- L29: `POSTGRES_DB: ringiflow_dev` → `POSTGRES_DB: ringiflow`
- L41: `pg_isready -U ringiflow -d ringiflow_dev` → `pg_isready -U ringiflow -d ringiflow`

**2. `infra/docker/docker-compose.api-test.yaml`**:
- L21: `POSTGRES_DB: ringiflow_api_test` → `POSTGRES_DB: ringiflow`
- L30: `pg_isready -U ringiflow -d ringiflow_api_test` → `pg_isready -U ringiflow -d ringiflow`

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

動作確認:
- [ ] Docker Compose 起動確認（開発環境）
- [ ] Docker Compose 起動確認（APIテスト環境）

### Phase 2: 環境変数・設定ファイル更新

環境変数ファイルとテンプレートの DATABASE_URL を更新する。

#### 確認事項

なし（既知のパターンのみ）

#### 実装内容

**1. `backend/.env.template`**:
- L8: `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15432/ringiflow_dev` → `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15432/ringiflow`

**2. `backend/.env.api-test`**:
- L10: `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15433/ringiflow_api_test` → `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15433/ringiflow`

**3. `infra/lightsail/.env.example`**:
- L13: `POSTGRES_DB=ringiflow_prod` → `POSTGRES_DB=ringiflow`

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

動作確認:
- [ ] `.env` 再生成後、開発環境起動確認
- [ ] API テスト環境起動確認

### Phase 3: CI/CD 更新

GitHub Actions ワークフローの PostgreSQL 設定を更新する。

#### 確認事項

なし（既知のパターンのみ）

#### 実装内容

**1. `.github/workflows/ci.yaml`**:
- L192: `POSTGRES_DB: ringiflow_test` → `POSTGRES_DB: ringiflow`
- L224: `DATABASE_URL: postgres://ringiflow:ringiflow@localhost:15432/ringiflow_test` → `DATABASE_URL: postgres://ringiflow:ringiflow@localhost:15432/ringiflow`

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

動作確認:
- [ ] CI ワークフローが正常に動作することを確認（PR 作成後）

### Phase 4: スクリプト更新

スクリプトファイルと justfile の DB 名参照を更新する。

#### 確認事項

- [x] justfile: `_psql_url` 変数の定義と使用箇所 → `justfile` L188
- [x] justfile: `api-test-reset-db` タスクの実装 → `justfile` L322（DATABASE_URL 内の `ringiflow_api_test` を `ringiflow` に置換）

#### 実装内容

**1. `scripts/generate-env.sh`**:
- L83: `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:${POSTGRES_PORT}/ringiflow_dev` → `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:${POSTGRES_PORT}/ringiflow`

**2. `justfile`**:
- L188: `_psql_url := "postgres://ringiflow:ringiflow@localhost:15432/ringiflow_dev"` → `_psql_url := "postgres://ringiflow:ringiflow@localhost:15432/ringiflow"`
- L322: `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15433/ringiflow_api_test` → `DATABASE_URL=postgres://ringiflow:ringiflow@localhost:15433/ringiflow`

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

動作確認:
- [ ] `just db-tables` で開発環境 DB に接続できることを確認
- [ ] `just api-test-reset-db` が正常に動作することを確認

### Phase 5: 本番環境ファイル更新

Lightsail 関連ファイルの DB 名参照を更新する。

#### 確認事項

- [x] `infra/lightsail/docker-compose.yaml` の存在と POSTGRES_DB の定義 → 存在する。L142, L149 で `${POSTGRES_DB}` 環境変数を参照しているため、ファイル自体の変更は不要

#### 実装内容

**1. `infra/lightsail/backup.sh`**:
- L57: `POSTGRES_DB=${POSTGRES_DB:-ringiflow_prod}` → `POSTGRES_DB=${POSTGRES_DB:-ringiflow}`

**2. `infra/lightsail/README.md`**:
- L133, L259 付近の `ringiflow_prod` → `ringiflow` に置換（具体的な箇所は実装時に確認）

**3. `infra/lightsail/docker-compose.yaml`**:
- 変更不要（環境変数 `${POSTGRES_DB}` で参照しているため、`.env.example` の変更で反映される）

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

動作確認:
- [ ] backup.sh のデフォルト値が `ringiflow` になっていることを確認
- [ ] README.md の手順が正確であることを確認

### Phase 6: ドキュメント更新

ドキュメントとレシピ内の DB 名参照を更新する。

#### 確認事項

なし（既知のパターンのみ）

#### 実装内容

**1. `docs/90_実装解説/02_APIテスト/02_APIテスト_コード解説.md`**:
- L232, L246 の `ringiflow_api_test` → `ringiflow` に置換

**2. `docs/60_手順書/01_開発参画/02_プロジェクトセットアップ.md`**:
- L94 の `ringiflow_dev` → `ringiflow` に置換

**3. `prompts/recipes/Lightsailデプロイ_IPv6.md`**:
- L27 の `ringiflow_prod` → `ringiflow` に置換

**4. `prompts/plans/468_db-schema-snapshot.md`**:
- L57, L59 の `ringiflow_dev`, `ringiflow_test` → `ringiflow` に置換

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

動作確認:
- [ ] ドキュメントの記述が正確であることを確認（読み直し）

### Phase 7: 動作確認

全環境での動作確認を実施する。

#### 確認事項

なし（既知のパターンのみ）

#### 実装内容

**1. 開発環境**:
1. `.env` ファイルを再生成: `just setup-env`
2. 依存サービスを再起動: `just dev-down && just dev-deps`
3. マイグレーション実行: `cd backend && sqlx migrate run`
4. DB 接続確認: `just db-tables`

**2. API テスト環境**:
1. API テスト DB をリセット: `just api-test-reset-db`
2. API テスト実行: `just test-api`

**3. CI**:
1. PR を作成し、GitHub Actions が正常に動作することを確認

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト:
- [ ] `just test-api` がパスすることを確認

E2E テスト: 該当なし

動作確認:
- [ ] 開発環境が正常に起動し、DB 接続できることを確認
- [ ] API テスト環境が正常に動作することを確認
- [ ] CI が正常にパスすることを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `infra/lightsail/docker-compose.yaml` の存在が未確認 | 未定義 | Phase 5 の確認事項に追加 |
| 2回目 | `justfile` の `api-test-reset-db` タスク内の具体的な置換箇所が不明確 | 未定義 | Phase 4 の確認事項に追加、実装時に Read で確認する |
| 3回目 | 各 Phase にテストリストがなく、テストピラミッドの層が明示されていない | 不完全なパス | 各 Phase にテストリスト（ユニット/ハンドラ/API/E2E）を追加。このタスクは設定変更のため、ほとんどが「該当なし」だが、動作確認は必須 |
| 4回目 | Phase 5 で `infra/lightsail/docker-compose.yaml` の変更が必要と記載されていたが、実際は環境変数参照のため不要 | 曖昧 | Phase 5 の実装内容を「変更不要」に修正。確認事項を「確認済み」に更新 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 探索で発見された全対象が計画に含まれている | OK | Explore エージェントが報告した18ファイル24箇所をすべて Phase 1〜6 でカバー。除外なし |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 「必要に応じて」「あれば」等の不確定表現なし。存在が不明なファイル（lightsail/docker-compose.yaml）は確認事項に明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載されている | OK | DB名の置換のみで、設計判断は不要。すべて `ringiflow` への機械的置換 |
| 4 | スコープ境界 | 対象と対象外が両方明記されている | OK | 対象: 18ファイル24箇所（Phase 1〜6）、対象外: なし（探索で発見された全箇所を対象） |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | PostgreSQL の healthcheck コマンド仕様、環境変数の読み込みタイミングを考慮。Phase 7 で `.env` 再生成とサービス再起動を明示 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | Issue #548 の As-Is と一致。ADR には該当なし（DB名は実装詳細） |

## 検証方法

### 1. 開発環境

```bash
# 環境変数再生成
just setup-env

# 依存サービス再起動
just dev-down
just dev-deps

# マイグレーション実行
cd backend && sqlx migrate run

# DB 接続確認
just db-tables
# → ringiflow のテーブル一覧が表示されること
```

### 2. API テスト環境

```bash
# API テスト DB リセット
just api-test-reset-db
# → ringiflow データベースが再作成されること

# API テスト実行
just test-api
# → すべてのテストがパスすること
```

### 3. CI

```bash
# PR 作成後、GitHub Actions の rust-integration ジョブを確認
# → POSTGRES_DB=ringiflow, DATABASE_URL に ringiflow が含まれていること
# → すべてのジョブがパスすること
```

### 4. 全体チェック

```bash
# すべての環境で DB 名が ringiflow に統一されていることを確認
just check-all
# → すべてのテストがパスすること
```

## 重要な設計判断

### 判断1: DB 名を環境で分けない

**理由**: 現在の分離戦略は「PostgreSQLインスタンスごと分離」（別ポート、別コンテナ、別AWS環境）であり、DB名での分離は冗長。

**代替案**:
- A案（採用）: すべて `ringiflow` に統一
- B案（不採用）: 環境ごとに異なるDB名を維持

**トレードオフ**:
- メリット: シンプルさ、認知負荷の軽減、設定ファイルの一貫性
- デメリット: なし（インスタンスレベルで既に分離されている）

### 判断2: 段階的な置換ではなく一括置換

**理由**: DB名は環境変数と設定ファイルのみに存在し、コードからは間接的に参照される。一括置換してもリスクは低い。

**代替案**:
- A案（採用）: すべての環境を一括で `ringiflow` に置換
- B案（不採用）: 開発環境 → API テスト → CI → 本番の順に段階的に置換

**トレードオフ**:
- メリット: PR 数が減り、レビューコストが低い。一貫性が保たれる
- デメリット: 一度に多くのファイルを変更するため、レビュー時の認知負荷がやや高い

## Critical Files

- `infra/docker/docker-compose.yaml`
- `infra/docker/docker-compose.api-test.yaml`
- `backend/.env.template`
- `backend/.env.api-test`
- `.github/workflows/ci.yaml`
- `scripts/generate-env.sh`
- `justfile`
- `infra/lightsail/.env.example`
- `infra/lightsail/backup.sh`
- `infra/lightsail/README.md`
- ドキュメント4ファイル

---

**実装方針**: 各 Phase を順に実行し、Phase 7 で全環境の動作確認を実施する。置換は機械的だが、healthcheck や環境変数の読み込みタイミングに注意する。
