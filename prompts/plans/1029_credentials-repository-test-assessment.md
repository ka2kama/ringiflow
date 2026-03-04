# CredentialsRepository テスト充足性評価と補完

## コンテキスト

### 目的
- Issue: #1029
- Want: セキュリティクリティカルな CredentialsRepository の各メソッドが十分にテストされていることを保証する
- 完了基準:
  - 既存テストによるカバレッジを評価し、結果を記録する
  - 評価の結果、不足があれば専用テストを追加する（不足がなければその根拠を記録して完了）

### ブランチ / PR
- ブランチ: `feature/1029-credentials-repository-test-assessment`
- PR: #1030（Draft）

### As-Is（探索結果の要約）

CredentialsRepository のトレイト（5 メソッド）:
- `find_by_user_and_type(tenant_id, user_id, credential_type) -> Result<Option<Credential>>`
- `create(user_id, tenant_id, credential_type, credential_data) -> Result<Uuid>`
- `delete_by_user(tenant_id, user_id) -> Result<()>`
- `delete_by_tenant(tenant_id) -> Result<()>`
- `update_last_used(id) -> Result<()>`

実装: `PostgresCredentialsRepository`（`backend/crates/infra/src/repository/credentials_repository.rs`）

テストカバレッジの現状:

| メソッド | 実 DB テスト | スタブテスト | API テスト |
|---------|------------|------------|-----------|
| `find_by_user_and_type` | ❌ | ✅ UseCase | ✅ Hurl login |
| `create` | ❌ | ✅ UseCase | ✅ Hurl seed |
| `delete_by_user` | ❌ | ✅ UseCase | ❌ |
| `delete_by_tenant` | ✅ deleter_test | — | — |
| `update_last_used` | ❌ | ✅ UseCase | ❌ |

既存テスト: `credentials_repository.rs` 内に `CredentialType` 変換テストと `Send + Sync` テストのみ。

テストパターン: 他リポジトリは `backend/crates/infra/tests/{name}_repository_test.rs` に `#[sqlx::test]` による統合テストを配置。`setup_test_data()` でテナント+ユーザーを作成、`sut` 命名規約。

### 進捗
- [x] Phase 1: カバレッジ評価の記録
- [x] Phase 2: 専用統合テストの実装

## 評価結果

不足あり。以下の理由から専用テストが必要:

1. テナント分離の検証不足: `find_by_user_and_type`, `create`, `delete_by_user` で別テナントのデータにアクセスできないことを検証するテストがない
2. SQL クエリの直接検証なし: フィールドマッピング（`CredentialType` の変換、`is_active`、`last_used_at` 等）が実 DB で正しく動作するかの検証がない
3. プロジェクトの一貫性: 他 12 リポジトリすべてに専用テストがあり、CredentialsRepository だけ欠落

## スコープ

対象:
- `credentials_repository_test.rs` の新規作成（`find_by_user_and_type`, `create`, `delete_by_user`, `update_last_used` の 4 メソッド）
- 評価結果の Issue コメントへの記録

対象外:
- `delete_by_tenant`: 既に `postgres_deleter_test.rs` で十分にテスト済み
- UseCase 層・Handler 層のテスト追加（既存テストで十分）
- `CredentialType` のパース/変換テスト（既存の `#[cfg(test)]` で十分）

## Phase 1: カバレッジ評価の記録

Issue コメントとして評価結果を記録する。

確認事項: なし（探索結果に基づく文書作成のみ）

操作パス: 該当なし（ドキュメント記録のみ）

テストリスト: 該当なし（ドキュメント記録のみ）

## Phase 2: 専用統合テストの実装

`backend/crates/infra/tests/credentials_repository_test.rs` を新規作成する。

### 確認事項
- 型: `Credential` 構造体のフィールド → `credentials_repository.rs`
- 型: `CredentialType` enum のバリアント → `credentials_repository.rs`
- パターン: 他リポジトリテストの構造 → `user_repository_test.rs`
- パターン: `setup_test_data()` ヘルパー → `tests/common/mod.rs`
- パターン: テナント分離テストのパターン → `user_repository_test.rs`（`別テナントのユーザーは取得できない`）
- ライブラリ: `sqlx::test` マクロの使用 → 既存テストで確認済み

### 設計判断

テストデータの作成方法:
- credentials の挿入は `PostgresCredentialsRepository::create` を使用する（正常系テスト後はリポジトリメソッド自体が検証済みになるため）
- 初回の `create` テストのみ、作成後の検証に raw SQL を使用して create メソッドの正しさを独立に確認する
- テナント・ユーザーは `setup_test_data()` ヘルパーを使用

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | credential を作成し、作成された credential が DB に正しく保存されている | 正常系 | ユニット（統合） |
| 2 | tenant_id + user_id + credential_type で credential を取得できる | 正常系 | ユニット（統合） |
| 3 | 取得した credential の全フィールドが正しい | 正常系 | ユニット（統合） |
| 4 | 存在しない credential を検索すると None が返る | 準正常系 | ユニット（統合） |
| 5 | 別テナントの credential は取得できない | 準正常系 | ユニット（統合） |
| 6 | 異なる credential_type では取得できない | 準正常系 | ユニット（統合） |
| 7 | ユーザーの全 credentials を削除できる | 正常系 | ユニット（統合） |
| 8 | 別テナントの credentials は削除されない | 準正常系 | ユニット（統合） |
| 9 | credentials がないユーザーの削除はエラーにならない | 準正常系 | ユニット（統合） |
| 10 | last_used_at を更新できる | 正常系 | ユニット（統合） |

### テストリスト

ユニットテスト（`#[sqlx::test]` 統合テスト）:

create:
- [ ] credential を作成すると UUID が返り、DB に正しく保存されている

find_by_user_and_type:
- [ ] 作成済み credential をテナント・ユーザー・種別で取得できる
- [ ] 取得した credential の全フィールド（id, user_id, tenant_id, credential_type, credential_data, is_active, last_used_at, created_at, updated_at）が正しい
- [ ] 存在しない credential を検索すると None が返る
- [ ] 別テナントの credential は取得できない（テナント分離）
- [ ] 異なる credential_type では取得できない

delete_by_user:
- [ ] ユーザーの全 credentials を削除できる
- [ ] 別テナントの credentials は削除されない（テナント分離）
- [ ] credentials がないユーザーの削除はエラーにならない

update_last_used:
- [ ] last_used_at が更新される

ハンドラテスト: 該当なし（Repository 層のテスト）
API テスト: 該当なし（Repository 層のテスト）
E2E テスト: 該当なし（Repository 層のテスト）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `delete_by_tenant` を対象に含めていた | 既存手段の見落とし | `postgres_deleter_test.rs` で十分テスト済みのため対象外に移動 |
| 2回目 | create テストでのデータ検証方法が未定義 | 曖昧 | 初回 create は raw SQL で検証、以降はリポジトリメソッドを使用する方針を設計判断に追記 |
| 3回目 | `update_last_used` の非存在 ID ケースを含めるべきか | 不完全なパス | PostgreSQL の UPDATE は存在しない行を更新しても成功（affected rows = 0）するため、Repository 層で特別なハンドリングはない。UseCase 層で `find` → `update` の順で呼ぶため、Repository 単体で非存在テストを追加する意義は低い。スコープ外とする |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象メソッドが計画に含まれている | OK | 5 メソッド中 4 メソッドが対象、1 メソッド（delete_by_tenant）は理由付きで対象外 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | テストデータ作成方法、各テストの検証内容が具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | テストデータ作成方法の判断を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象 4 メソッド、対象外 3 項目を明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `#[sqlx::test]` のトランザクション自動ロールバック、`setup_test_data()` の存在を確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | テストパターンは他リポジトリテストと一貫 |
