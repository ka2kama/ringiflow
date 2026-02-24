# 知識-実行乖離の構造的対策: lint 自動検証の横展開

Issue: #843

## 概要

知識-実行乖離が 3 期連続でカテゴリ最多（31%→40%→45%）に加速している問題に対し、#735 で成功した lint 自動検証のアプローチを横展開した。OpenAPI ハンドラ登録照合と計画ファイルのテスト層網羅確認の 2 つの lint スクリプトを追加した。

## 実施内容

### Phase 1: OpenAPI ハンドラ登録照合

`scripts/check/openapi-handler-registration.sh` を新規作成した。`#[utoipa::path]` アノテーションが付いたハンドラ関数が `openapi.rs` の `paths()` マクロに登録されているかを検証する。

対応した技術的ポイント:
- `auth/`, `workflow/` のサブディレクトリ構造と単一ファイルモジュールの両方を処理
- `#[utoipa::path]` と `pub async fn` の間に `#[tracing::instrument]` が挟まるケースをフラグ方式で対応
- `mod.rs` は自動的にスキップ（`#[utoipa::path]` を含まないため）

### Phase 2: テスト層網羅確認

`scripts/check/plan-test-layers.sh` を新規作成した。計画ファイルの `#### テストリスト` セクションに 4 つのテスト層（ユニットテスト / ハンドラテスト / API テスト / E2E テスト）が全て明記されているかを検証する。

免除条件:
- `#### テストリスト: 該当なし（理由）` 形式のヘッダー行
- main ブランチでの実行はスキップ（`plan-confirmations.sh` と同じパターン）

### 統合

justfile に `lint-openapi-handlers` と `lint-plan-test-layers` ターゲットを追加し、`parallel.sh` の Non-Rust レーンに組み込んだ。

### health_check の修正

lint 作成中に `health_check` 関数に `#[utoipa::path]` が付いているが `openapi.rs` に未登録であることを発見した。当初はバグとして `openapi.rs` に追加したが、`just check-all` の失敗から「`/health` はインフラ用で OpenAPI 仕様には含めない」という意図的な設計であることが判明した。lint ルール（アノテーションがあるなら登録必須）に従い、`health_check` から `#[utoipa::path]` を削除する方針で対応した。

## 判断ログ

- `health_check` の `#[utoipa::path]` 除去: `openapi.rs` に追加するのではなく、アノテーションを削除する方針を採用。lint ルール「アノテーションがあるなら登録必須、意図的に除外するならアノテーションを削除する」との整合性を優先した
- ShellCheck SC2094 の抑制: `plan-test-layers.sh` で `check_section "$file"` を `done < "$file"` ループ内で呼び出しているが、読み取り専用のため安全。ファイル先頭で `# shellcheck disable=SC2094` を指定

## 成果物

### コミット

- `c20da9f` #843 Add lint scripts for OpenAPI handler registration and plan test layers

### 作成・更新ファイル

| ファイル | 種類 |
|---------|------|
| `scripts/check/openapi-handler-registration.sh` | 新規 |
| `scripts/check/plan-test-layers.sh` | 新規 |
| `justfile` | 更新（lint ターゲット追加） |
| `scripts/check/parallel.sh` | 更新（Non-Rust レーン追加） |
| `backend/apps/bff/src/handler/health.rs` | 更新（`#[utoipa::path]` 削除） |
| `prompts/plans/843_lint-knowledge-execution-gap.md` | 新規（計画ファイル） |
