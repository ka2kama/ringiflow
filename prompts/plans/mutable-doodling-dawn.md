# 計画: #447 Rust CI のビルド時間を短縮する

## Context

Rust CI ジョブ（`rust`）がパイプライン全体のボトルネック。キャッシュ冷時 ~10 分、暖時 ~6 分。
目標: 安定して 6 分以内にする。

Issue の方針「改善は段階的に行い、各変更の効果を計測してから次に進む」に従い、
短期改善の中から効果が大きく低リスクな2項目を実施する。

## スコープ

### 対象

1. **cargo-binstall の導入**: ツールのプリビルドバイナリインストール
2. **sccache の CI 有効化**: コンパイルキャッシュの改善

### 対象外

- `--release` フラグの除去（別途ベンチマーク後に判断）
- production build 検証の条件付き実行（設計判断が必要、別 Issue 候補）
- ジョブ間のビルドアーティファクト共有（大きなリファクタ）
- Larger Runners（有料オプション）

## 設計判断

### 1. cargo-binstall の導入方法

**選択肢:**
- A: 公式インストールスクリプト（`curl | bash`）
- B: GitHub Action（`cargo-bins/cargo-binstall`）

**選択: A（インストールスクリプト）**

理由:
- GitHub Actions 許可リストへの追加が不要
- 依存する外部 Action が増えない
- インストールスクリプトは公式で信頼性あり

### 2. sccache のキャッシュ戦略

**選択肢:**
- A: sccache + actions/cache 併用（target/ 含む）
- B: sccache のみ（actions/cache から target/ を除外）
- C: sccache + actions/cache 併用（target/ 除外、~/.cargo/ のみ）

**選択: C**

理由:
- sccache が compilation unit レベルでキャッシュするため、`target/` 全体のキャッシュは冗長
- `~/.cargo/{registry,git,bin}` は sccache の対象外なので actions/cache で引き続きキャッシュ
- キャッシュの保存/復元サイズが小さくなり、時間短縮

### 3. sccache-action のバージョン

`mozilla-actions/sccache-action@v0.0.9`（2024-06-18 リリース、最新 stable）を使用。
GitHub Actions 許可リストへの追加が必要。
sccache-action は `SCCACHE_GHA_ENABLED` と `ACTIONS_RESULTS_URL`/`ACTIONS_RUNTIME_TOKEN` を自動設定する。

### 4. フォールバック戦略

cargo-binstall が失敗した場合に `cargo install` にフォールバックする。
CI の安定性を優先する。

## 変更対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.github/workflows/ci.yaml` | cargo-binstall 導入、sccache 追加、cache 調整 |
| `docs/06_ナレッジベース/devtools/GitHubActions.md` | 許可設定テーブルに sccache-action を追加 |

## Phase 1: cargo-binstall の導入

#### 確認事項

- [ ] cargo-binstall インストールスクリプトの URL と使い方 → 公式 README
- [ ] cargo-machete のバイナリ配布有無 → crates.io / GitHub releases
- [ ] sqlx-cli のバイナリ配布有無 → crates.io / GitHub releases
- [ ] sqlx-cli の features 指定が binstall でサポートされるか → cargo-binstall ドキュメント

#### 変更内容

**rust ジョブ（cargo-machete）:**

現状:
```yaml
- name: Install cargo-machete
  run: |
    if ! command -v cargo-machete &> /dev/null; then
      cargo install cargo-machete --locked
    fi
```

変更後:
```yaml
- name: Install cargo-binstall
  run: curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

- name: Install cargo-machete
  run: |
    if ! command -v cargo-machete &> /dev/null; then
      cargo binstall cargo-machete --locked --no-confirm
    fi
```

**rust-integration / api-test / e2e-test ジョブ（sqlx-cli）:**

現状:
```yaml
- name: Install sqlx-cli
  run: |
    if ! command -v sqlx &> /dev/null; then
      cargo install sqlx-cli --no-default-features --features rustls,postgres
    fi
```

変更後:
```yaml
- name: Install cargo-binstall
  run: curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash

- name: Install sqlx-cli
  run: |
    if ! command -v sqlx &> /dev/null; then
      cargo binstall sqlx-cli --locked --no-confirm
    fi
```

注: cargo-binstall は `--no-default-features --features` フラグをサポートしていない（実装時に判明）。
プリビルドバイナリは全機能を含むため、features 指定なしで問題ない。

#### テストリスト

ユニットテスト: 該当なし（CI ワークフローの変更）

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

CI 検証:
- [ ] rust ジョブで cargo-machete が正常にインストール・実行されること
- [ ] rust-integration ジョブで sqlx-cli が正常にインストールされること
- [ ] api-test ジョブで sqlx-cli が正常にインストールされること
- [ ] e2e-test ジョブで sqlx-cli が正常にインストールされること

#### 期待効果

| ジョブ | ツール | Before | After | 削減 |
|-------|--------|--------|-------|------|
| rust | cargo-machete | ~51s | ~5-10s | ~40-45s |
| rust-integration | sqlx-cli | ~3-5min | ~10-20s | ~3-5min |
| api-test | sqlx-cli | ~3-5min | ~10-20s | ~3-5min |
| e2e-test | sqlx-cli | ~3-5min | ~10-20s | ~3-5min |

## Phase 2: sccache の CI 有効化

#### 確認事項

- [ ] mozilla-actions/sccache-action の最新バージョン → GitHub releases
- [ ] sccache-action の設定方法（環境変数、キャッシュキー）→ 公式 README
- [ ] actions/cache から target/ を除外した場合の影響 → sccache がカバーするか確認
- [ ] security ジョブの CARGO_BUILD_RUSTC_WRAPPER 設定の理由 → cargo-deny の Docker コンテナとの関係
- [x] sccache-action の間接依存（内部で呼び出す Action）の有無 → action.yml 確認済み、間接依存なし（自己完結型 Node.js アクション）

#### 変更内容

**全 Rust ジョブ（rust, rust-integration, api-test, e2e-test）に共通:**

1. sccache-action を追加:
```yaml
- name: Setup sccache
  uses: mozilla-actions/sccache-action@v0.0.9
```

2. `CARGO_BUILD_RUSTC_WRAPPER: ""` を削除（sccache を有効化）

3. `SCCACHE_GHA_ENABLED: "on"` を環境変数に追加（sccache-action が自動設定するが明示的にも設定）

4. actions/cache のパスから `backend/target/` を除外:
```yaml
- name: Cache Cargo
  uses: actions/cache@v5
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}
    restore-keys: |
      ${{ runner.os }}-cargo-
```

**security ジョブは変更しない:**
`cargo-deny-action` は Docker コンテナ内で実行されるため、sccache は使用不可。
`CARGO_BUILD_RUSTC_WRAPPER: ""` のコメントも既に正確（「cargo-deny の Docker コンテナ内にも存在しない」）。

**ナレッジベース更新:**

`docs/06_ナレッジベース/devtools/GitHubActions.md` の許可設定テーブルに追加:
```
| `mozilla-actions/sccache-action@*` | Rust コンパイルキャッシュ |
```

#### テストリスト

ユニットテスト: 該当なし

ハンドラテスト: 該当なし

API テスト: 該当なし

E2E テスト: 該当なし

CI 検証:
- [ ] rust ジョブで sccache が有効化され、コンパイルが成功すること
- [ ] rust-integration ジョブで sccache が有効化され、テストが通ること
- [ ] api-test ジョブで sccache が有効化され、テストが通ること
- [ ] e2e-test ジョブで sccache が有効化され、テストが通ること
- [ ] sccache のキャッシュヒット統計が出力されること（sccache --show-stats）

#### 期待効果

sccache は初回実行時にキャッシュを構築し、2回目以降で効果を発揮する。

- キャッシュ冷時: ほぼ変化なし（キャッシュ構築のみ）
- キャッシュ暖時: コンパイル時間 30-50% 短縮の見込み
- Cargo.lock 変更時: 変更されたクレートのみ再コンパイル（actions/cache では全再コンパイル）

actions/cache + Cargo.lock ハッシュ方式との比較:
- actions/cache: Cargo.lock が変わると target/ 全体がキャッシュミス → 全再コンパイル
- sccache: コンパイル単位でキャッシュ → Cargo.lock 変更でも変更クレートのみ再コンパイル

## Phase 3: 効果測定と Issue 更新

1. CI 実行ログから各ジョブの所要時間を記録
2. Before/After を比較
3. Issue のチェックリストを更新
4. 残りの改善候補の優先度を再評価

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | security ジョブの `CARGO_BUILD_RUSTC_WRAPPER` は cargo-deny の Docker コンテナ内で sccache が存在しないため必要 → 変更対象外にすべき | アーキテクチャ不整合 | security ジョブを Phase 2 の変更対象から除外 |
| 2回目 | `--release` 除去は OpenAPI 生成・production build と連動するため、テストだけ dev にすると2プロファイルのコンパイルが発生し逆効果の可能性 | 既存手段の見落とし | `--release` 除去はスコープ外（別途ベンチマーク後に判断）として明記 |
| 3回目 | cargo-binstall の `--no-default-features --features` がバイナリインストール時にどう扱われるか不明 | 曖昧 | binstall はプリビルドバイナリがない場合 cargo install にフォールバックする旨を記載。実装時に公式ドキュメントで確認 |
| 4回目 | cargo-binstall は `--no-default-features --features` フラグをサポートしていない（実装時に Auto Review で判明） | 技術的前提 | Phase 1 の sqlx-cli コマンドから features 指定を削除。プリビルドバイナリは全機能を含むため影響なし |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の短期改善候補3項目のうち2項目を対象、1項目は理由付きで対象外 |
| 2 | 曖昧さ排除 | OK | 各 Phase の変更内容をコードスニペットで明示 |
| 3 | 設計判断の完結性 | OK | binstall 導入方法、sccache 戦略、フォールバック方針に判断理由を記載 |
| 4 | スコープ境界 | OK | 対象（binstall, sccache）と対象外（--release 除去、production build 条件化）を明記 |
| 5 | 技術的前提 | OK | sccache-action の動作、cargo-deny との非互換を確認 |
| 6 | 既存ドキュメント整合 | OK | GitHub Actions ナレッジベースの許可設定テーブル更新を計画に含む |
