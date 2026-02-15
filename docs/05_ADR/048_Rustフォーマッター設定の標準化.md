# ADR-048: Rust フォーマッター設定の標準化

- ステータス: 承認
- 決定日: 2026-02-15
- 決定者: ka2kama + Claude Code

## 文脈

当初の `.rustfmt.toml` は `tab_spaces = 3` を使用しており、Rust 公式デフォルト（4スペース）から逸脱していた。また、unstable features を使用していたが、その選択理由が文書化されていなかった。

## 決定

以下の設定で Rust フォーマッターを標準化する:

1. **`tab_spaces = 4`**: Rust 公式デフォルトに準拠
2. **unstable features を維持**: import 自動整理などの価値を優先
3. **rustfmt-nightly を使用**: unstable features を有効化

## 理由

### `tab_spaces = 4`

- Rust 公式デフォルトとの一致
- LLM が生成するコードとの整合性
- 科学的根拠（Miara et al., 1983）: 4スペースが最適な可読性

### unstable features の維持

以下の機能は手動維持が困難で、自動化の価値が高い:

- `group_imports`: import を標準ライブラリ・外部・自クレートで自動グループ化
- `imports_granularity`: import をクレート単位で整理
- `imports_layout`: 長い import リストを垂直展開
- `format_code_in_doc_comments`: ドキュメント内のコード例もフォーマット
- `reorder_impl_items`: impl ブロック内の一貫した並び順
- `struct_field_align_threshold`: 構造体フィールドの整列

### rustfmt-nightly の使用

- Rust 本体は stable を維持
- rustfmt のみ nightly 版を使用（`cargo +nightly fmt`）
- justfile で既に設定済み、CI でも対応済み

## 代替案

### A. stable のみ（unstable features 削除）

- メリット: 安定性、警告なし
- デメリット: import 整理が手動、フォーマット機能制限
- 却下理由: 手動維持のコストが高い

### B. nightly ツールチェーン全体を導入

- メリット: rustfmt 以外の nightly 機能も利用可能
- デメリット: 不要なコンポーネント（rustdoc など）も含まれる
- 却下理由: 最小限のインストールで十分

## 影響

- 開発者: rustfmt-nightly のインストールが必要（開発環境構築手順書に記載）
- CI: 既に対応済み
- git blame: `.git-blame-ignore-revs` でリフォーマットコミットを除外

## 参照

- Issue: #541
- コミット: 9457099 (リフォーマット), 53989ad (.git-blame-ignore-revs)
