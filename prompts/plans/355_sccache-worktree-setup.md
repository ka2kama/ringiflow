# Issue #355: sccache 導入と worktree セットアップ自動化

## Context

git worktree を新規作成するたびに、依存クレートの再ビルド（数分）と手動セットアップステップが並行開発のボトルネットになっている。Claude Code が複数 worktree で同時に稼働するため、共有 target-dir はロック競合とキャッシュスラッシングで不適切。sccache によるコンパイルキャッシュ共有と、worktree 作成スクリプトのセットアップ自動化で解決する。

## 対象

- sccache の設定（`.cargo/config.toml`）
- CI での sccache 無効化（`ci.yaml`）
- justfile への `setup-worktree` レシピ追加と `check-tools` 更新
- `worktree-add.sh` のセットアップ自動化（`--no-setup` オプション付き）
- 関連ドキュメント更新（3 つのトラブルシューティング + worktree 手順書 + ナレッジベース + 開発環境構築）

## 対象外

- CI での sccache 導入（`actions/cache` で代替済み、別 Issue で検討）
- sccache のリモートストレージ設定（ローカルキャッシュのみ）

## 設計判断

| 判断 | 選択 | 理由 |
|------|------|------|
| sccache 設定場所 | `backend/.cargo/config.toml` | `just` 経由でも直接 `cargo` でも有効。CI は `CARGO_BUILD_RUSTC_WRAPPER=""` で簡単に無効化 |
| CI 無効化方法 | `CARGO_BUILD_RUSTC_WRAPPER: ""` | Cargo の環境変数オーバーライド（config.toml より優先） |
| worktree セットアップ | `just setup-worktree` レシピ | `setup` から不要なステップ（check-tools, setup-env, setup-hooks）を省略した worktree 版 |
| `CARGO_TARGET_DIR` 共有 | 非推奨→sccache で代替 | 並行ビルドでロック競合・キャッシュスラッシングが起きない |

---

## Phase 1: sccache 設定と CI 対応

### 確認事項

- ライブラリ: `CARGO_BUILD_RUSTC_WRAPPER` が `build.rustc-wrapper` を上書きすること → Cargo リファレンス

### 変更ファイル

**1. `backend/.cargo/config.toml`（新規作成）**

```toml
[build]
rustc-wrapper = "sccache"
```

**2. `.github/workflows/ci.yaml`（3 箇所）**

各 Rust ジョブの `env` セクションに追加:
- `rust` ジョブ（L64）
- `rust-integration` ジョブ（L156）
- `api-test` ジョブ（L305）

```yaml
CARGO_BUILD_RUSTC_WRAPPER: ""  # sccache は CI では使用しない（actions/cache で代替）
```

**3. `justfile`（check-tools に sccache 追加）**

L45（cargo-machete の後）に:
```
@which sccache > /dev/null || (echo "ERROR: sccache がインストールされていません" && exit 1)
```

### テストリスト

- [ ] `cd backend && cargo build` で sccache が使用される（`sccache --show-stats`）
- [ ] `CARGO_BUILD_RUSTC_WRAPPER="" cargo build` で sccache が無効化される

---

## Phase 2: worktree セットアップ自動化

### 確認事項: なし（既知のパターンのみ）

### 変更ファイル

**1. `justfile`**

- `setup-worktree` レシピ追加（L77 `reset-db` の後）
- `worktree-add` レシピに `*flags` パラメータ追加（`--no-setup` をパススルー）

```just
# worktree 用セットアップ（Docker 起動 → DB マイグレーション → 依存関係インストール）
setup-worktree: dev-deps setup-db setup-deps
    @echo ""
    @echo "✓ worktree セットアップ完了"
```

```just
worktree-add name branch *flags:
    ./scripts/worktree-add.sh {{flags}} {{name}} {{branch}}
```

**2. `scripts/worktree-add.sh`**

- `--no-setup` フラグのオプション解析を追加
- `.env` 生成後に `just setup-worktree` を自動実行（`--no-setup` でスキップ）
- 末尾メッセージの更新

**3. `scripts/worktree-issue.sh`**

- 末尾の「次のステップ」メッセージを更新（手動ステップ不要になるため）

### テストリスト

- [ ] `worktree-add.sh name branch` でセットアップが自動実行される
- [ ] `worktree-add.sh --no-setup name branch` でスキップされる
- [ ] `just worktree-add name branch --no-setup` でパススルーされる

---

## Phase 3: ドキュメント更新

### 確認事項: なし（既知のパターンのみ）

### 変更ファイル

**1. `docs/04_手順書/01_開発参画/01_開発環境構築.md`**

- 概要テーブル（L12-33）に sccache 行を追加
- セクション 19 として sccache インストール手順を新設（既存 19〜20 を繰り下げ）
- トラブルシューティング「Rust のビルドが遅い」（L806-815）を `.cargo/config.toml` 方式に更新

**2. `docs/04_手順書/00_はじめに.md`**（L109-116 トラブルシューティング更新）

**3. `docs/04_手順書/01_開発参画/02_プロジェクトセットアップ.md`**（L224-233 トラブルシューティング更新）

**4. `docs/04_手順書/04_開発フロー/04_並行開発（Worktree）.md`**

- 「worktree で開発を開始する」（L54-76）: 自動セットアップに合わせて簡略化
- 「cargo build のキャッシュ」（L133-142）: sccache の説明に変更

**5. `docs/06_ナレッジベース/devtools/git_worktree.md`**

- 「ディスク使用量」（L149-161）: `CARGO_TARGET_DIR` 共有を非推奨にし sccache を推奨

---

## 検証

```bash
# Phase 1: sccache 動作確認
cd backend && cargo clean && cargo build  # sccache 経由でビルド
sccache --show-stats                       # キャッシュヒットを確認

# Phase 2: CI オーバーライド確認
CARGO_BUILD_RUSTC_WRAPPER="" cargo check   # sccache なしで動作すること

# 全体: just check-all
just check-all
```

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | sccache 設定を `.env` にするか `.cargo/config.toml` にするか未決定 | 未定義 | 並行開発で直接 `cargo test` も使うため `.cargo/config.toml` を採用、CI は環境変数で無効化 |
| 2回目 | トラブルシューティングが 3 箇所（はじめに、開発環境構築、プロジェクトセットアップ）に散在 | 不完全なパス | 3 箇所すべてを更新対象に追加 |
| 3回目 | `just worktree-add` が `--no-setup` をスクリプトに渡せない | 未定義 | justfile の `worktree-add` レシピに `*flags` パラメータを追加 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | sccache 設定、CI 対応、自動化、ドキュメント（6 ファイル）をすべて計画に含む |
| 2 | 曖昧さ排除 | OK | 各ファイルの変更箇所と行番号を明示 |
| 3 | 設計判断の完結性 | OK | 設定場所（.cargo/config.toml vs .env）、CI 無効化方法、CARGO_TARGET_DIR 非推奨化の判断理由を記載 |
| 4 | スコープ境界 | OK | 対象外（CI での sccache、リモートストレージ）を明記 |
| 5 | 技術的前提 | OK | `CARGO_BUILD_RUSTC_WRAPPER` の優先度が config.toml より高いことを Cargo リファレンスで確認予定 |
| 6 | 既存ドキュメント整合 | OK | 既存 3 箇所のトラブルシューティングを更新対象に含む |
