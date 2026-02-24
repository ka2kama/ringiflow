# #835 improvement-records.sh を rust-script に移行

## 概要

改善記録のバリデーションスクリプト `scripts/check/improvement-records.sh`（133行）を rust-script に移行した。先行事例（`instrumentation.rs`、#834）のパターンに従い、enum + `FromStr` による型安全なバリデーションに置き換えた。

## 実施内容

- `scripts/check/improvement-records.rs` を新規作成
  - enum 定義: `Category`（6種）、`FailureType`（3種）、`Nature`（3種）
  - `FromStr` 実装で有効値の検証とエラーメッセージ生成を一体化
  - `extract_value` 関数: `str::strip_prefix` + `str::find` による値抽出（regex 不採用）
  - `validate_file` 関数: ファイル内容を受け取る純粋関数としてテスタビリティを確保
  - `glob` クレートでファイル探索（`process/improvements/????-??/*.md`）
  - ユニットテスト 19 件を `#[cfg(test)]` で同梱
- justfile の `lint-improvements` タスクを `rust-script` 呼び出しに更新
- 旧 `improvement-records.sh` を削除

## 判断ログ

- 計画段階で `regex` クレート不採用を決定（KISS: 文字列操作で十分）
- `git ls-files`（instrumentation.rs のアプローチ）ではなく `glob` クレートを採用。改善記録ファイルは `.gitignore` を考慮する必要がないため、よりシンプル
- `lint-shell`（ShellCheck）が削除済み `.sh` を `git ls-files --cached` で検出する問題を発見し、`git add` でインデックスを更新して解決

## 成果物

コミット:
- `a5a3c6f` #835 Migrate improvement-records.sh to rust-script

ファイル:
- 新規: `scripts/check/improvement-records.rs`
- 削除: `scripts/check/improvement-records.sh`
- 変更: `justfile`
- PR: #853（Draft）
