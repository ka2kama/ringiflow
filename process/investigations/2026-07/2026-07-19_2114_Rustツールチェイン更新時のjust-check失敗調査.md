# Rust ツールチェイン 1.97.1 更新時の just check 失敗調査

関連: Rust ツールチェイン 1.94.0 → 1.97.1 更新の PR

## 症状

`rust-toolchain.toml` を 1.97.1 に更新した直後の `just check` が 3 種類の失敗を示した。

- エラーメッセージ 1: `error[E0463]: can't find crate for std`（多数の依存クレートのコンパイルで発生。リンカが `~/.rustup/toolchains/1.97.1-.../lib/libstd-*.rlib` を発見できない）
- エラーメッセージ 2: `[ERR_PNPM_ABORTED_REMOVE_MODULES_DIR_NO_TTY]`（`lint-openapi` レシピの `pnpm exec redocly lint` が誘発する `pnpm install` で発生）
- エラーメッセージ 3: `DynamoDb("テーブル 'test_audit_logs' の確認に失敗: dispatch failure")`（`audit_log_repository_test` の統合テスト 11 件がテーブルセットアップで panic）
- 発生タイミング: ツールチェイン更新後の初回 `just check`（1 は初回のみ、2・3 は再現性あり）
- 影響範囲: ローカルの品質ゲートのみ。CI には影響しない（後述）

## 環境

| 項目 | 値 |
|------|-----|
| ブランチ | `chore/update-rust-1.97` |
| 実行環境 | ローカル（Linux, rustup 1.29.0, pnpm 11.14.0） |
| 関連コミット | `6463242`（#1047 AWS リージョンの環境変数化） |

## 仮説と検証

| # | 仮説 | 予測（正しければ何が観察されるか） | 検証手段 | 結果 | 判定 |
|---|------|--------------------------------|---------|------|------|
| 1 | 1.97.1 の `rust-std` コンポーネントが未インストール | `rustup component list` に installed がない | `rustup component list --toolchain 1.97.1` + rlib の実在確認 | installed かつ「見つからない」とされた rlib が実在（62 ファイル） | 棄却 |
| 2 | rustup の自動インストール（アーカイブ展開中）と `just check` の並列コンパイルが競合し、展開完了前の std を参照した | 再実行すれば成功する | `just check` 再実行 | fmt / clippy / ユニットテスト / doctest すべて通過 | 支持 |
| 3 | pnpm の失敗は TTY なしで modules purge の確認プロンプトを出せず中断したもの | `CI=true` で purge が許可され進行する | `CI=true just check` | purge は進行したが、新たに `ERR_PNPM_IGNORED_BUILDS`（core-js, protobufjs）で失敗。pnpm 11 が package.json の `pnpm` フィールドを読まなくなった設定移行の問題で、本 PR の diff（pnpm 関連ファイルを含まない）と独立 | 部分支持 |
| 4 | DynamoDB 統合テストの失敗は dynamodb-local コンテナ未起動 | `docker ps` に該当コンテナがない | `docker ps` | `ringiflow-dynamodb-1` は Up/healthy（ポート 18000）、`curl` でも疎通確認（400 応答は正常） | 棄却 |
| 5 | `AWS_REGION` 未設定により AWS SDK が dispatch failure を返す | `AWS_REGION` を明示すればテストが通る | `AWS_REGION=ap-northeast-1 cargo test ...` | 単体・統合テスト全件（同レシピ）通過 | 支持 |

## 切り分け（Isolate)

| 確認レイヤー | 確認手段 | 結果 |
|------------|---------|------|
| DynamoDB Local コンテナ | `docker ps` / `curl localhost:18000` | 正常 |
| AWS SDK クライアント設定 | `dynamodb.rs` の `create_client` を Read（クレデンシャルは固定、リージョンは環境変数任せ） | リージョン欠落の可能性 |
| CI の環境変数 | `ci.yaml` の rust-integration ジョブ | `AWS_REGION: ap-northeast-1` を明示設定（238 行目）→ CI では発生しない |

## 根本原因

3 つの失敗はそれぞれ独立した原因を持ち、いずれもツールチェイン更新の内容自体（コンパイラ・clippy の非互換）とは無関係だった。

1. std 欠落エラー: rustup が新ツールチェインを自動インストールしている最中に並列レーンのコンパイルが走り、展開途中のツールチェインを参照した一過性の競合。
2. pnpm エラー: pnpm 11 が package.json の `pnpm` フィールド（`neverBuiltDependencies` 等）を読まなくなった設定移行の過渡期の問題。ローカル環境固有で、本 PR とは独立。
3. dispatch failure: #1047（`6463242`）で AWS リージョンが環境変数化されて以降、ローカルの `.env` に `AWS_REGION` が未設定だと DynamoDB 統合テストが失敗する。CI はジョブ環境変数で設定済みのため顕在化しなかった。

## 修正と検証

修正内容: ツールチェイン更新そのものに修正は不要。検証は以下の組み合わせで実施。

- Rust レーン: `just check` 再実行で fmt / clippy / ユニット / doctest 通過（新 lint の指摘ゼロ）
- 統合テスト: `AWS_REGION=ap-northeast-1 just test-rust-integration` で全件通過
- pnpm / lint-openapi: ローカル環境の問題としてスコープ外（CI で検証）

検証結果: 1.97.1 でバックエンド全テスト通過。

## 診断パターン（Knowledge）

- 症状「E0463 can't find crate for std が大量発生」が見られたら、まず rustup のツールチェイン自動インストールとの競合を疑い、単純に再実行する
- 症状「AWS SDK の dispatch failure」が見られたら、エンドポイント疎通の前に `AWS_REGION` の設定を確認する（コンテナ起動確認より先にコストの低い検証）
- ツールチェイン更新の PR で `just check` が失敗しても、失敗箇所が更新内容と因果を持つか（コンパイラ非互換か、環境要因か）を切り分けてから対処する

## 関連ドキュメント

- セッションログ: [2026-07-19_2114_Rustツールチェイン1.97.1更新.md](../../../prompts/runs/2026-07/2026-07-19_2114_Rustツールチェイン1.97.1更新.md)
- 関連 Issue: #1096（AWS_REGION 未設定による dispatch failure の前例）、#1098（環境差異の設計課題)
