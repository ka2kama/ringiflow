# Phase A-2: 採番サービス

## 概要

表示用 ID のバックエンド実装。ドメイン層に値オブジェクト群（`DisplayId`, `DisplayIdEntityType`, `display_prefix`）を追加し、infra 層に `DisplayIdCounterRepository` を実装した。ワークフロー作成時のタイムスタンプ暫定採番をカウンター採番に置き換えた。

### 対応 Issue

[#206 表示用 ID バックエンド実装](https://github.com/ka2kama/ringiflow/issues/206)
PR: [#218](https://github.com/ka2kama/ringiflow/pull/218)

### 設計書との対応

- [表示用 ID 設計 > 表示用 ID 値オブジェクト](../../03_詳細設計書/12_表示用ID設計.md#表示用-id-値オブジェクト)
- [表示用 ID 設計 > 採番サービス](../../03_詳細設計書/12_表示用ID設計.md#採番サービス)

## 実装したコンポーネント

| ファイル | 責務 |
|---------|------|
| [`backend/crates/domain/src/value_objects.rs`](../../../backend/crates/domain/src/value_objects.rs) | `DisplayId`, `DisplayIdEntityType`, `display_prefix` 定数 |
| [`backend/crates/infra/src/repository/display_id_counter_repository.rs`](../../../backend/crates/infra/src/repository/display_id_counter_repository.rs) | `SELECT FOR UPDATE` による排他的採番 |
| [`backend/apps/core-service/src/usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs) | `WorkflowUseCaseImpl` に型パラメータ `C` を追加 |

## 実装内容

### DisplayId 値オブジェクト

```rust
pub struct DisplayId {
    prefix: &'static str,
    number: DisplayNumber,
}

impl fmt::Display for DisplayId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.prefix, self.number)
    }
}
```

`DisplayId` はプレフィックス（`"WF"` 等）と `DisplayNumber`（連番）を組み合わせて `"WF-42"` 形式の文字列を生成する。ドメイン層では構造体として保持し、文字列化は `to_string()` 呼び出し時に行う。

### DisplayIdEntityType

```rust
pub enum DisplayIdEntityType {
    WorkflowInstance,
    WorkflowStep,
}
```

採番対象のエンティティ種別を表す列挙型。DB のカウンターテーブルのキーとして使用する。

### display_prefix 定数

```rust
pub mod display_prefix {
    pub const WORKFLOW_INSTANCE: &str = "WF";
    pub const WORKFLOW_STEP: &str = "ST";
}
```

Phase B で `WorkflowStep` にも表示用 ID を付与する際に使用するプレフィックス定数。

### DisplayIdCounterRepository

`SELECT FOR UPDATE` による排他的採番を実装。トランザクション内で呼び出すことで、同一エンティティ型の連番が重複しないことを保証する。

### WorkflowUseCaseImpl の型パラメータ追加

`WorkflowUseCaseImpl` に `DisplayIdCounterRepository` の型パラメータ `C` を追加し、DI 配線を更新。`create_workflow()` 内でカウンターリポジトリから `next_number()` を取得し、`WorkflowInstance::new()` に渡す。

## テスト

| テスト | 場所 | 内容 |
|--------|------|------|
| `test_初回採番で1を返す` | `backend/crates/infra/tests/display_id_counter_repository_test.rs` | 新規エンティティ型の初回採番 |
| `test_連続採番で連番を返す` | 同上 | 連続呼び出しで 1, 2, 3... |
| `test_異なるエンティティ型は独立して採番される` | 同上 | テナント＋エンティティ型ごとに独立 |
| `test_create_workflow_正常系` | `backend/apps/core-service/src/usecase/workflow.rs` | ワークフロー作成で `display_number` が設定される |

## 関連ドキュメント

- [Phase A-1: DB スキーマ変更](./01_PhaseA1_DBスキーマ変更.md)
- [Phase A-3: API + フロントエンド](./03_PhaseA3_APIとフロントエンド.md)
- [表示用 ID 設計](../../03_詳細設計書/12_表示用ID設計.md)

---

## 設計解説

### 1. DisplayId を構造体として定義し、Display トレイトで文字列化

場所: [`backend/crates/domain/src/value_objects.rs`](../../../backend/crates/domain/src/value_objects.rs)

なぜこの設計か: `String` で `"WF-42"` を直接保持する代わりに、プレフィックスと番号を分離して保持する。これにより、プレフィックスの変更やフォーマットの変更に型安全に対応できる。

代替案:
- `String` 型で保持: シンプルだが、不正な形式の値を許容してしまう。パース処理も必要になる
- `DisplayId` をドメインモデルのフィールドにする: ドメインモデルにプレゼンテーション層の関心が混入する。Phase A-3 で DTO 変換時に文字列化する方が責務分離の観点で適切

### 2. SELECT FOR UPDATE による排他的採番

場所: [`backend/crates/infra/src/repository/display_id_counter_repository.rs`](../../../backend/crates/infra/src/repository/display_id_counter_repository.rs)

なぜこの設計か: テナント × エンティティ種別ごとに一意な連番を保証する必要がある。`SELECT FOR UPDATE` で行ロックを取得し、カウンターをインクリメントして返すことで、同時リクエストでも連番の重複を防ぐ。

代替案:
- PostgreSQL の `SEQUENCE`: テナント分離が難しい。テナントごとに SEQUENCE を動的に作成する必要があり、管理が複雑
- アプリケーション層でのロック: Redis 等の分散ロックが必要になり、インフラ依存が増える
- UUID ベースの短縮 ID: 連番の「順序」が失われ、ユーザーにとっての可読性が低下する

### 3. WorkflowUseCaseImpl に型パラメータ C を追加

場所: [`backend/apps/core-service/src/usecase/workflow.rs`](../../../backend/apps/core-service/src/usecase/workflow.rs)

なぜこの設計か: 既存のジェネリック型パラメータパターン（`W: WorkflowInstanceRepository`, `S: WorkflowStepRepository` 等）に倣い、`C: DisplayIdCounterRepository` を追加。テスト時にモック差し替えが可能な DI 構造を維持する。

代替案:
- `Arc<dyn DisplayIdCounterRepository>` によるダイナミックディスパッチ: 型パラメータの増加を避けられるが、既存パターンとの一貫性が失われる
