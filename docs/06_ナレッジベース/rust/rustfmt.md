# rustfmt

## 概要

Rust の公式コードフォーマッター。一貫したコードスタイルを自動的に維持する。

公式: https://github.com/rust-lang/rustfmt

## このプロジェクトでの使用

### 基本設定

- **`tab_spaces = 4`**: Rust 公式デフォルト
- **unstable features を使用**: import 自動整理などの高度な機能を有効化

### 実行方法

```bash
# 推奨: justfile 経由
just fmt-rust

# 直接実行（nightly 必須）
cargo +nightly fmt
```

**重要**: `cargo fmt`（stable）ではなく、必ず `cargo +nightly fmt` または `just fmt-rust` を使用すること。

### unstable features

以下の機能が有効:

| 設定 | 効果 |
|------|------|
| `group_imports` | import を std/external/crate で自動グループ化 |
| `imports_granularity` | import をクレート単位で整理 |
| `imports_layout` | 長い import リストを垂直展開 |
| `format_code_in_doc_comments` | ドキュメント内のコード例もフォーマット |
| `reorder_impl_items` | impl ブロック内を一貫した順序で並び替え |
| `struct_field_align_threshold` | 構造体フィールドを整列 |

これらは手動では維持困難で、自動化の価値が高い。

## 環境構築

```bash
# rustfmt-nightly のみをインストール
rustup component add rustfmt --toolchain nightly
```

nightly ツールチェーン全体は不要。rustfmt のみで十分。

## CI

`.github/workflows/ci.yaml` で nightly rustfmt を使用するよう設定済み。

## 参照

- ADR: [048_Rustフォーマッター設定の標準化](../../05_ADR/048_Rustフォーマッター設定の標準化.md)
- 設定ファイル: `backend/.rustfmt.toml`
