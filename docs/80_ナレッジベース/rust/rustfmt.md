# rustfmt

## 概要

Rust の公式コードフォーマッター。一貫したコードスタイルを自動的に維持する。

公式: https://github.com/rust-lang/rustfmt

## このプロジェクトでの使用

### 基本設定

rustfmt 公式のベストプラクティスに基づいた包括的な設定を採用:

**Stable 設定:**
- **`edition = "2024"`**: Cargo.toml と一致（rustfmt と cargo fmt の不整合を防止）
- **`tab_spaces = 4`**: Rust 公式デフォルト
- **`max_width = 100`**: コード幅の基準
- **`use_field_init_shorthand = true`**: 簡潔な記法を推奨
- **`use_try_shorthand = true`**: `?` 演算子を推奨

**Unstable 設定:**
- **`style_edition = "2024"`**: フォーマットスタイルのバージョン統一（最重要）
- **Import 自動整理**: `group_imports`、`imports_granularity`、`imports_layout`
- **その他**: ドキュメント内コードのフォーマット、impl 並び順、構造体整列

### 実行方法

```bash
# 推奨: justfile 経由
just fmt-rust

# 直接実行（nightly 必須）
cargo +nightly fmt
```

**重要**: `cargo fmt`（stable）ではなく、必ず `cargo +nightly fmt` または `just fmt-rust` を使用すること。

### Unstable Features の選択理由

以下の機能を有効化（手動維持が困難で、自動化の価値が高い）:

| カテゴリ | 設定 | 効果 |
|---------|------|------|
| Edition | `style_edition` | フォーマットスタイルのバージョン統一（rustfmt と cargo fmt の整合性） |
| Import | `group_imports` | import を std/external/crate で自動グループ化 |
| Import | `imports_granularity` | import をクレート単位で整理 |
| Import | `imports_layout` | 長い import リストを垂直展開 |
| Doc | `format_code_in_doc_comments` | ドキュメント内のコード例もフォーマット |
| Impl | `reorder_impl_items` | impl ブロック内を一貫した順序で並び替え |
| Struct | `struct_field_align_threshold` | 構造体フィールドを整列 |

参考: rustfmt は全86個の設定オプションを提供。このプロジェクトではベストプラクティスに基づき厳選して使用。

### 除外した設定と理由

| 設定 | 除外理由 |
|------|---------|
| `wrap_comments` | OpenAPI 仕様の破損（doc コメントの改行が `summary` フィールドに混入）、Markdown リンクの破壊（約15箇所）、日本語の不自然な折り返し |
| `normalize_comments` | `wrap_comments` と同様の問題を引き起こすため一緒に除外 |

**詳細**: [ADR-048](../../70_ADR/048_Rustフォーマッター設定の標準化.md#comment-関連設定の除外理由)

## 環境構築

```bash
# rustfmt-nightly のみをインストール
rustup component add rustfmt --toolchain nightly
```

nightly ツールチェーン全体は不要。rustfmt のみで十分。

## CI

`.github/workflows/ci.yaml` で nightly rustfmt を使用するよう設定済み。

## git blame の保護

### `.git-blame-ignore-revs` とは

全体的なリフォーマットを行うと、すべての行の最終変更者が「リフォーマットを実行した人」になってしまい、`git blame` で実際のロジックを書いた人を追跡できなくなる。

`.git-blame-ignore-revs` は、フォーマット変更などの「意味のない変更」を `git blame` から除外するための Git の標準機能。

### 設定方法

```bash
# Git にこのファイルを認識させる（1回のみ、各開発者が実行）
git config blame.ignoreRevsFile .git-blame-ignore-revs
```

### このプロジェクトでの使用

`.git-blame-ignore-revs` に以下のコミットが登録されている:

```
# Rustfmt standardization: comprehensive settings based on best practices
c47998301859a4dd05ff4cb2dec4e9e4e4cb1cbb
```

これにより、rustfmt 包括的設定適用のリフォーマットコミットが `git blame` でスキップされる。

### 今後の運用

全体的なフォーマット変更を行った場合は、`.git-blame-ignore-revs` にコミットハッシュを追加する。

## 参照

- ADR: [048_Rustフォーマッター設定の標準化](../../70_ADR/048_Rustフォーマッター設定の標準化.md)
- 設定ファイル: `backend/.rustfmt.toml`
