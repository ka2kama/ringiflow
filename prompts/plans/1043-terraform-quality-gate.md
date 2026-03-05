# #1043 Terraform 品質ゲートと CI 統合

## コンテキスト

### 目的
- Issue: #1043
- Want: Terraform ファイルの品質検証（fmt, validate, lint）をローカル品質ゲートと CI に統合し、Rust/Elm と同等の品質保証を実現する
- 完了基準:
  - `.tf` ファイルを変更した PR で CI の Terraform 検証ジョブが実行される
  - `terraform fmt` 違反が品質ゲートで検出される
  - `terraform validate` 失敗が品質ゲートで検出される

### ブランチ / PR
- ブランチ: `feature/1043-terraform-quality-gate`
- PR: #1044（Draft）

### As-Is（探索結果の要約）
- `.tf` ファイル: `infra/terraform/environments/dev/`（6 ファイル）+ `infra/terraform/modules/ses/`（3 ファイル）
- prod/stg 環境: 空ディレクトリ（`.tf` ファイルなし）
- `.mise.toml`: terraform/tflint 未登録（node, elm, elm-format のみ）
- `justfile`: terraform 関連コマンドなし。`check-tools` にも terraform チェックなし
- `lefthook.yaml`: pre-commit に rustfmt-check, elm-format-check あり。terraform なし
- `ci.yaml`: `changes` ジョブに terraform パスフィルターなし。terraform ジョブなし
- `scripts/check/parallel.sh`: Non-Rust レーンで各種 lint を並列実行。terraform なし
- `terraform validate` は `terraform init` が必要（プロバイダースキーマのダウンロード）
- `terraform fmt -check` は init 不要
- `tflint` は `--init` でプラグインを自動ダウンロード可能

### 進捗
- [x] Phase 1: ツールチェーン整備（mise, tflint 設定, just setup）
- [x] Phase 2: ローカル品質ゲート（just check, lefthook）
- [x] Phase 3: CI 統合（ci.yaml に terraform ジョブ追加）
- [x] Phase 4: ドキュメント（ナレッジベース, ルールファイル更新）

## 設計判断

### terraform validate の実行方式

`terraform validate` は `terraform init` が必要。CI とローカルで異なるアプローチを取る。

| 環境 | 方式 | 理由 |
|------|------|------|
| CI | `terraform init -backend=false` → `terraform validate` | State バックエンド（S3）接続不要。プロバイダーのみダウンロード |
| ローカル | `.terraform` ディレクトリ存在チェック → 存在すれば validate、なければスキップ | init は重い操作。開発者が明示的に init した環境のみ検証 |

### tflint の設定

`.tflint.hcl` をプロジェクトルートではなく `infra/terraform/` に配置する。Terraform ファイルと同じディレクトリツリーに置くことで、tflint が自動検出する。

AWS プラグイン（`tflint-ruleset-aws`）を使用。AWS リソースのベストプラクティス違反（存在しないインスタンスタイプ、非推奨のリソース等）を検出する。

### ローカル品質ゲートの条件付き実行

`.tf` ファイルが存在しない環境（prod, stg）や Terraform 未インストール環境では品質チェックをスキップする。

- `just check` の terraform チェック: `which terraform` で存在確認。なければスキップ（警告出力）
- lefthook の terraform fmt: `.tf` ファイルの staged files がなければ自動スキップ（glob フィルター）

### CI での Terraform バージョン管理

`hashicorp/setup-terraform` アクションを使用。`versions.tf` の `required_version` と整合するバージョンを指定する。

## Phase 1: ツールチェーン整備

### 確認事項
- パターン: `.mise.toml` の既存エントリ形式 → `.mise.toml`（確認済み: `[tools]` セクションに `キー = "バージョン"` 形式）
- パターン: `justfile` の `check-tools` セクション → `justfile:37-65`（確認済み: `which xxx || echo "ERROR: ..."` パターン）
- ライブラリ: tflint のプラグイン設定形式 → tflint 公式ドキュメント

### 操作パス: 該当なし（操作パスが存在しない）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `mise install` で terraform と tflint がインストールされる
- [ ] `just check-tools` で terraform と tflint が検出される

### 実装内容
1. `.mise.toml` に terraform と tflint を追加
2. `infra/terraform/.tflint.hcl` を作成（AWS プラグイン設定）
3. `justfile` の `check-tools` に terraform と tflint のチェックを追加

## Phase 2: ローカル品質ゲート

### 確認事項
- パターン: `scripts/check/parallel.sh` の Non-Rust レーン → `scripts/check/parallel.sh:30-53`（確認済み）
- パターン: `lefthook.yaml` の pre-commit コマンド形式 → `lefthook.yaml:6-45`（確認済み: glob + run パターン）
- パターン: `justfile` の lint/check コマンド形式 → `justfile:266-298`（確認済み）

### 操作パス: 該当なし（操作パスが存在しない）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `just lint-terraform` で fmt チェックと validate が実行される
- [ ] `just check` で terraform lint が含まれる
- [ ] lefthook pre-commit で `.tf` ファイルの fmt チェックが実行される
- [ ] terraform 未インストール時にスキップされる

### 実装内容
1. `justfile` に `lint-terraform` レシピを追加
   - `terraform fmt -check -recursive infra/terraform/`
   - 各環境ディレクトリで `terraform validate`（init 済みの場合のみ）
   - `tflint`（インストール済みの場合のみ）
2. `justfile` の `fmt` に `fmt-terraform` を追加
3. `justfile` の `lint` に `lint-terraform` を追加
4. `scripts/check/parallel.sh` の Non-Rust レーンに `just lint-terraform` を追加
5. `lefthook.yaml` の pre-commit に `terraform-fmt-check` を追加

## Phase 3: CI 統合

### 確認事項
- パターン: `ci.yaml` のジョブ構造 → `.github/workflows/ci.yaml`（確認済み: changes → 条件付きジョブ → ci-success）
- パターン: `changes` ジョブのパスフィルター形式 → `ci.yaml:41-58`（確認済み）
- ライブラリ: `hashicorp/setup-terraform` アクションの使い方 → GitHub Marketplace で確認

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | `.tf` ファイルを変更した PR で terraform ジョブが実行される | 正常系 | CI（手動検証） |
| 2 | `.tf` ファイル以外の変更で terraform ジョブがスキップされる | 正常系 | CI（手動検証） |

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] CI の terraform ジョブが `.tf` 変更時に実行される（この PR 自体で検証）
- [ ] `terraform fmt -check`、`terraform validate`、`tflint` が CI で成功する
- [ ] `ci-success` ジョブが terraform ジョブを含む

### 実装内容
1. `ci.yaml` の `changes` ジョブに `terraform` パスフィルターを追加
2. `terraform` ジョブを追加（fmt, validate, tflint）
3. `ci-success` ジョブに `terraform` を追加

## Phase 4: ドキュメント

### 確認事項: なし（既知のパターンのみ）

### 操作パス: 該当なし（ドキュメントのみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 実装内容
1. `docs/80_ナレッジベース/devtools/Terraform.md` を作成
2. `.claude/rules/terraform.md` の手動検証の記述を更新

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `terraform validate` が init 必要だがローカルでは init されていない可能性 | 不完全なパス | ローカルでは `.terraform` 存在チェックで条件付き実行。CI では `-backend=false` で init |
| 2回目 | prod/stg が空ディレクトリで validate 対象外 | 不完全なパス | `.tf` ファイル存在チェックで環境を動的検出 |
| 3回目 | terraform 未インストール時のローカル品質ゲートの挙動 | 不完全なパス | `which terraform` で存在確認、なければスキップ（警告） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Issue のスコープ（ツールチェーン、ローカル品質ゲート、CI、ドキュメント）すべてが Phase 1-4 に含まれている |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各 Phase の実装内容が具体的。条件付き実行の基準も明示 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | validate の実行方式、tflint 設定の配置、条件付き実行の基準を明示 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: fmt, validate, tflint の統合。対象外: terraform plan/apply の自動化（別 Issue） |
| 5 | 技術的前提 | 前提が考慮されている | OK | validate の init 必要性、`-backend=false` の挙動、tflint の `--init` を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | `.claude/rules/terraform.md` の記載（#1043 で対応予定）と整合 |
