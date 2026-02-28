# DevAuth の Feature Flag 導入

## 概要

DevAuth（開発用認証バイパス）を Cargo feature flag `dev-auth` による条件コンパイルに切り替え、本番ビルドからコードを完全に除外した。ADR-034 を作成し、ナレッジベースも更新した。

## 背景と目的

Issue #288: DevAuth は環境変数 `DEV_AUTH_ENABLED` で制御されていたが、本番バイナリにもコードが含まれていた。セキュリティの多層防御として、コンパイル時に本番ビルドから除外する仕組みが必要だった。

## 実施内容

### Phase 1: Feature flag 導入 + コード修正

- `backend/apps/bff/Cargo.toml` に `[features]` セクションを追加（`default = ["dev-auth"]`）
- `lib.rs`, `config.rs`, `main.rs` の DevAuth 関連コードを `#[cfg(feature = "dev-auth")]` でガード
- リリースビルドのパニックガード（`#[cfg(not(debug_assertions))]`）を削除

### Phase 2: Dockerfile + CI 更新

- Dockerfile の `cargo chef cook` と `cargo build` に `--no-default-features` を追加
- CI に本番ビルド検証ステップを追加

### Phase 3: ADR + ドキュメント更新

- ADR-034 を作成
- DevAuth ナレッジベースの安全策セクションを更新

## 設計上の判断

### パニックガードの削除

Issue の前提を精査した結果、既に `#[cfg(not(debug_assertions))]` によるパニックガードが存在していた。feature flag 導入後はこれが冗長になるだけでなく、feature enabled + release build（CI の API テスト）でパニックするリスクがあるため削除した。

feature flag は `#[cfg(not(debug_assertions))]` より精度が高い:
- `not(debug_assertions)`: リリースビルド全般に作用（CI の API テスト用ビルドも該当）
- `feature = "dev-auth"`: 本番ビルド（`--no-default-features`）のみを正確にターゲット

### default feature への配置

`dev-auth` を `default` features に含めた理由:
- `cargo run` だけで開発可能な DX を維持
- 本番ビルドは `--no-default-features` で明示的に除外
- Rust エコシステムの標準的なパターン

## 成果物

### コミット

| コミット | 内容 |
|---------|------|
| `e8cac89` | Phase 1: Feature flag 導入 + コード修正 |
| `a8d031d` | Phase 2: Dockerfile + CI 更新 |
| `de4a964` | Phase 3: ADR + ドキュメント更新 |

### 作成・更新ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/bff/Cargo.toml` | `[features]` セクション追加 |
| `backend/apps/bff/src/lib.rs` | `dev_auth` モジュールを条件コンパイル |
| `backend/apps/bff/src/config.rs` | `dev_auth_enabled` を条件コンパイル、パニックガード削除 |
| `backend/apps/bff/src/main.rs` | DevAuth import と初期化を条件コンパイル |
| `backend/Dockerfile` | `--no-default-features` 追加 |
| `.github/workflows/ci.yaml` | 本番ビルド検証ステップ追加 |
| `docs/70_ADR/034_DevAuthのFeatureFlag導入.md` | 新規作成 |
| `docs/80_ナレッジベース/security/DevAuth.md` | 安全策セクション更新 |

### PR

- Draft PR #308: [#288 Introduce dev-auth feature flag for compile-time exclusion](https://github.com/ka2kama/ringiflow/pull/308)

## 議論の経緯

### Issue 前提の精査

Issue #288 は「環境変数のみでガード」という前提だったが、実際には `#[cfg(not(debug_assertions))]` によるパニックガードが既に存在していた。この発見により、パニックガードの扱い（削除 vs 共存）を検討する必要が生じた。feature flag とパニックガードの共存は CI の API テスト（`cargo build --release`）でパニックする問題があるため、削除が妥当と判断した。

## 学んだこと

- `#[cfg(not(debug_assertions))]` と `#[cfg(feature = "...")]` は粒度が異なる。前者はビルドプロファイル全体に作用し、後者は特定の機能単位で制御できる
- `cargo chef cook` に渡す feature flags は `cargo build` と一致させる必要がある。一致しないと依存関係のキャッシュが無駄になる
- Issue の前提は常に精査すべき。実装を読んで初めて分かる事実がある

## 次のステップ

- PR #308 のレビュー対応・マージ
