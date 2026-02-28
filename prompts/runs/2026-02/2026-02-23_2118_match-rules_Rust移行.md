# 2026-02-23 match-rules の Python → Rust 移行

## 概要

Issue #815（CI スクリプトの言語選定）に基づき、`.github/scripts/match-rules.py`（Python 177行）を `.github/scripts/match-rules.rs`（Rust 362行、テスト含む）に移行した。`rust-script` + `globset` クレートを使用し、CI ワークフローを更新、ADR-056 を作成した。

## 実施内容

### Phase 1: rust-script でスクリプト作成（ロジック + テスト）

1. rust-script v0.36.0 をインストール
2. TDD で実装:
   - Red: テストを先に記述（11 テスト）
   - Green: `parse_frontmatter_paths`, `strip_frontmatter`, `compile_glob`, `match_rules` を実装
   - globset の `literal_separator(true)` が必要であることを TDD で発見
3. Python 版との出力互換性を `diff` で検証（バイト一致）

### Phase 2: CI ワークフロー変更 + ADR + クリーンアップ

1. `claude-rules-check.yaml` を更新:
   - Rust ツールチェイン（1.93.0）+ sccache + Cargo キャッシュ追加
   - `python3` → `rust-script` に変更
2. `.github/scripts/match-rules.py` を削除
3. ADR-056「CI スクリプトの言語選定方針」を作成
4. `just check-all` 全通過を確認

### 検証

- `rust-script --test`: 11 テスト全通過
- 手動実行: Python 版と出力が `diff` でバイト一致
- `just check-all`: exit code 0

## 判断ログ

- globset のデフォルト `*` はパス区切りを超えてマッチする。`GlobBuilder::new(pattern).literal_separator(true)` を指定して Python 互換の挙動にした。TDD の Red フェーズで発見
- `tempfile` クレートはテスト用依存として `[dependencies]` に含めた（rust-script は `[dev-dependencies]` を区別しない）
- cargo script（RFC 3424）は Rust 1.93 時点で未安定化。rust-script を採用し、cargo script を将来の移行パスとして ADR に記録

## 成果物

### コミット

- `d4fe38d #815 Migrate match-rules from Python to Rust (rust-script)`

### 作成/変更ファイル

| ファイル | 操作 |
|---------|------|
| `.github/scripts/match-rules.rs` | 新規作成 |
| `.github/scripts/match-rules.py` | 削除 |
| `.github/workflows/claude-rules-check.yaml` | 変更 |
| `docs/70_ADR/056_CIスクリプトの言語選定方針.md` | 新規作成 |

### PR

- #825（Draft）

## 議論の経緯

- Issue #815 は当初 Shell vs Python の 2 択だったが、プロジェクトの技術スタック（Rust + Elm）と ADR-015 の移行基準を考慮し、Rust（rust-script）を第 4 の選択肢として追加
- ワークスペースクレート案も検討したが、~80行のスクリプトに対して過剰（KISS に反する）として rust-script に決定
- cargo script の安定化状況を調査し、未安定化を確認。将来の移行パスとして ADR に記録
