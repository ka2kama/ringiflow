# ADR-057: workspace.lints による lint 管理の標準化

## ステータス

承認済み

## コンテキスト

Rust の clippy lint レベルを CLI フラグ（`-- -D warnings`）で管理していた。この方法には以下の問題がある:

1. 二重管理: `justfile` と `.github/workflows/ci.yaml` の両方に同じフラグを記述する必要がある
2. 宣言性の欠如: lint 設定がビルドコマンドに埋め込まれており、コードベースの一部として管理されていない
3. 拡張性の制限: 個別 lint の deny/allow を追加するたびに CLI フラグが長くなる

Cargo 1.74 で導入された `[workspace.lints]` は、lint 設定を `Cargo.toml` に宣言的に記述する仕組みを提供する。

## 検討した選択肢

### 選択肢 1: CLI フラグを維持

`-- -D warnings` を justfile と CI の両方で維持する現状維持案。

評価:
- 利点: 変更不要
- 欠点: 二重管理が続く、個別 lint 設定の追加が煩雑

### 選択肢 2: `[workspace.lints]` に一本化

lint 設定を `backend/Cargo.toml` の `[workspace.lints.clippy]` に集約し、CLI フラグを削除する。

評価:
- 利点: 宣言的、単一の管理場所、個別 lint の override が容易
- 欠点: 各 member crate に `[lints] workspace = true` の追加が必要（一度きりの作業）

### 比較表

| 観点 | CLI フラグ維持 | workspace.lints |
|------|-------------|----------------|
| 管理場所 | justfile + CI YAML（2箇所） | Cargo.toml（1箇所） |
| 宣言性 | × | ◎ |
| 個別 lint 設定 | CLI フラグが長大化 | TOML テーブルで整理 |
| VCS 管理 | ビルドスクリプトに分散 | コードベースと同居 |

## 決定

**選択肢 2: `[workspace.lints]` に一本化** を採用する。

主な理由:

1. **単一管理場所**: lint 設定が `backend/Cargo.toml` の 1 箇所に集約される
2. **宣言的設定**: lint ルールがコードベースの一部としてバージョン管理される
3. **拡張性**: Phase 2 で pedantic lint を追加する際、TOML テーブルに行を追加するだけで済む

### 設定

```toml
# backend/Cargo.toml
[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
```

- `clippy::all` はデフォルト lint グループに対応し、従来の `-D warnings` と同等
- `priority = -1` により、個別 lint を `"allow"` で上書き可能（デフォルト priority は 0）

## 帰結

### 肯定的な影響

- lint 設定の二重管理が解消される
- 個別 lint の追加・変更が容易になる（Phase 2 の pedantic lint 導入の基盤）
- lint ルールがコードレビューの対象になる

### 否定的な影響・トレードオフ

- 各 member crate の `Cargo.toml` に `[lints] workspace = true` を追加する一回限りの作業が必要
- Cargo 1.74 以降が必須（プロジェクトは edition 2024 を使用しており問題なし）

### 関連ドキュメント

- Issue: #923
- ルール: `.claude/rules/rust.md`

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-25 | 初版作成 |
