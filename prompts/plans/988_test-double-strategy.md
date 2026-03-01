# 計画: #988 テストダブル戦略の明文化と命名不整合の修正

## コンテキスト

テスト戦略に 3 つの歪みがある:

1. ADR-036 は「Handler 層にユニットテストは追加しない」と決定しているが、実際には多数のハンドラテストが存在する
2. `mock.rs` の実装は Fake（インメモリ実装）だが Mock と命名されている
3. テスト学派（古典学派 = 状態検証）が暗黙知のまま

## スコープ

対象:

- ADR-036 の改訂
- `backend/crates/infra/src/mock.rs` → `fake.rs` リネーム + `Mock*` → `Fake*`
- `backend/crates/infra/src/deletion/registry.rs` のローカル `MockDeleter` → `FakeDeleter`
- 全参照箇所（import, 使用箇所）の更新
- TDD 手順書・テスト戦略概要・基本設計書のドキュメント更新

対象外:

- `handler/auth/tests.rs` の `Stub*` 命名 — 正しい命名のため変更なし
- セッションログ・計画ファイル（`prompts/runs/`, `prompts/plans/`）— 過去の記録は歴史的文書として保持
- 実装解説（`docs/90_実装解説/`）— 過去の PR の記録であり、当時の命名が正確

## 実装計画

### Phase 1: ADR-036 改訂

確認事項:
- 型: なし
- パターン: 既存 ADR の形式 → `docs/70_ADR/036_Handler層テスト戦略.md`
- ライブラリ: なし

操作パス: 該当なし（ドキュメント修正）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

内容:

- ステータス: 「承認済み」→「改訂済み」
- 改訂理由: Handler が「薄いプロキシ」の前提が変化（ビジネスロジックを含む Handler が増加）
- 新しい方針:
  - Handler にビジネスロジックがある場合はハンドラテストを追加する
  - テストダブルは Fake（インメモリ実装 + 状態検証）を基本とする
  - テスト学派: 古典学派（Detroit School）を採用。状態検証が基本、相互作用検証は使用しない
- テストダブルの使い分け基準を追記:

| 種類 | 用途 | 例 |
|------|------|-----|
| Stub | 固定値を返す。テスト対象の依存を単純化 | `StubUserRepository`（handler テスト） |
| Fake | インメモリ実装。状態を持ち、簡易的だが動作する | `FakeWorkflowInstanceRepository`（usecase テスト） |
| Mock | 相互作用検証（呼び出し回数・引数の検証）。原則不使用 | — |

対象ファイル: `docs/70_ADR/036_Handler層テスト戦略.md`

### Phase 2: コードリネーム（mock.rs → fake.rs）

確認事項:
- 型: `Mock*` 構造体の全リスト → `backend/crates/infra/src/mock.rs`
- パターン: `pub mod mock` 宣言 → `backend/crates/infra/src/lib.rs`
- ライブラリ: なし

操作パス: 該当なし（リファクタリング）

テストリスト:
ユニットテスト:
- [ ] `just check` が通る（既存テスト全通過 = リネームの正しさを検証）

ハンドラテスト（該当なし — 既存テストの通過で検証）
API テスト（該当なし）
E2E テスト（該当なし）

手順:

1. `backend/crates/infra/src/mock.rs` → `backend/crates/infra/src/fake.rs` にリネーム（`git mv`）
2. `fake.rs` 内の全 `Mock*` → `Fake*` に置換（10 構造体）
3. `fake.rs` 内のドキュメントコメント更新
4. `backend/crates/infra/src/lib.rs` の `pub mod mock` → `pub mod fake`
5. 全参照ファイルの import と使用箇所を更新

対象ファイル（Rust ソース 15 ファイル）:

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/infra/src/mock.rs` → `fake.rs` | ファイルリネーム + 全構造体名変更 |
| `backend/crates/infra/src/lib.rs` | `pub mod mock` → `pub mod fake` |
| `backend/crates/infra/src/deletion/registry.rs` | ローカル `MockDeleter` → `FakeDeleter` |
| `backend/apps/core-service/src/test_utils/workflow_test_builder.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/workflow/command.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/workflow/command/comment.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/workflow/query.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/task.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/folder.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/dashboard.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/workflow_definition.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/usecase/notification/service.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/handler/folder.rs` | import + 使用箇所 |
| `backend/apps/core-service/src/handler/workflow_definition.rs` | import + 使用箇所 |
| `backend/apps/core-service/tests/workflow_definition_integration_test.rs` | import + 使用箇所 |

### Phase 3: ドキュメント更新

確認事項:
- パターン: 各ドキュメントの該当箇所 → Phase 1 探索で特定済み

操作パス: 該当なし（ドキュメント修正）

テストリスト:
ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

対象ファイル:

| ファイル | 変更内容 |
|---------|---------|
| `docs/60_手順書/04_開発フロー/02_TDD開発フロー.md` | テストダブル表の説明更新、コード例の `MockUserRepository` → `FakeUserRepository` |
| `docs/50_テスト/00_テスト戦略概要.md` | ADR-036 の説明を現状に合わせて更新 |
| `docs/30_基本設計書/02_プロジェクト構造設計.md` | ファイルツリーの `mock.rs` → `fake.rs` |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `deletion/registry.rs` にローカル `MockDeleter` が存在 | 既存手段の見落とし | Phase 2 の対象に追加 |
| 2回目 | セッションログ・計画ファイルの Mock 参照をどうするか未定 | スコープ境界 | 過去の記録は歴史的文書として対象外に明記 |
| 3回目 | handler/auth/tests.rs の `Stub*` は正しい命名 | 既存手段の見落とし | 対象外に明記（変更不要） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | Explore エージェントで Rust ソース 15 ファイル + ドキュメント 3 ファイルを特定済み |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | リネーム対象の構造体名・ファイルパスがすべて確定 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | セッションログの扱い、Stub* の扱い、MockDeleter の扱いを決定済み |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象外セクションで明示 |
| 5 | 技術的前提 | 前提が考慮されている | OK | `git mv` でリネーム、`replace_all` で一括置換 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | ADR-036 の改訂で整合を回復する |
