# CI/CD 構築手順

## 目的

GitHub Actions を使用した CI（継続的インテグレーション）パイプラインを構築する。
この手順完了後、プッシュ時に自動でリント・テスト・ビルドが実行される。

## 前提条件

- GitHub リポジトリへのプッシュ権限があること
- `.github/workflows/` ディレクトリが存在すること

```bash
# ディレクトリ確認
ls -la .github/workflows/
# .gitkeep または空であること
```

---

## 概要

GitHub Actions で Rust / Elm の CI を構築する。
設計の詳細は [ADR-004: CI 並列化と変更検出](../../04_ADR/004_CI並列化と変更検出.md) を参照。

→ 参照:
- [`/.github/workflows/ci.yml`](/.github/workflows/ci.yml)
- [`/.github/dependabot.yml`](/.github/dependabot.yml)

---

## 1. SQLx オフラインモードの設定

CI 環境では PostgreSQL に接続できないため、SQLx のオフラインモードを使用する。

### .sqlx ディレクトリの準備

ローカル環境で以下を実行:

```bash
cd apps/core-api

# データベースが起動していることを確認
docker compose -f ../../infra/docker/docker-compose.yml ps

# クエリメタデータを生成
cargo sqlx prepare --workspace
```

期待される出力:

```
query data written to `.sqlx` in the workspace root
```

### .sqlx のコミット

```bash
# .sqlx ディレクトリをコミット
git add .sqlx
git commit -m ".sqlx を追加"
```

**重要**: `.sqlx` ディレクトリは `.gitignore` に含めないこと。

---

## 2. pnpm-lock.yaml の生成

CI では `pnpm install --frozen-lockfile` を使用するため、`pnpm-lock.yaml` が必要。

```bash
cd apps/web
pnpm install
# pnpm-lock.yaml が生成される

git add pnpm-lock.yaml
git commit -m "pnpm-lock.yaml を追加"
```

---

## 3. ブランチ保護ルールの設定（GitHub UI）

GitHub リポジトリの Settings > Branches で設定する。

### main ブランチの保護

1. Branch name pattern: `main`
2. Protect matching branches:
   - ✅ Require a pull request before merging
     - ✅ Require approvals: 1（個人開発なら 0）
   - ✅ Require status checks to pass before merging
     - ✅ Require branches to be up to date before merging
     - Status checks: `CI Success`
   - ✅ Do not allow bypassing the above settings

### 設定手順

```
1. GitHub リポジトリページを開く
2. Settings タブをクリック
3. 左メニューの Branches をクリック
4. Add branch protection rule をクリック
5. 上記の設定を入力
6. Create をクリック
```

---

## 4. ローカルでの CI 実行確認

プッシュ前にローカルで CI と同等のチェックを実行する。

### just での一括実行

```bash
# プロジェクトルートで
just ci
```

### 個別実行

```bash
# Rust
just fmt-check-rust
just lint-rust
just test-rust

# Elm
just fmt-check-elm
just test-elm
```

---

## 5. 初回プッシュと CI 確認

### コミットとプッシュ

```bash
# ワークフローファイルを追加
git add .github/workflows/ci.yml
git add .github/dependabot.yml

# コミット
git commit -m "GitHub Actions CI を追加"

# プッシュ
git push origin main
```

### CI 実行確認

1. GitHub リポジトリページを開く
2. Actions タブをクリック
3. 最新のワークフロー実行をクリック
4. 変更されたファイルに応じたジョブのみ実行されることを確認
   - Rust 関連のみ変更: Rust ジョブのみ実行
   - Elm 関連のみ変更: Elm ジョブのみ実行
   - 両方変更: 両ジョブが並列実行
5. CI Success が緑になることを確認

---

## 6. 完了確認チェックリスト

| 項目 | 確認方法 | 期待結果 |
|------|---------|----------|
| ci.yml 存在 | `cat .github/workflows/ci.yml` | ファイル内容表示 |
| ローカル Rust チェック | `just fmt-check-rust && just lint-rust && just test-rust` | 終了コード 0 |
| ローカル Elm チェック | `just fmt-check-elm && just test-elm` | 終了コード 0 |
| ローカル全チェック | `just ci` | 終了コード 0 |
| GitHub Actions 成功 | GitHub Actions タブ | CI Success 緑 |

---

## トラブルシューティング

### `cargo fmt --check` が失敗

```bash
# ローカルでフォーマット実行
cargo fmt --all

# 差分を確認してコミット
git diff
git add -A
git commit -m "コードを整形"
```

### `pnpm install --frozen-lockfile` が失敗

```bash
# pnpm-lock.yaml が存在するか確認
ls apps/web/pnpm-lock.yaml

# 存在しない場合は生成
cd apps/web
pnpm install
git add pnpm-lock.yaml
git commit -m "pnpm-lock.yaml を追加"
```

### SQLx オフラインモードエラー

```bash
# .sqlx が最新か確認
ls -la .sqlx/

# 再生成
cd apps/core-api
cargo sqlx prepare --workspace

# コミット
git add .sqlx
git commit -m ".sqlx を更新"
```

### キャッシュが効かない

```yaml
# ワークフローでキャッシュキーを確認
key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
```

Cargo.lock が変更されるとキャッシュが無効になる。

### GitHub Actions のタイムアウト

```yaml
jobs:
  rust:
    timeout-minutes: 30  # タイムアウト時間を延長
```

---

## 次のステップ

CI/CD 構築が完了したら、[`02_Terraform基盤構築.md`](02_Terraform基盤構築.md) に進む。

---

## 変更履歴

| 日付 | 変更内容 | 担当 |
|------|---------|------|
| 2026-01-13 | 初版作成 | - |
