# ADR-034: DevAuth の Feature Flag 導入

## ステータス

承認済み

## コンテキスト

DevAuth（開発用認証バイパス）は、フロントエンド開発を先行させるための仕組み（[DevAuth ナレッジベース](../80_ナレッジベース/security/DevAuth.md)）。

従来は環境変数 `DEV_AUTH_ENABLED` とリリースビルドでのパニックガード（`#[cfg(not(debug_assertions))]`）で制御していたが、以下の課題があった:

- **本番バイナリにコード残存**: 環境変数による実行時制御では、DevAuth のコード自体は本番バイナリに含まれる。攻撃面の不要な拡大
- **パニックガードの脆弱性**: `#[cfg(not(debug_assertions))]` はリリースビルド全般に作用するため、CI の API テスト（`cargo build --release`）でもパニックする可能性がある
- **防御の多層化不足**: セキュリティ対策は多層防御が原則。コンパイル時除外という根本的な対策が欠けていた

## 検討した選択肢

### 選択肢 1: Cargo feature flag（`dev-auth`）

`#[cfg(feature = "dev-auth")]` による条件コンパイル。`default` features に含め、本番ビルドでは `--no-default-features` で除外する。

評価:
- 利点: コンパイル時に除外されるため本番バイナリにコードが存在しない、Rust の標準的な手法、DX への影響なし（`cargo run` だけで開発可能）
- 欠点: ビルドコマンドに `--no-default-features` の指定が必要（Dockerfile, CI で管理）

### 選択肢 2: 環境変数のみ（現状維持）

実行時に `DEV_AUTH_ENABLED` で制御し、リリースビルドではパニックガードで防御。

評価:
- 利点: ビルド手順が変わらない
- 欠点: 本番バイナリにコードが残る、パニックガードと CI の `--release` ビルドが競合する可能性

### 選択肢 3: 別クレートに分離

DevAuth のコードを別の Cargo クレート（`ringiflow-dev-auth`）に切り出し、`dev-dependencies` に配置。

評価:
- 利点: コード分離が明確
- 欠点: DevAuth は BFF 起動時の初期化処理に密結合しており、クレート分離はインターフェース設計が過剰。feature flag で十分な要件に対してオーバーエンジニアリング

### 比較表

| 観点 | Feature flag | 環境変数のみ | 別クレート |
|------|-------------|-------------|-----------|
| 本番バイナリからの除外 | 完全除外 | 含まれる | 完全除外 |
| 実装コスト | 低い | なし | 高い |
| DX への影響 | なし | なし | クレート間依存の管理 |
| CI との整合性 | `--no-default-features` で明示的 | パニックガードとの競合リスク | 複雑 |

## 決定

**選択肢 1: Cargo feature flag** を採用する。

理由:
1. コンパイル時除外により、本番バイナリに DevAuth コードが一切含まれない
2. Rust エコシステムの標準的な手法であり、追加の依存やツールが不要
3. `default` features に含めることで、開発時の DX を維持できる

パニックガード（`#[cfg(not(debug_assertions))]`）は feature flag と冗長になるため削除する。feature disabled なら コード自体が存在せず、feature enabled ならデバッグ/テスト環境なのでパニックは不適切。

## 帰結

### 肯定的な影響

- 本番バイナリから DevAuth コードが完全に除外され、攻撃面が縮小する
- Dockerfile の `--no-default-features` により、本番ビルドの意図が明示的になる
- CI に本番ビルド検証ステップが追加され、feature flag の除外が壊れていないことを継続的に検証できる

### 否定的な影響・トレードオフ

- Dockerfile と CI に `--no-default-features` の管理が必要（ただし、これは本番ビルドの意図を明示化するメリットでもある）
- 新しい dev-only の依存を追加する場合、feature flag の管理を意識する必要がある

### 関連ドキュメント

- ナレッジベース: [DevAuth](../80_ナレッジベース/security/DevAuth.md)
- 実装: `backend/apps/bff/src/dev_auth.rs`
- Issue: [#288](https://github.com/ka2kama/ringiflow/issues/288)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-07 | 初版作成 |
