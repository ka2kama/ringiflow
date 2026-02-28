# sccache 導入と worktree セットアップ自動化

Issue: #355

## 概要

git worktree の新規作成時に発生する依存クレートの再ビルドと手動セットアップを解消するため、sccache によるコンパイルキャッシュ共有と worktree セットアップの自動化を実装した。

## 背景と目的

Claude Code が複数 worktree で並行稼働する運用において、2つの課題があった:

1. worktree 新規作成のたびに Rust の依存クレートが再ビルドされる（数分）
2. Docker 起動、DB マイグレーション、依存関係インストールを毎回手動で実行する必要がある

`CARGO_TARGET_DIR` 共有は並行ビルドでロック競合とキャッシュスラッシングを起こすため不適切。sccache のオブジェクトレベルキャッシュで安全に共有し、セットアップ手順は `just setup-worktree` として自動化した。

## 実施内容

### Phase 1: sccache 設定と CI 対応

- `backend/.cargo/config.toml` を新規作成し `rustc-wrapper = "sccache"` を設定
- CI の 3 つの Rust ジョブ（`rust`, `rust-integration`, `api-test`）に `CARGO_BUILD_RUSTC_WRAPPER: ""` を追加して sccache を無効化
- `justfile` の `check-tools` に sccache のインストール確認を追加

### Phase 2: worktree セットアップ自動化

- `justfile` に `setup-worktree` レシピを追加（`dev-deps` → `setup-db` → `setup-deps`）
- `worktree-add` レシピに `*flags` 可変長引数を追加し `--no-setup` をパススルー可能に
- `scripts/worktree-add.sh` に `--no-setup` フラグの解析と `just setup-worktree` の自動実行を追加
- `scripts/worktree-issue.sh` の完了メッセージを簡略化

### Phase 3: ドキュメント更新

- `docs/60_手順書/01_開発参画/01_開発環境構築.md`: sccache セクション新設（セクション 19）、概要テーブル追加、トラブルシューティング更新
- `docs/60_手順書/00_はじめに.md`: トラブルシューティング更新
- `docs/60_手順書/01_開発参画/02_プロジェクトセットアップ.md`: トラブルシューティング更新
- `docs/60_手順書/04_開発フロー/04_並行開発（Worktree）.md`: 自動セットアップに合わせて手順を簡略化、sccache の説明に変更
- `docs/80_ナレッジベース/devtools/git_worktree.md`: `CARGO_TARGET_DIR` 共有を非推奨にし sccache を推奨

## 設計上の判断

| 判断 | 選択 | 理由 |
|------|------|------|
| sccache 設定場所 | `backend/.cargo/config.toml` | `just` 経由でも直接 `cargo` でも自動適用。`.env` + `RUSTC_WRAPPER` だと `just` 経由でないと効かない |
| CI 無効化方法 | `CARGO_BUILD_RUSTC_WRAPPER: ""` | Cargo の設定優先順位（環境変数 > config.toml）を活用。CI では `actions/cache` が代替 |
| `setup-worktree` の構成 | `dev-deps` + `setup-db` + `setup-deps` | `setup` から worktree で不要なステップ（`check-tools`, `setup-env`, `setup-hooks`）を除外 |
| `CARGO_TARGET_DIR` 共有 | 非推奨化 | 並行ビルドでファイルロック競合とキャッシュスラッシングが発生する。sccache はオブジェクトレベルキャッシュのため安全 |

## 判断ログ

特筆すべき判断なし。計画通りに実装を完了した。

## 成果物

コミット:
- `e7dce59` #355 Introduce sccache and automate worktree setup

変更ファイル（10 ファイル）:
- `backend/.cargo/config.toml`（新規）
- `.github/workflows/ci.yaml`
- `justfile`
- `scripts/worktree-add.sh`
- `scripts/worktree-issue.sh`
- ドキュメント 5 ファイル

## 議論の経緯

計画が事前に策定済みだったため、実装は計画に沿って進めた。特別な議論や方針変更はなかった。

## 学んだこと

- Cargo の設定優先順位（CLI > 環境変数 > config.toml > デフォルト）を活用すると、ローカル設定と CI 設定を綺麗に分離できる
- sccache はオブジェクトレベルのキャッシュであり、独立した `target/` ディレクトリでも同じソースコードのコンパイル結果を共有できる。`CARGO_TARGET_DIR` 共有とは異なりロック競合が起きない
- justfile の `*flags` 可変長引数を使うと、オプションフラグをスクリプトに綺麗にパススルーできる
