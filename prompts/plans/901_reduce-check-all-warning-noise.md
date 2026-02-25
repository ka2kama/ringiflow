# 実装計画: #901 check-all の警告ノイズを削減する

## Context

`just check-all` のすべての warning 系チェックが exit 0 で通過しており、警告が慢性的に蓄積している（割れ窓効果）。ラチェット方式のベースラインを導入し、警告の悪化を構造的に防止する。ADR-042 にも「閾値を設定して CI ブロックに移行できる」と記載されており、本 Issue はその計画された移行を実行する。

## 設計判断

### 1. ベースライン管理方式: `.config/baselines.env`

shell-sourceable な key=value ファイルに全ベースラインを集約する。

選択肢:
- A. `.config/baselines.env`（sourceable shell file）← 採用
- B. `.config/baselines.toml`（TOML ファイル + 読み取りヘルパー）
- C. justfile 変数にハードコード
- D. 各スクリプト内にハードコード

理由:
- 完了基準「設定ファイルで管理、diff で変化が見える」を満たす
- bash スクリプトから `source` で直接読み取り可能（パーサー不要）
- 一覧性がある（散在しない）
- justfile の shebang レシピから `source` 可能

### 2. jscpd 閾値: `--threshold` オプション直接使用

`--exitCode 0` を削除し、`--threshold` で重複率上限を指定する。

- jscpd v4.0.8 の CLI help で `--threshold [number]` の存在を確認済み
- ADR-042 のバグにより `.jscpd.json` は使用不可。CLI 引数のみで運用
- 現在のインライン justfile レシピを `scripts/check/check-duplicates.sh` に移動（baselines.env の source が必要なため）

### 3. cargo deny: `deny` + `skip` 方式

`multiple-versions = "warn"` → `"deny"` に変更し、既知 duplicate を `skip` で明示許容。

理由: `warn` + `skip` だと新しい duplicate も警告止まり（exit 0）。`deny` + `skip` なら skip に含まれない新 duplicate で CI が失敗する。ラチェットの本質と一致。

### 4. improvement-records.rs: CLI 引数でベースライン受け取り

justfile の shebang レシピで baselines.env を source し、値を CLI 引数として渡す。

理由: rust-script に TOML パーサーを追加すると初回コンパイルが重くなる。bash wrapper パターンで他スクリプトと統一。

## 対象外

- Rust コード重複の根本解消（#902 で対応済み、CLOSED）
- 改善記録の既存 70 件の「問題の性質」追記（#900 で対応、OPEN）
- fn-size.sh / doc-links.sh / stale-annotations.sh 等（既に exit 1 で fail する仕組みがある）

---

## Phase 1: ベースラインインフラ + jscpd 閾値

### 対象ファイル

| ファイル | 変更 |
|---------|------|
| `.config/baselines.env` | 新規作成 |
| `scripts/check/check-duplicates.sh` | 新規作成（justfile インラインから移動） |
| `justfile` | `check-duplicates` レシピを新スクリプト呼び出しに変更 |

### 変更内容

1. `.config/baselines.env` を作成:
   ```bash
   # ベースライン設定（ラチェット方式）
   # 値は改善に伴い下げていく。ベースライン超過で CI が失敗する。

   # jscpd: 重複率の上限（%）
   JSCPD_RUST_THRESHOLD=13
   JSCPD_ELM_THRESHOLD=3

   # file-size: 500行超ファイル数の上限
   FILE_SIZE_MAX_COUNT=30

   # improvement-records: 「問題の性質」未記載の上限
   # #900 完了後に 0 に更新する
   IMPROVEMENT_RECORDS_MAX_MISSING_NATURE=70
   ```
   注: 閾値の具体値は実装時に `just check-duplicates` で実測し、現状値を少し上回る値に設定する。

2. `scripts/check/check-duplicates.sh` を作成:
   - baselines.env を source
   - `--exitCode 0` を削除し、`--threshold "$JSCPD_RUST_THRESHOLD"` / `"$JSCPD_ELM_THRESHOLD"` を使用
   - 既存の jscpd CLI オプション（`--min-lines 10 --min-tokens 50` 等）はそのまま維持

3. justfile `check-duplicates` レシピを `./scripts/check/check-duplicates.sh` 呼び出しに変更

### 確認事項

- ライブラリ: jscpd `--threshold` の振る舞い → CLI help で確認済み。`--exitCode 0` を削除しないと threshold が無視される点に注意
- パターン: 既存の check スクリプト（`file-size.sh` 等）の shebang・構造 → `scripts/check/file-size.sh` を参照
- 技術的前提: ADR-042 の `.jscpd.json` バグ → CLI 引数のみ使用を継続

### 操作パス

該当なし（CI スクリプト変更）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- `just check-duplicates` が exit 0 で通過する（現状値がベースライン以下）
- threshold を意図的に低く設定（例: 1）して exit 1 で失敗することを確認し、正しい値に戻す

---

## Phase 2: file-size.sh ベースライン

### 対象ファイル

| ファイル | 変更 |
|---------|------|
| `scripts/check/file-size.sh` | baselines.env 読み込み + 件数ベースライン比較 + exit code |

### 変更内容

1. スクリプト冒頭で baselines.env を source し、`FILE_SIZE_MAX_COUNT` を読み取る
2. 検出ファイル数 `$found` が `$FILE_SIZE_MAX_COUNT` を超えた場合 exit 1
3. `$found` が `$FILE_SIZE_MAX_COUNT` を下回った場合、ベースライン更新を促すメッセージを出力（exit 0）
4. スクリプト冒頭のコメント「警告のみ（exit 0）」を更新

### 確認事項

- パターン: `file-size.sh` の構造 → 確認済み（L1-37）
- 型: `$found` は整数、bash の `-gt` 比較で十分

### 操作パス

該当なし（CI スクリプト変更）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証: `just check-file-size` が exit 0 で通過する

---

## Phase 3: improvement-records.rs ベースライン

### 対象ファイル

| ファイル | 変更 |
|---------|------|
| `scripts/check/improvement-records.rs` | CLI 引数 `--max-missing-nature` の受け取り + ベースライン比較 |
| `justfile` | `lint-improvements` レシピを shebang 化し、baselines.env から引数を渡す |

### 変更内容

1. `improvement-records.rs`:
   - `run()` 関数に `max_missing_nature: Option<usize>` パラメータを追加
   - `main()` で `std::env::args()` から `--max-missing-nature <N>` を解析して `run()` に渡す
   - `all_warnings.len()` が `max_missing_nature` を超えた場合、exit 1（現在値とベースライン値をエラーメッセージに表示）
   - ベースラインを下回った場合、更新を促すメッセージを表示
   - `--max-missing-nature` が未指定の場合は従来通り警告のみ（exit 0）— 後方互換

2. justfile `lint-improvements` レシピ:
   ```
   lint-improvements:
       #!/usr/bin/env bash
       set -euo pipefail
       source .config/baselines.env
       rust-script ./scripts/check/improvement-records.rs \
           --max-missing-nature "$IMPROVEMENT_RECORDS_MAX_MISSING_NATURE"
   ```

### 確認事項

- 型: `run()` の戻り値は `i32`、`main()` は `std::process::exit(run())` → 確認済み（L212-276）
- パターン: CLI 引数解析 — 外部クレート不要。`std::env::args()` で十分（引数は 1 つだけ）
- ライブラリ: `pretty_assertions` は既にテスト依存に含まれる → 確認済み（L5）

### 操作パス

該当なし（CI スクリプト変更）

### テストリスト

ユニットテスト:
- [ ] `run_with_baseline()`: 警告件数がベースライン以下のとき exit 0
- [ ] `run_with_baseline()`: 警告件数がベースラインを超えたとき exit 1
- [ ] `run_with_baseline()`: ベースラインを下回ったとき更新メッセージを出力
- [ ] `run_with_baseline()`: ベースライン未指定のとき従来通り exit 0（後方互換）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

---

## Phase 4: cargo deny skip + ELIFECYCLE 143 抑制

### 対象ファイル

| ファイル | 変更 |
|---------|------|
| `backend/deny.toml` | `multiple-versions = "deny"` + `[[bans.skip]]` 追加 |
| `scripts/test/run-e2e.sh` | trap のクリーンアップ改善 |

### 変更内容

#### cargo deny

1. `multiple-versions = "warn"` → `"deny"` に変更
2. `[bans]` セクションの `wildcards = "allow"` の後に `skip` リストを追加:

   対象クレート（旧バージョン側を skip）:
   ```toml
   [[bans.skip]]
   crate = "getrandom@0.2"
   reason = "sqlx / argon2 が旧 rand エコシステムに依存"

   [[bans.skip]]
   crate = "h2@0.3"
   reason = "AWS SDK が hyper 0.14 経由で依存"
   # ... 以下、cargo deny check bans の出力に基づく全クレート
   ```

   実装時に `cargo deny check bans` を実行し、各 warning のクレート名とバージョンを正確に取得する。

#### ELIFECYCLE 143

1. `run-e2e.sh` L28 の trap を改善:
   ```bash
   # 変更前
   trap 'kill $(jobs -p) 2>/dev/null' EXIT

   # 変更後
   cleanup() {
       local pids
       pids=$(jobs -p 2>/dev/null) || true
       if [ -n "$pids" ]; then
           kill $pids 2>/dev/null || true
           wait $pids 2>/dev/null || true
       fi
   } 2>/dev/null
   trap cleanup EXIT
   ```
   `wait` でプロセス終了を待ち、関数全体の stderr を抑制して ELIFECYCLE ノイズを除去。

### 確認事項

- ライブラリ: cargo-deny の `[[bans.skip]]` 書式 → `{ crate = "name@version", reason = "..." }` 形式
- パターン: `deny.toml` の既存構造 → 確認済み（L56-60）
- 技術的前提: `wait` がバックグラウンドプロセスの stderr を抑制するか → 実装時に検証。`} 2>/dev/null` で関数全体の stderr 抑制がフォールバック

### 操作パス

該当なし（設定・スクリプト変更）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- `cd backend && cargo deny check bans` が警告ゼロで通過
- `just test-e2e` 実行後に ELIFECYCLE 143 が出力されない

---

## Phase 5: 統合検証

### 検証項目

1. `just check-all` が全てパスする
2. `just check-all` の出力に未対処の警告がない（目視確認）
3. 各ベースラインの機能確認:
   - jscpd: threshold を一時的に 1 に変更 → exit 1 を確認 → 元に戻す
   - file-size: `FILE_SIZE_MAX_COUNT=1` で exit 1 を確認
   - improvement-records: `--max-missing-nature 0` で exit 1 を確認
   - cargo deny: skip から 1 つ削除 → exit 1 を確認 → 元に戻す
4. parallel.sh の exit code 伝播が壊れていないこと

---

## ファイル変更サマリー

| ファイル | 変更種別 | Phase |
|---------|---------|-------|
| `.config/baselines.env` | 新規作成 | 1 |
| `scripts/check/check-duplicates.sh` | 新規作成 | 1 |
| `justfile` | 修正（check-duplicates, lint-improvements） | 1, 3 |
| `scripts/check/file-size.sh` | 修正 | 2 |
| `scripts/check/improvement-records.rs` | 修正 | 3 |
| `backend/deny.toml` | 修正 | 4 |
| `scripts/test/run-e2e.sh` | 修正 | 4 |

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | jscpd `--threshold` の存在が未確認 | 技術的前提 | CLI help で確認。v4.0.8 で利用可能 |
| 2回目 | baselines の管理方式が未決定（TOML vs shell vs justfile） | 曖昧 | `.config/baselines.env`（sourceable shell file）を選択。理由を記載 |
| 3回目 | `--exitCode 0` と `--threshold` の相互作用が不明 | 未定義 | `--exitCode 0` は threshold を上書きして常に exit 0 にする。両方削除+追加が必要 |
| 4回目 | improvement-records.rs の TOML 読み取り方法が未決定 | 競合・エッジケース | CLI 引数方式を選択（rust-script のコンパイル負荷回避） |
| 5回目 | cargo deny `warn` + `skip` vs `deny` + `skip` の選択 | シンプルさ | `deny` + `skip` を選択。ラチェットの本質（新 duplicate で fail）と一致 |
| 6回目 | ADR-042 の `.jscpd.json` バグとの整合 | 既存ドキュメント整合 | CLI 引数のみ使用を継続。check-duplicates.sh でカプセル化 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 完了基準 8 項目すべてに対応する Phase がある |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | jscpd threshold 確認済み、cargo deny skip 書式確認済み。ベースライン具体値は実測で確定（設計は確定） |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 4 つの設計判断を理由付きで記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: 6 つの警告カテゴリ。対象外: #900, #902, 既に exit 1 のスクリプト群 |
| 5 | 技術的前提 | 前提が考慮されている | OK | jscpd v4.0.8 threshold、cargo-deny skip 書式、ADR-042 バグ |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-042「閾値を設定して CI ブロックに移行できる」と一致 |
