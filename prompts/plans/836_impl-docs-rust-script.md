# #836 impl-docs.sh を rust-script に移行

## Context

`scripts/check/impl-docs.sh`（83行）は実装解説ドキュメント（`docs/90_実装解説/`）のファイル命名規則をチェックするスクリプト。Bash 4+ の連想配列（`declare -A`）と `BASH_REMATCH` による正規表現キャプチャに依存しており、ADR-015 の移行基準に該当する。

先行移行（#834 instrumentation.rs, #835 improvement-records.rs）でパターンが確立されているため、そのパターンに従って移行する。

## 対象

- 新規: `scripts/check/impl-docs.rs`
- 変更: `justfile`（`check-impl-docs` タスク）
- 削除: `scripts/check/impl-docs.sh`

## 対象外

- 命名規則自体の変更（既存ルールをそのまま移植）
- 他のチェックスクリプトの移行

## 設計判断

### regex クレートの採用

improvement-records.rs は文字列操作で十分だったため regex 不採用だったが、impl-docs.sh は以下の理由で `regex` クレートを採用する:

- ファイル名パターン `^[0-9]{2}_.+_(機能解説|コード解説)\.md$` は regex が自然
- トピック抽出にキャプチャグループが必要（`^[0-9]{2}_(.+)_(機能解説|コード解説)\.md$`）
- ディレクトリ名パターン（`^[0-9]+_`, `^PR[0-9]+_`）も regex で簡潔に書ける

### glob クレートでファイル探索

improvement-records.rs と同じパターン。`docs/90_実装解説/` は `.gitignore` を考慮する必要がないため `glob` が適切。

### 構造: validate_dir 純粋関数

improvement-records.rs の `validate_file` パターンに倣い、ディレクトリ単位のバリデーション関数 `validate_dir` を純粋関数として実装する。テスタビリティのため、ファイルシステムアクセスを `run()` に集約し、`validate_dir` はディレクトリ名とファイル名のリストを受け取る。

## Phase 1: rust-script 実装

### 確認事項

- パターン: `scripts/check/improvement-records.rs` の構造（glob + validate + テスト同梱） → 既読
- パターン: `scripts/check/impl-docs.sh` のチェックロジック → 既読
- ライブラリ: `regex` クレートの `Regex::new`, `Regex::is_match`, `Regex::captures` → Grep 既存使用なし、docs.rs で確認
- ライブラリ: `glob` クレートの使用 → `improvement-records.rs` で確認済み

### チェックロジックの移植

元のシェルスクリプトの3つのチェックを移植する:

1. ディレクトリ名チェック
   - 旧形式（`^[0-9]+_`）を拒否
   - PR プレフィックスがある場合は `^PR[0-9]+_` を検証
   - feature モード（PR プレフィックスなし）は許容

2. ファイル名チェック
   - `^[0-9]{2}_.+_(機能解説|コード解説)\.md$` に合致すること

3. ペアチェック
   - トピック単位で機能解説とコード解説がペアで存在すること
   - `HashMap<String, HashSet<DocType>>` で追跡（Bash の `declare -A topics_feature` / `topics_code` の置き換え）

### 関数設計

```rust
/// ドキュメントの種類
enum DocType { Feature, Code }

/// ディレクトリ内のファイル情報
struct DirFiles {
    dir_name: String,
    dir_path: String,
    file_names: Vec<String>,
}

/// バリデーション結果
struct ValidationResult {
    errors: Vec<String>,
}

/// ディレクトリ単位のバリデーション（純粋関数）
fn validate_dir(dir: &DirFiles) -> ValidationResult

/// エントリポイント
fn run() -> i32
```

### 出力形式

元のスクリプトと同一:
- 成功: `✅ 実装解説のファイル命名規則に準拠しています`
- 失敗: `⚠️  実装解説の命名規則違反が見つかりました (N 件):` + 詳細

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 正しい命名規則のディレクトリ・ファイルでエラーなし | 正常系 | ユニット |
| 2 | 旧形式の連番ディレクトリ名を検出 | 準正常系 | ユニット |
| 3 | 不正な PR プレフィックスを検出 | 準正常系 | ユニット |
| 4 | 不正なファイル名を検出 | 準正常系 | ユニット |
| 5 | コード解説が欠如しているトピックを検出 | 準正常系 | ユニット |
| 6 | 機能解説が欠如しているトピックを検出 | 準正常系 | ユニット |
| 7 | 実際のディレクトリに対して実行し同じ結果を得る | 正常系 | 手動確認 |

### テストリスト

ユニットテスト:
- [ ] 正常系: PR形式ディレクトリ + 正しいファイルペアでエラーなし
- [ ] 正常系: feature 形式ディレクトリ（PR プレフィックスなし）でエラーなし
- [ ] 異常系: 旧形式の連番ディレクトリ名でエラー
- [ ] 異常系: 不正な PR プレフィックス（PR なしの数字）でエラー
- [ ] 異常系: 不正なファイル名でエラー
- [ ] 異常系: コード解説が欠如しているトピックでエラー
- [ ] 異常系: 機能解説が欠如しているトピックでエラー
- [ ] 異常系: 複数のエラーを同時に検出

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: justfile 更新と旧スクリプト削除

### 確認事項: なし（既知のパターンのみ）

- justfile の `check-impl-docs` タスクを `rust-script` 呼び出しに更新
- `scripts/check/impl-docs.sh` を削除
- `git add` でインデックスを更新（lint-shell が削除済み `.sh` を検出する問題の回避 — improvement-records.rs セッションログの知見）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動確認:
- [ ] `just check-impl-docs` が正常に動作する
- [ ] `just check-all` が通過する

## 検証方法

1. `rust-script ./scripts/check/impl-docs.rs` を実行し、既存の `./scripts/check/impl-docs.sh` と同じ結果になることを確認
2. `just check-impl-docs` が更新後も動作することを確認
3. `just check-all` が全体として通過することを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | validate_dir の入力型が未定義 | 曖昧 | DirFiles 構造体を定義し、ファイルシステムアクセスと分離 |
| 2回目 | feature モード（PR プレフィックスなしディレクトリ）がテストリストに未反映 | 操作パス網羅漏れ | テストケース追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 3つのチェック（ディレクトリ名・ファイル名・ペア）すべて網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 関数シグネチャ、正規表現パターン、出力形式を明記 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | regex 採用、glob 採用、validate_dir 設計を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象・対象外セクションあり |
| 5 | 技術的前提 | 前提が考慮されている | OK | regex クレートの API、glob パターンを確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | docs/90_実装解説/README.md の命名規則と整合 |
