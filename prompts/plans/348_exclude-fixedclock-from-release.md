# #348 FixedClock をリリースビルドから除外する

## Context

#338 で `Clock` trait と `FixedClock` を `domain` クレートに導入した。`FixedClock` はテスト専用だが `#[cfg(test)]` なしで定義されており、リリースビルドに含まれている。テスト専用コードをリリースから除外する。

## 設計判断: feature flag vs テスト専用クレート

### 選択肢

| 方法 | 概要 |
|------|------|
| A. feature flag（採用） | domain クレートに `test-support` feature を追加し、`FixedClock` を条件付きコンパイルにする |
| B. テスト専用クレート（不採用） | `backend/crates/test-support/` を新規作成し、`FixedClock` をそこに移動する |

### 判断: A を採用

理由:
- **凝集度**: `FixedClock` は `Clock` trait のテストダブル。trait の隣に置くのが自然
- **KISS/YAGNI**: 構造体1つのために新クレートは過剰。将来テストユーティリティが増えたら分離すればよい
- **Rust エコシステムの慣例**: `tokio` の `test-util`、`sqlx` の `runtime-tokio` など、feature flag でテスト用コードをゲートするパターンは広く使われている

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/Cargo.toml` | `[features] test-support = []` を追加 |
| `backend/crates/domain/src/clock.rs` | `FixedClock` に `#[cfg(any(test, feature = "test-support"))]` を付与 |
| `backend/apps/core-service/Cargo.toml` | `[dev-dependencies]` に `ringiflow-domain` を features 付きで追加 |

## 実装計画

### Phase 1: feature flag 導入と FixedClock の条件付きコンパイル

#### 確認事項
- パターン: workspace dev-dependencies での features 指定方法 → Cargo ドキュメント
- パターン: `#[cfg(any(test, feature = "..."))]` の構文 → 既知

#### 変更内容

1. `backend/crates/domain/Cargo.toml` に features セクションを追加:

```toml
[features]
test-support = []
```

2. `backend/crates/domain/src/clock.rs` の `FixedClock` 関連コードに cfg ゲートを付与:

```rust
/// 固定時刻を返すテスト用実装
#[cfg(any(test, feature = "test-support"))]
pub struct FixedClock {
    now: DateTime<Utc>,
}

#[cfg(any(test, feature = "test-support"))]
impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}
```

3. `backend/apps/core-service/Cargo.toml` の `[dev-dependencies]` に追加:

```toml
ringiflow-domain = { workspace = true, features = ["test-support"] }
```

#### cfg の動作確認

| ビルド | test | test-support | FixedClock |
|--------|------|-------------|------------|
| `cargo build`（リリース含む） | false | false | 除外 ✅ |
| `cargo test -p ringiflow-domain` | true | false | 含む ✅ |
| `cargo test -p ringiflow-core-service` | false（domain 側） | true | 含む ✅ |

#### テストリスト

- [ ] `cargo test -p ringiflow-domain` — domain の clock テスト3件がパスする
- [ ] `cargo test -p ringiflow-core-service` — workflow テスト12件がパスする
- [ ] `just check-all` — 全テスト・lint がパスする

## 検証

```bash
# 1. リリースビルドから除外されていることを確認
# FixedClock のシンボルがバイナリに含まれないことを検証
cargo build --release -p ringiflow-core-service 2>&1
nm target/release/ringiflow-core-service 2>/dev/null | grep -i fixedclock || echo "FixedClock not found in binary ✅"

# 2. テストがすべてパスすることを確認
just check-all
```

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `#[cfg]` を struct / impl FixedClock / impl Clock for FixedClock の3箇所に個別に付ける必要がある | 未定義 | 3つの `#[cfg]` を明示的に記載 |
| 2回目 | ギャップなし | — | — |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | FixedClock の定義・impl・使用箇所（domain テスト + core-service テスト）を全て確認。変更は3ファイルで完結 |
| 2 | 曖昧さ排除 | OK | cfg 条件、変更ファイル、変更内容を具体的に記載 |
| 3 | 設計判断の完結性 | OK | feature flag vs テスト専用クレートの判断理由を記載 |
| 4 | スコープ境界 | OK | 対象: FixedClock の条件付きコンパイル化。対象外: Clock trait / SystemClock は変更なし |
| 5 | 技術的前提 | OK | Cargo の workspace dev-dependencies features 指定、cfg(any()) 構文は既知 |
| 6 | 既存ドキュメント整合 | OK | クレート追加/削除ではないため基本設計書の更新は不要 |
