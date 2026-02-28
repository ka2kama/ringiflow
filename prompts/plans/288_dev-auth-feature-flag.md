# Plan: #288 Prevent DevAuth from being enabled in production builds

## Context

DevAuth（開発用認証バイパス）が環境変数 `DEV_AUTH_ENABLED` のみでガードされており、本番ビルドでもコードがバイナリに含まれる。Cargo feature flag `dev-auth` を導入し、本番ビルドからコンパイル時に除外する。

**Issue 精査での発見**: 現状は「環境変数のみ」ではなく、`#[cfg(not(debug_assertions))]` によるリリースビルドでのパニックガードも存在する（`config.rs:34-42`）。feature flag 導入後はこのパニックガードは冗長になるため削除する。

## To-Be

- `#[cfg(feature = "dev-auth")]` で DevAuth コードが条件コンパイルされる
- 開発時は default feature で有効、本番ビルド（Dockerfile）では除外
- CI で本番ビルド（feature なし）のコンパイルが検証される

## 対象・対象外

対象: BFF クレートの feature flag 導入、Dockerfile 更新、CI 更新、ADR 作成、ナレッジベース更新
対象外: Core Service / Auth Service（DevAuth は BFF のみ）

## 設計判断

### 1. feature flag の設計

```toml
[features]
default = ["dev-auth"]
dev-auth = []
```

- `default` に含めることで `cargo run` だけで開発可能（DX 維持）
- 本番ビルドは `--no-default-features` で明示的に除外
- `dev-auth` は追加の依存を持たない（既存の依存で十分）

### 2. パニックガード（`#[cfg(not(debug_assertions))]`）の扱い

**削除する。** feature flag がコンパイル時に除外するため冗長。

- feature disabled → コード自体が存在しない（パニックの必要なし）
- feature enabled → 開発/テスト環境。env var で制御

パニックガードを残すと「feature enabled + release build」でパニックし、CI の API テスト（`cargo build --release`）が壊れる。

### 3. CI の対応

- `cargo clippy --all-features` → DevAuth コードもlint対象 ✅（変更不要）
- `cargo test --all-features` → DevAuth テストも実行 ✅（変更不要）
- `cargo build --release`（API テスト用）→ default features で DevAuth 含む。`.env.api-test` は `DEV_AUTH_ENABLED` を設定しないので問題なし
- **追加**: `cargo build --release --no-default-features` の検証ステップを CI に追加し、本番ビルドが壊れていないことを保証

## 実装計画

### Phase 1: Feature flag 導入 + コード修正

**変更ファイル:**

| ファイル | 変更内容 |
|---------|---------|
| `backend/apps/bff/Cargo.toml` | `[features]` セクション追加 |
| `backend/apps/bff/src/lib.rs:13` | `pub mod dev_auth;` → `#[cfg(feature = "dev-auth")] pub mod dev_auth;` |
| `backend/apps/bff/src/config.rs` | `dev_auth_enabled` フィールドと解析を `#[cfg(feature = "dev-auth")]` でガード。パニックガード削除 |
| `backend/apps/bff/src/main.rs:87,132-150` | import と初期化ブロックを `#[cfg(feature = "dev-auth")]` でガード |

**config.rs の変更詳細:**

```rust
pub struct BffConfig {
   pub host: String,
   pub port: u16,
   pub redis_url: String,
   pub core_url: String,
   pub auth_url: String,
   #[cfg(feature = "dev-auth")]
   pub dev_auth_enabled: bool,
}

impl BffConfig {
   pub fn from_env() -> Result<Self, env::VarError> {
      #[cfg(feature = "dev-auth")]
      let dev_auth_enabled = env::var("DEV_AUTH_ENABLED")
         .map(|v| v.eq_ignore_ascii_case("true"))
         .unwrap_or(false);

      // パニックガード削除（feature flag が代替）

      Ok(Self {
         // ...
         #[cfg(feature = "dev-auth")]
         dev_auth_enabled,
      })
   }
}
```

**main.rs の変更詳細:**

```rust
#[cfg(feature = "dev-auth")]
use ringiflow_bff::dev_auth;

// ...

#[cfg(feature = "dev-auth")]
if config.dev_auth_enabled {
   // ... 既存の初期化コード
}
```

**テスト:**

- `dev_auth.rs` のテスト: モジュールごと `#[cfg(feature = "dev-auth")]` でガードされるため自動的に除外
- `config.rs` のテスト: `parse_dev_auth_enabled` のテストも `#[cfg(feature = "dev-auth")]` でガード
- `cargo test --all-features` で全テスト実行を確認

### Phase 2: Dockerfile + CI 更新

**変更ファイル:**

| ファイル | 変更内容 |
|---------|---------|
| `backend/Dockerfile:51` | `cargo chef cook --release --no-default-features` に変更 |
| `backend/Dockerfile:63` | `cargo build --release --no-default-features` に変更 |
| `.github/workflows/ci.yaml` | 本番ビルド検証ステップを追加（Rust job 内） |

**Dockerfile の変更:**

```dockerfile
# Stage 3: Builder
RUN cargo chef cook --release --no-default-features --recipe-path recipe.json
# ...
RUN cargo build --release --no-default-features --bin ringiflow-bff --bin ringiflow-core-service --bin ringiflow-auth-service
```

**CI の追加ステップ（Rust job 内）:**

```yaml
- name: Verify production build (no dev features)
  run: cargo build --release --no-default-features --bin ringiflow-bff
  working-directory: backend
```

### Phase 3: ADR + ドキュメント更新

**作成・更新ファイル:**

| ファイル | 内容 |
|---------|------|
| `docs/70_ADR/034_DevAuthのFeatureFlag導入.md` | 技術選定の記録 |
| `docs/80_ナレッジベース/security/DevAuth.md` | 安全策セクションに feature flag を追記 |

## 検証

```bash
# Phase 1 後
cargo test --all-features               # 全テスト通過
cargo build --no-default-features        # DevAuth なしでコンパイル成功
cargo build                              # DevAuth ありでコンパイル成功
just check                               # lint + test

# Phase 2 後
just check-all                           # 全体チェック（API テスト含む）
docker build -t ringiflow-test backend/  # Docker ビルド成功
```

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | DevAuth 関連の全参照箇所（lib.rs, main.rs, config.rs, dev_auth.rs）を探索済み。Dockerfile, CI, deploy.sh, ナレッジベースも確認 |
| 2 | 曖昧さ排除 | OK | 各ファイルの変更内容をコードスニペットで明示。パニックガード削除の判断理由も記載 |
| 3 | 設計判断の完結性 | OK | default feature の是非、パニックガード削除の理由、CI 追加ステップの根拠を記載 |
| 4 | スコープ境界 | OK | 対象（BFF）と対象外（Core/Auth Service）を明記 |
| 5 | 技術的前提 | OK | `#[cfg(feature = ...)]` は Rust の標準機能。`--no-default-features` のワークスペース挙動を確認済み |
| 6 | 既存ドキュメント整合 | OK | CLAUDE.md のセキュリティ要件、既存 ADR との矛盾なし |

### ブラッシュアップループの記録

| ループ | きっかけ | 調査内容 | 結果 |
|-------|---------|---------|------|
| 1回目 | Issue 精査 → 「環境変数のみ」の前提確認 | config.rs の実装を読む | パニックガード（`#[cfg(not(debug_assertions))]`）が既に存在。Issue の前提は不正確 |
| 2回目 | パニックガード削除の是非 | feature flag とパニックガードの共存を検討 | feature enabled + release build でパニック → CI の API テストが壊れる。削除が妥当 |
| 3回目 | CI への影響 | ci.yaml の全ビルドコマンドを確認 | `--all-features` 系は変更不要。API テスト用 `cargo build --release` は default features で問題なし。本番ビルド検証ステップを追加 |
| 4回目 | Dockerfile の `cargo chef cook` への影響 | chef cook と feature flags の関係を確認 | `--no-default-features` を chef cook にも渡す必要あり（依存関係の正確なキャッシュのため） |
