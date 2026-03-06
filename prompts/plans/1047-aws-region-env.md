## コンテキスト

### 目的
- Issue: #1047
- Want: AWS SDK クライアントのリージョン設定をハードコードから環境変数に移行し、設定の一元管理とマルチリージョン展開への備えを実現する
- 完了基準:
  - 3 箇所のハードコードされた `ap-northeast-1` が削除されている
  - `AWS_REGION` 環境変数でリージョンが制御可能になっている
  - ローカル開発・API テスト・本番環境すべてで動作する

### ブランチ / PR
- ブランチ: `feature/1047-aws-region-env`
- PR: #1059（Draft）

### As-Is（探索結果の要約）
- 3 箇所でハードコード:
  - `backend/crates/infra/src/notification/ses.rs:21` — `create_ses_client()`
  - `backend/crates/infra/src/s3.rs:200` — `create_client(endpoint)`
  - `backend/crates/infra/src/dynamodb.rs:62` — `create_client(endpoint)`
- すべて同一パターン: `.region(aws_config::Region::new("ap-northeast-1"))`
- `AWS_REGION` は env ファイルに未設定
- env ファイル構成:
  - `backend/.env.template` — 手動テンプレート（開発用）
  - `backend/.env.api-test.template` — 手動テンプレート（API テスト用）
  - `scripts/env/generate.sh` — worktree 用自動生成（`.env` と `.env.api-test` を生成）
- `aws-config` のデフォルトプロバイダチェーン: `AWS_REGION` 環境変数 → AWS config file → EC2/ECS メタデータ

### 進捗
- [x] Phase 1: `.region()` ハードコード削除 + env ファイル更新

## 仕様整理

### スコープ
- 対象: ses.rs, s3.rs, dynamodb.rs の `.region()` 呼び出し、env テンプレート 2 件、generate.sh
- 対象外: 本番環境のインフラ設定（ECS タスク定義の環境変数追加は別途）

### 操作パス

操作パス: 該当なし（内部リファクタリング。ユーザー操作の変更なし）

## 設計

### 設計判断

| # | 判断 | 選択肢 | 選定理由 | 状態 |
|---|------|--------|---------|------|
| 1 | `.region()` を削除してプロバイダチェーンに委譲 | A: 削除 / B: 環境変数を読んで `.region()` に渡す | A が最もシンプル。`aws-config` のデフォルト挙動を活用する。B は二重管理になる | 確定 |

### Phase 1: `.region()` ハードコード削除 + env ファイル更新

#### 確認事項
- 型: `aws_config::defaults()` のビルダーで `.region()` を呼ばない場合の挙動 → プロバイダチェーンでフォールバック
- パターン: 3 ファイルとも同一パターン（`.region(aws_config::Region::new("ap-northeast-1"))` 行を削除）
- ライブラリ: `aws-config` のリージョン解決は環境変数 `AWS_REGION` / `AWS_DEFAULT_REGION` を参照（公式ドキュメント確認済み）

#### テストリスト

ユニットテスト: 該当なし（既存の Send+Sync テストのみ。リージョン解決は SDK 内部の責務）
ハンドラテスト: 該当なし
API テスト: 既存の S3/DynamoDB 統合テストが `AWS_REGION` 環境変数で動作することを確認
E2E テスト: 該当なし

## ブラッシュアップ

### ギャップ発見の観点 進行状態

| 観点 | 状態 | メモ |
|------|------|------|
| 未定義 | 完了 | env ファイルに `AWS_REGION` 未設定を発見 → 追加で対応 |
| 曖昧 | 完了 | なし |
| 競合・エッジケース | 完了 | DynamoDB Local はリージョン不問だが SDK が署名に必要 → env 設定で解決 |
| 不完全なパス | 完了 | ローカル/API テスト/本番の全パスを確認 |
| アーキテクチャ不整合 | 完了 | なし |
| 既存手段の見落とし | 完了 | `aws-config` のデフォルトプロバイダチェーンが既存手段 |

### ループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | env ファイルに `AWS_REGION` 未設定 → `.region()` 削除だけでは動作しない | 未定義 | テンプレート + generate.sh に `AWS_REGION` 追加を計画に含めた |

### 未解決の問い
なし

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 3 Rust ファイル + 2 テンプレート + 1 generate.sh |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 変更内容が行レベルで確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | プロバイダチェーン委譲の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 本番インフラ設定は対象外と明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `aws-config` のリージョン解決挙動を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 通知機能設計書・ドキュメント管理設計書に `AWS_REGION=ap-northeast-1` の記載あり（整合） |
