# Issue #606: cargo-watch 実行中の API テスト起動タイムアウト

## Context

`just dev-all`（cargo-watch）実行中に `just test-api` / `just check-all` を実行すると、Cargo パッケージキャッシュのロック競合によりサービスビルドがブロックされ、起動タイムアウトで失敗する。

根本原因: Cargo は workspace 単位でパッケージキャッシュと artifact directory のロックを排他制御する。cargo-watch が `cargo run` を実行中は、別プロセスの `cargo build` / `cargo run` がロック待ちになる。

## 対策の選定

| 対策案 | 判定 | 理由 |
|--------|------|------|
| cargo-watch 検知 + エラー終了 | **採用** | KISS。根本原因（並行 cargo 実行）を排除する方向。即座にフィードバック |
| タイムアウト延長 | 不採用 | 症状の緩和のみ。ロック待ちが無限の可能性がありタイムアウト値に正解がない |
| `CARGO_TARGET_DIR` 分離 | 不採用 | sccache 導入時に非推奨化済み。ディスク使用量倍増、ビルド重複 |

## 対象・対象外

対象:
- `scripts/run-api-tests.sh` — `cargo build` + ヘルスチェック 30s タイムアウトで失敗
- `scripts/run-e2e-tests.sh` — 同じパターン、ヘルスチェック 60s タイムアウトで失敗
- `scripts/check-parallel.sh` — `cargo clippy` / `cargo test` がロック待ちで非常に遅い

対象外:
- justfile のレシピ変更（スクリプト内で完結させる）
- タイムアウト値の変更（検知で fail-fast するため不要）

## 設計

### 検知ロジック

```bash
# cargo-watch 検知: 実行中だとパッケージキャッシュのロック競合が発生するため
if pgrep -f "cargo-watch" > /dev/null 2>&1; then
    echo "エラー: cargo-watch が実行中のため、Cargo パッケージキャッシュのロック競合が発生します。" >&2
    echo "開発サーバーを停止してから再実行してください（just dev-down または mprocs を終了）。" >&2
    exit 1
fi
```

`pgrep -f "cargo-watch"`: `cargo watch` サブコマンドは内部的に `cargo-watch` バイナリを起動するため、プロセスの実行ファイル名で検出する。

### 配置位置

各スクリプトの `cargo build` / cargo コマンド実行前（冒頭の変数定義直後）に配置する。

重複は 3 箇所だが、各チェックは 5 行程度であり、スクリプトの自己完結性を優先する（共有ライブラリの抽出は過度な抽象化）。

## Phase 1: cargo-watch 検知チェックの追加

### 確認事項

- [x] パターン: 既存スクリプトのエラーメッセージ形式 → `run-api-tests.sh` L57 `echo "エラー: ..." >&2; exit 1`
- [x] ライブラリ: `pgrep -f` の動作 → Linux 標準。`-f` はフルコマンドラインでマッチ

### 変更ファイル

1. `scripts/run-api-tests.sh` — `cd "$PROJECT_ROOT/backend"` (L27) の直後、`.env.api-test` 読み込み (L30) の前に検知チェックを追加
2. `scripts/run-e2e-tests.sh` — 同じ位置（`cd "$PROJECT_ROOT/backend"` L31 の直後）に検知チェックを追加
3. `scripts/check-parallel.sh` — `skip_db` 判定 (L13) の直後に検知チェックを追加

### テストリスト

ユニットテスト: 該当なし（bash スクリプトの変更）

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

手動検証:
- [ ] cargo-watch 実行中に `just test-api` → エラーメッセージが表示され即座に終了
- [ ] cargo-watch 停止後に `just test-api` → 通常通りテストが実行される

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `run-e2e-tests.sh` も同じパターン（`cargo build` + タイムアウト）で影響を受ける | 網羅性 | 対象スクリプトに `run-e2e-tests.sh` を追加 |
| 2回目 | `check-parallel.sh` は cargo をビルドしないが lint/test でロック競合する | 網羅性 | 対象スクリプトに `check-parallel.sh` を追加（遅延防止） |
| 3回目 | 検知漏れなし | — | ギャップなし |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | cargo を使用する 3 スクリプトすべてを対象に含めた |
| 2 | 曖昧さ排除 | OK | 検知ロジック、配置位置、エラーメッセージをコードレベルで明示 |
| 3 | 設計判断の完結性 | OK | 3 対策案の比較と選定理由を記載 |
| 4 | スコープ境界 | OK | 対象・対象外を明記（justfile 変更は対象外） |
| 5 | 技術的前提 | OK | `pgrep -f` は Linux 標準ユーティリティ、`cargo-watch` のプロセス名を確認済み |
| 6 | 既存ドキュメント整合 | OK | `CARGO_TARGET_DIR` 非推奨化（sccache 導入 ADR）と整合 |
