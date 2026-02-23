# sync-epic.sh の rust-script 移行

関連: #837, PR #861

## 概要

`scripts/issue/sync-epic.sh`（79行）を `scripts/issue/sync-epic.rs` に移行した。perl/grep -P/sed 依存を排除し、Rust の `regex` クレートで同等の機能を実現。20 個のユニットテストを追加。

## 判断ログ

### I/O 境界の分離方式

trait 抽象化（`GhClient` trait + テスト用モック）ではなく、純粋関数の抽出を選択。

- `extract_epic_number`, `check_already_updated`, `check_exists_unchecked`, `update_checkbox` の4関数は `&str` → 値 の変換で副作用ゼロ
- `gh` CLI 呼び出しは `run()` 内のオーケストレーションに留め、テスト対象外
- 既存の移行済みスクリプト（instrumentation.rs, impl-docs.rs）と同じパターン

### 正規表現の設計

- `\s` は Rust regex でもデフォルトで `\n` を含むため、perl の `-0777` 相当のマルチラインマッチがフラグなしで可能
- `(?m)` はチェックボックスパターン（`^` で行頭マッチ）のみに使用
- 部分一致防止: `(?:\D|$)` で `#NNN` の後に数字が続かないことを保証

## 成果物

- `scripts/issue/sync-epic.rs`: 新規（340行、うちテスト150行）
- `justfile`: sync-epic レシピを rust-script 呼び出しに変更
- `scripts/issue/sync-epic.sh`: 削除
