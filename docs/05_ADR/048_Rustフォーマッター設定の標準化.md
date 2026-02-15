# ADR-048: Rust フォーマッター設定の標準化

- ステータス: 承認
- 決定日: 2026-02-15
- 決定者: ka2kama + Claude Code

## 文脈

当初の `.rustfmt.toml` は以下の問題があった:

1. `tab_spaces = 3` を使用しており、Rust 公式デフォルト（4スペース）から逸脱
2. 設定項目が最小限（7個のみ）で、rustfmt が提供する86個の設定オプションのうちほとんどが未設定
3. unstable features の選択理由が文書化されていない
4. `edition` や `style_edition` が未設定で、rustfmt と cargo fmt の不整合リスクがあった

## 決定

rustfmt のベストプラクティスを起点とした包括的な設定を採用する:

### Stable オプション

1. **`edition = "2024"`**: Cargo.toml との整合性確保（rustfmt のデフォルトは "2015"）
2. **`tab_spaces = 4`**: Rust 公式デフォルトに準拠
3. **`max_width = 100`**: 明示的に記載（デフォルトだが重要な設定）
4. **`hard_tabs = false`**: スペース使用を明示
5. **`newline_style = "Unix"`**: Linux 環境での明示的指定
6. **`use_field_init_shorthand = true`**: より簡潔な記法を採用
7. **`use_try_shorthand = true`**: `?` 演算子の使用を推奨（モダンな Rust）

### Unstable オプション

1. **`style_edition = "2024"`**: rustfmt と cargo fmt の整合性確保（最重要）
2. **Import 関連**: `group_imports`、`imports_granularity`、`imports_layout`
3. **コメント関連**: `wrap_comments`、`normalize_comments`（新規追加）
4. **その他**: `format_code_in_doc_comments`、`reorder_impl_items`、`struct_field_align_threshold`

### rustfmt-nightly の使用

Rust 本体は stable を維持し、rustfmt のみ nightly 版を使用する

## 理由

### ベストプラクティス起点のアプローチ

従来は既存の設定を部分的に修正するアプローチだったが、rustfmt 公式のベストプラクティスを起点とした包括的なレビューに変更した。これにより:

- rustfmt が提供する全86個の設定オプションを検討対象に
- 公式ドキュメントとコミュニティのベストプラクティスに基づく選択
- edition 設定の重要性など、見落としがちな設定を発見

### Edition 設定の重要性

`edition` と `style_edition` を明示的に設定することで:

- rustfmt（デフォルト "2015"）と cargo fmt（Cargo.toml から推論）の不整合を防止
- プロジェクト全体で一貫したフォーマットを保証
- 将来の Rust Edition 移行時の混乱を回避

### Stable オプションの充実

デフォルト値であっても重要な設定は明示的に記載:

- `max_width = 100`: コード幅の基準を明確化
- `hard_tabs = false`: スペース使用を明示
- `use_try_shorthand = true`: モダンな Rust イディオムの採用

### Unstable Features の選択理由

手動維持が困難で、自動化の価値が高い機能を厳選:

- Import 関連: 手動での一貫性維持は現実的でない
- Comment 関連（新規追加）: 長いコメントの自動折り返しとスタイル統一
- その他: ドキュメント品質向上、impl ブロックの並び順統一

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
