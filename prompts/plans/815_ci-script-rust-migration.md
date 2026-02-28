# Issue #815: CI スクリプトの言語選定 — match-rules の Rust 移行

## Context

#813 で `.github/scripts/match-rules.py`（177行、Python）を導入した。プロジェクトの技術スタックは Rust + Elm であり、Shell も Python も好みの言語ではない。

ADR-015（開発スクリプトの品質担保方針）の将来の移行基準:
> スクリプトに複雑なロジック（正規表現パース、条件分岐が多い等）が必要になった場合、rust-script または dev-tools クレートへの移行を検討する

`match-rules.py` は glob-to-regex 変換（正規表現パース）を含み、この移行基準に該当する。`rust-script` で Rust に移行し、ADR で方針を記録する。

## 設計判断

### 実装形式: rust-script（単一ファイルスクリプト）

`.github/scripts/match-rules.rs` に単一ファイルで配置する。

- Python 版と同じディレクトリ・同じ役割（置き換え）
- ADR-015 が明示的に挙げた移行先（「rust-script または dev-tools クレート」）
- ~80行のスクリプトにワークスペースクレートは過剰（KISS）
- 依存はスクリプト内にインライン宣言（Cargo.toml 不要）
- 将来 `cargo script` が安定化（1.94-1.95 見込み）されれば、外部ツール不要に移行可能

### Glob マッチング: `globset` クレート

Python 版の手動 glob-to-regex 変換（~40行）を廃止し、`globset`（ripgrep エコシステム）のネイティブ glob マッチングを使用。

- `Glob::new(pattern)?.compile_matcher()` → `matcher.is_match(path)` で判定
- `**`, `*` を全サポート（実際に使用されている全パターンに対応）
- Python 版との挙動: 両方ともフルパスマッチ、`**` = 0個以上のパスコンポーネント。同等

### 出力互換性

Python 版の出力形式を厳密に再現する。ワークフローが `contains(steps.match-rules.outputs.MATCHED_RULES, '<!-- no-matching-rules -->')` で判定しているため。

## 対象ファイル

| ファイル | 操作 |
|---------|------|
| `.github/scripts/match-rules.rs` | 新規作成（Python 版の置き換え） |
| `.github/scripts/match-rules.py` | 削除 |
| `.github/workflows/claude-rules-check.yaml` | `python3` → `rust-script` に変更 + Rust セットアップ追加 |
| `docs/70_ADR/056_CIスクリプトの言語選定方針.md` | 新規作成 |

対象外: `backend/Cargo.toml`（ワークスペース変更なし）、`scripts/` 配下のシェルスクリプト

## Phase 1: rust-script でスクリプト作成（ロジック + テスト）

### 確認事項
- ライブラリ: `globset::Glob::new()`, `compile_matcher()`, `GlobMatcher::is_match()` → docs.rs で確認
- パターン: Python 版の出力形式 → `.github/scripts/match-rules.py` L141-176
- パターン: Python 版のフロントマターパース → `.github/scripts/match-rules.py` L62-104
- ライブラリ: rust-script のインライン依存宣言の構文 → 既存使用なし → 公式ドキュメント確認

### スクリプト構造

```rust
#!/usr/bin/env rust-script
//! ```cargo
//! [dependencies]
//! globset = "0.4"
//! ```

// --- フロントマターパース ---
fn parse_frontmatter_paths(content: &str) -> Vec<String> { ... }
fn strip_frontmatter(content: &str) -> &str { ... }

// --- マッチング ---
struct MatchedRule { path: String, body: String }
fn match_rules(changed_files: &[String], rules_dir: &Path) -> Vec<MatchedRule> { ... }

// --- CLI ---
fn main() { ... }

// --- テスト ---
#[cfg(test)]
mod tests { ... }
```

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 変更ファイルがルールの glob パターンにマッチ → マッチしたルールが出力される | 正常系 | ユニット |
| 2 | 変更ファイルがどのルールにもマッチしない → `<!-- no-matching-rules -->` | 正常系 | ユニット |
| 3 | 変更ファイル一覧が空 → `<!-- no-matching-rules -->` | 正常系 | ユニット |
| 4 | フロントマターに paths がないルールファイル → スキップ | 正常系 | ユニット |

### テストリスト

ユニットテスト（`rust-script --test` で実行）:
- [ ] `parse_frontmatter_paths`: 標準的なフロントマターから paths リストを抽出できる
- [ ] `parse_frontmatter_paths`: フロントマターがない場合は空リストを返す
- [ ] `parse_frontmatter_paths`: paths キーがない場合は空リストを返す
- [ ] `parse_frontmatter_paths`: クォート（ダブル・シングル）が除去される
- [ ] `strip_frontmatter`: フロントマターを除去して本文を返す
- [ ] `strip_frontmatter`: フロントマターがない場合は元のコンテンツをそのまま返す
- [ ] glob マッチ: `**/*.rs` が深いパスにマッチする
- [ ] glob マッチ: `*` がディレクトリ区切りを超えない
- [ ] `match_rules`: テスト用一時ディレクトリでマッチングが動作する
- [ ] `match_rules`: マッチしたルールがファイル名順でソートされている
- [ ] `match_rules`: paths なしのルールファイルがスキップされる

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: CI ワークフロー変更 + ADR + クリーンアップ

### 確認事項
- パターン: CI の Rust ツールチェインセットアップ → `.github/workflows/ci.yaml` L76-79
- パターン: sccache セットアップ → `ci.yaml` L88-89
- パターン: cargo-binstall インストール → `ci.yaml` L103-104
- テンプレート: ADR → `docs/70_ADR/template.md`

### CI ワークフロー変更（`claude-rules-check.yaml`）

Checkout ステップの後に追加:

```yaml
- name: Setup Rust toolchain
  if: steps.check-draft.outputs.is_draft == 'false'
  uses: dtolnay/rust-toolchain@stable
  with:
    toolchain: 1.93.0

- name: Setup sccache
  if: steps.check-draft.outputs.is_draft == 'false'
  uses: mozilla-actions/sccache-action@v0.0.9

- name: Cache Cargo
  if: steps.check-draft.outputs.is_draft == 'false'
  uses: actions/cache@v5
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
    key: ${{ runner.os }}-rules-check-${{ hashFiles('.github/scripts/match-rules.rs') }}
    restore-keys: |
      ${{ runner.os }}-rules-check-

- name: Install rust-script
  if: steps.check-draft.outputs.is_draft == 'false'
  run: |
    if ! command -v rust-script &> /dev/null; then
      cargo install rust-script
    fi
```

match-rules ステップの変更:

```yaml
- name: Match rules to changed files
  id: match-rules
  if: steps.check-draft.outputs.is_draft == 'false'
  env:
    SCCACHE_GHA_ENABLED: "true"
    RUSTC_WRAPPER: sccache
  run: |
    {
      echo "MATCHED_RULES<<EOF_MATCHED_RULES"
      rust-script .github/scripts/match-rules.rs /tmp/changed-files.txt
      echo "EOF_MATCHED_RULES"
    } >> "$GITHUB_OUTPUT"
```

### ADR-056

CI スクリプトの言語選定方針。内容:
- ADR-015 の移行基準に合致し Rust（rust-script）へ移行した経緯
- `scripts/` は Shell 維持、`.github/scripts/` は Rust（rust-script）を許容
- cargo script 安定化後の移行パス

### クリーンアップ

`.github/scripts/match-rules.py` を削除（`.rs` で置き換え済みのため）

### 操作パス

操作パス: 該当なし（CI ワークフローとドキュメント変更のみ）

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## 検証方法

1. `rust-script --test .github/scripts/match-rules.rs`（ユニットテスト）
2. 手動実行: `echo "backend/apps/bff/src/main.rs" > /tmp/test-files.txt && rust-script .github/scripts/match-rules.rs /tmp/test-files.txt`（出力が Python 版と一致）
3. `just check-all`（既存テストが壊れないこと）
4. PR 作成後、Claude Rules Check ワークフローの実行結果で CI 動作を確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Issue の選択肢が Shell vs Python の枠組みのみ。ADR-015 の移行基準と技術スタックを考慮していない | 既存手段の見落とし | Rust 移行を第 4 の選択肢として追加。Issue を再構成 |
| 2回目 | ワークスペースクレート案は ~80行のスクリプトに過剰。ADR-015 が rust-script を明示 | シンプルさ | rust-script（単一ファイル）に変更 |
| 3回目 | cargo script（RFC 3424）の安定化状況が未確認 | 既存手段の見落とし | 調査の結果 1.93 では未安定化。rust-script を採用し、cargo script を将来の移行パスとして ADR に記録 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | スクリプト作成、テスト、CI 変更、Python 削除、ADR の全てを網羅 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 「必要に応じて」等の不確定表現なし |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | 実装形式（rust-script）、glob 方式（globset）、出力互換性を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: match-rules 移行 + ADR。対象外: backend/Cargo.toml、scripts/ 配下 |
| 5 | 技術的前提 | 前提が考慮されている | OK | globset のマッチセマンティクスの同等性検証済み、cargo script 未安定化を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-015 の移行基準・移行先に合致 |
