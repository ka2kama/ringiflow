# CI バイナリキャッシュ導入（Issue #592）

## Context

フロントエンドのみ変更時に `e2e-test` / `api-test` ジョブで不要な `cargo build --release` が走り、数分のコストが発生している。`actions/cache` でリリースバイナリをキャッシュし、バックエンドソース未変更時のビルドをスキップする。

## 対象と対象外

**対象**:
1. `.github/workflows/ci.yaml` — `api-test` ジョブ（L465-467）と `e2e-test` ジョブ（L574-575）にバイナリキャッシュを追加
2. `docs/80_ナレッジベース/devtools/GitHubActions.md` — バイナリキャッシュ戦略セクションを追記

**対象外**:
- `rust-test` / `rust-lint` / `rust-integration`: `rust == 'true'` でジョブ単位スキップ済み
- sccache 設定: 別レイヤーの最適化、そのまま共存
- 変更検出フィルタ: 正しく動作しており変更不要

## 設計判断

### キャッシュキー

```yaml
key: ${{ runner.os }}-backend-release-${{ hashFiles('backend/**/*.rs', 'backend/**/Cargo.toml', 'backend/Cargo.lock') }}
```

- `backend/**/*.rs`: Rust ソース。checkout 後のみ対象（`target/` は `.gitignore` で除外）
- `backend/**/Cargo.toml`: 依存・フィーチャー・プロファイル設定
- `backend/Cargo.lock`: ロックされた依存バージョン
- Rust toolchain バージョン（1.93.0）は含めない: ci.yaml にピン留めされ、変更時は通常 Cargo.toml/Lock も変わるため

### restore-keys 不使用

バイナリキャッシュは完全一致が必要（異なるソースのバイナリは使えない）。Cargo registry キャッシュ（部分ヒットが有用）とは異なる。

### sccache との共存

sccache = 中間生成物キャッシュ、バイナリキャッシュ = 最終成果物キャッシュ。別レイヤーで共存。キャッシュヒット時も sccache セットアップは残す（条件分岐よりシンプル）。

### キャッシュの共有

`actions/cache` は main のキャッシュを他ブランチから参照可能。main への push で CI が走るため、フロントエンド専用ブランチでも main のキャッシュを使える。api-test と e2e-test は同一キーでキャッシュを共有する（同じビルドフラグのため）。

## Phase 1: バイナリキャッシュ導入 + ドキュメント追記

### 確認事項

- [x] パターン: 既存の `actions/cache@v5` 使用パターン → ci.yaml L91-101。`path`（パイプ複数行）、`key`（hashFiles）、`restore-keys` の構文を確認
- [x] パターン: `steps.*.outputs.cache-hit` の条件判定 → プロジェクト内に既存使用なし。actions/cache 公式仕様で `'true'` / `''` を返す。`!= 'true'` で判定
- [x] ライブラリ: `hashFiles` の複数パターン指定 → 既存使用は単一パターンのみ（L99）。カンマ区切りで複数指定可能

### 変更内容

**ci.yaml — api-test ジョブ（L465-467 の前に追加 + ビルドステップに条件追加）:**

```yaml
      - name: Cache backend binaries
        id: backend-cache
        uses: actions/cache@v5
        with:
          path: |
            backend/target/release/ringiflow-bff
            backend/target/release/ringiflow-core-service
            backend/target/release/ringiflow-auth-service
          key: ${{ runner.os }}-backend-release-${{ hashFiles('backend/**/*.rs', 'backend/**/Cargo.toml', 'backend/Cargo.lock') }}

      - name: Build services (production config, no dev features)
        if: steps.backend-cache.outputs.cache-hit != 'true'
        run: cargo build --release --no-default-features
        working-directory: backend
```

**ci.yaml — e2e-test ジョブ（L574-575 の前に追加 + ビルドステップに条件追加）:**

```yaml
      - name: Cache backend binaries
        id: backend-cache
        uses: actions/cache@v5
        with:
          path: |
            backend/target/release/ringiflow-bff
            backend/target/release/ringiflow-core-service
            backend/target/release/ringiflow-auth-service
          key: ${{ runner.os }}-backend-release-${{ hashFiles('backend/**/*.rs', 'backend/**/Cargo.toml', 'backend/Cargo.lock') }}

      - name: Build backend services (production config, no dev features)
        if: steps.backend-cache.outputs.cache-hit != 'true'
        run: cargo build --release --no-default-features
        working-directory: backend
```

**GitHubActions.md — 「PR レビューシステムの制約」セクションの前にバイナリキャッシュセクションを追加:**

バイナリキャッシュ戦略:
- 目的と対象ジョブ
- キャッシュキー設計の根拠
- sccache との違い（中間生成物 vs 最終バイナリ）
- restore-keys を使わない理由
- 変更履歴に追記

### テストリスト

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

### 検証

- [ ] `just lint-ci` で actionlint 通過
- [ ] `just check-all` 通過
- [ ] PR の CI でキャッシュステップの動作確認（初回: cache miss → ビルド実行、2回目: cache hit → ビルドスキップ）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `hashFiles('backend/**/*.rs')` が `target/` 内ファイルにマッチする可能性 | エッジケース | `.gitignore` で除外され checkout 後には存在しない。問題なし |
| 2回目 | `--no-default-features` がキャッシュキーに反映されるか | 未定義 | BFF の default features は Cargo.toml に定義。`backend/**/Cargo.toml` がキーに含まれるため反映済み |
| 3回目 | api-test と e2e-test で同一キーのキャッシュ共有の動作 | 競合・エッジケース | 同じビルドフラグのためバイナリは同一。先に実行されたジョブが保存し後続が利用。効率的 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | Issue の完了基準3項目（e2e-test、api-test、検証）すべて計画に含む |
| 2 | 曖昧さ排除 | OK | キャッシュキー・パス・条件式を具体的な YAML で記載 |
| 3 | 設計判断の完結性 | OK | キーの構成要素、restore-keys、sccache 共存、パス設計の4判断を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象2ファイル、対象外3項目を明記 |
| 5 | 技術的前提 | OK | `hashFiles` の動作、`cache-hit` の出力形式、main キャッシュの共有メカニズムを確認 |
| 6 | 既存ドキュメント整合 | OK | ADR-004 と矛盾なし。Cargo キャッシュ（restore-keys あり）とバイナリキャッシュ（なし）の差異は意図的 |
