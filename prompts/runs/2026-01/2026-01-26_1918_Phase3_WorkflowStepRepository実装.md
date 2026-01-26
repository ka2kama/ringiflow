# Phase 3: WorkflowStepRepository 実装

## セッション情報

- 日時: 2026-01-26 19:18-19:34
- 対応 Issue: [#35 ワークフロー申請機能](https://github.com/ka2kama/ringiflow/issues/35)
- 対応 PR: [#114](https://github.com/ka2kama/ringiflow/pull/114)
- コミット: 157e575, a69f7d0, 96262e9

## 実施内容

### 1. Phase 1-2 実装解説ドキュメント作成（19:18）

Phase 1-2 の実装解説ドキュメントが未作成だったため、以下を作成:

- `docs/07_実装解説/04_ワークフロー申請機能/00_概要.md`
- `docs/07_実装解説/04_ワークフロー申請機能/01_Phase1_WorkflowDefinitionRepository.md`
- `docs/07_実装解説/04_ワークフロー申請機能/02_Phase2_WorkflowInstanceRepository.md`

各 Phase の設計判断、代替案、トレードオフを文書化。

### 2. Phase 3: WorkflowStepRepository 実装（19:24）

ワークフローステップ（承認タスク）の永続化リポジトリを実装:

**実装内容:**
- `WorkflowStepRepository` トレイト定義
- `PostgresWorkflowStepRepository` 実装
- 統合テスト 9 件（tests/ 配置）
- SQLx クエリキャッシュ更新

**メソッド:**
- `save`: ステップの保存（UPSERT）
- `find_by_id`: ID でステップを検索
- `find_by_instance`: インスタンスのステップ一覧を取得
- `find_by_assigned_to`: 担当者のタスク一覧を取得

**プロセス適用:**
Phase 1-2 で確立したプロセスを適用:
1. テストを tests/ に配置
2. `#[sqlx::test(migrations = "../../migrations")]` 使用
3. `just sqlx-prepare` でキャッシュ更新
4. `just pre-commit` で全体チェック

**結果:** 一発で CI をパス ✅

### 3. Phase 3 実装解説ドキュメント作成（19:26）

`docs/07_実装解説/04_ワークフロー申請機能/03_Phase3_WorkflowStepRepository.md` を作成。

設計判断の文書化:
- JOIN によるテナント分離
- 担当者検索の部分インデックス活用
- UPSERT の更新対象フィールド選定
- Phase 1-2 プロセスの適用成果

### 4. 進捗更新

- Issue #35: Phase 進捗セクションを追加
- PR #114: Phase 1-3 完了をチェック

## 主な設計判断

### 1. JOIN によるテナント分離

**判断:** workflow_steps テーブルに tenant_id カラムを持たせず、workflow_instances との JOIN でテナント分離を実現。

**理由:**
- 正規化を維持（テナント情報は workflow_instances で管理）
- 多層防御（DB スキーマレベル + クエリレベル）
- データ不整合のリスク回避

**代替案:**
- workflow_steps に tenant_id を追加 → 正規化違反、冗長性
- アプリケーション層でチェック → DB 側でガードなし、リスク高

**実装例:**
```rust
async fn find_by_id(...) -> Result<Option<WorkflowStep>, InfraError> {
    sqlx::query!(
        r#"
        SELECT s.*
        FROM workflow_steps s
        INNER JOIN workflow_instances i ON s.instance_id = i.id
        WHERE s.id = $1 AND i.tenant_id = $2
        "#,
        // ...
    )
    // ...
}
```

### 2. 部分インデックスによる最適化

**判断:** 担当者検索で `WHERE status = 'active'` の部分インデックスを使用。

**理由:**
- 未完了のアクティブタスクのみインデックス対象
- 完了済みステップを除外して検索を高速化
- DB 容量の節約

**DB 設計:**
```sql
CREATE INDEX workflow_steps_assigned_to_idx
ON workflow_steps(assigned_to)
WHERE status = 'active';
```

### 3. UPSERT の更新対象選定

**判断:** 状態遷移で変化するフィールドのみを更新対象にする。

**更新するフィールド:**
- status, decision, comment
- started_at, completed_at
- updated_at

**更新しないフィールド:**
- 識別子（id, instance_id, step_id, step_name, step_type）
- assigned_to, due_date（MVP では再割り当て機能なし）
- created_at（不変）

**理由:**
- 必要最小限の更新
- 不変フィールドの保護
- 意図しない上書きの防止

## 学んだこと

### プロセスの効果

Phase 1 では 3 回 CI に失敗したが、Phase 3 では一発で成功。

**成功要因:**
1. `.claude/rules/repository.md` にプロセスを明記
2. `justfile` に標準コマンドを集約（`sqlx-prepare`, `pre-commit`）
3. AI エージェントがルールに自動的に従う

**効果:**
- 人的ミスの構造的防止
- 新メンバーの学習コスト削減
- 一貫した品質の維持

### JOIN によるテナント分離パターン

子テーブル（workflow_steps）に親テーブル（workflow_instances）のテナント情報を持たせず、JOIN で分離するパターンは：

**メリット:**
- 正規化を維持
- データ不整合のリスクなし
- 多層防御

**デメリット:**
- クエリに JOIN が必須
- わずかなオーバーヘッド

**判断:** メリットがデメリットを大きく上回る。

## 成果物

### コード

| ファイル | 行数 | 内容 |
|---------|------|------|
| `workflow_step_repository.rs` | 259 | トレイト + PostgreSQL 実装 |
| `workflow_step_repository_test.rs` | 354 | 統合テスト 9 件 |
| `.sqlx/*.json` | - | クエリキャッシュ 4 件 |

### ドキュメント

| ファイル | 内容 |
|---------|------|
| `00_概要.md` | Phase 構成と学習ポイント |
| `01_Phase1_WorkflowDefinitionRepository.md` | Phase 1 の設計解説 |
| `02_Phase2_WorkflowInstanceRepository.md` | Phase 2 の設計解説 |
| `03_Phase3_WorkflowStepRepository.md` | Phase 3 の設計解説 |

### CI 結果

| ジョブ | 結果 |
|-------|------|
| Rust | ✅ SUCCESS |
| Rust Integration | ✅ SUCCESS |
| API Test | ✅ SUCCESS |

## 次のステップ

### Phase 4: ワークフロー作成ユースケース

**実装対象:**
- ユースケース層の実装開始
- ワークフローインスタンスの作成ロジック
- ビジネスルールの適用

**進捗:**
- Phase 1-3: リポジトリ層 ✅ 完了
- Phase 4-7: ユースケース層・API 層 🚧 未着手

## 振り返り

### うまくいったこと

1. **プロセスの適用**: Phase 1-2 のルール化により、Phase 3 は一発で CI をパス
2. **設計判断の文書化**: 各 Phase の「なぜ」を記録し、形式知化
3. **進捗の可視化**: Issue/PR のチェックボックスで進捗を明確化

### 改善点

1. **テストプランの扱い**: PR のテストプランは最終確認時にチェックすべき（途中でチェックしていた）
2. **ドキュメント作成タイミング**: Phase 1-2 のドキュメントが後回しになっていた

### 次回への示唆

- Phase ごとにドキュメントを即座に作成する習慣を維持
- PR のテストプランは全 Phase 完了まで未チェックを保つ
