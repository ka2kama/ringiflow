# 実装計画: #528 ドメイン層のクローン削減・ファイルサイズ削減

## コンテキスト

Epic #467 の Story 5。jscpd で検出されるドメイン層の15クローン削減と5ファイルのサイズ超過解消が目標。

## スコープ

### 対象

1. **UUID ID 型マクロ（`define_uuid_id!`）**: definition.rs/comment.rs ↔ step.rs のボイラープレート重複 2 クローンを解消。他の同一パターン（TenantId, UserId, RoleId, WorkflowInstanceId）にも適用
2. **StringNewtype マクロ（`define_validated_string!`）**: value_objects.rs 内部 + user.rs との重複 2 クローンを解消。UserName, WorkflowName, TenantName の 3 型に適用
3. **step.rs テストヘルパー**: step.rs 内部の 9 クローン（テスト Record 構築）を削減
4. **instance.rs テストヘルパー**: instance.rs テスト内の同一パターンを削減（ファイルサイズ対策）
5. **role.rs テスト確認 + jscpd 検証**: role.rs 内部の 2 クローンを確認し、必要に応じて対応

### 対象外

- **Getter のマクロ化**: proc macro が必要で複雑度が過大。KISS 原則に基づきスコープ外
- **Email のマクロ化**: trim なし + `@` 構造検証 + `value.len()`（`chars().count()` でない）と差異が 3 つあり、マクロパラメータが複雑になりすぎる
- **テストの外部ファイル化**: ディレクトリ構造の変更（`step.rs` → `step/mod.rs`）を伴い、テストヘルパーで十分な削減が得られるため見送り
- **new()/from_db() の共通化**: 各エンティティのフィールドが異なり、トレイト化しても各型で個別 impl が必要。コード量の削減にならない

## 設計判断

### 判断 1: UUID ID 型 — マクロ vs トレイト

**選択: 宣言型マクロ（`macro_rules!`）**

理由:
- derive 属性（`#[derive(Debug, Clone, ...)]`）と `#[display("{_0}")]` をトレイトでは共通化できない
- 全 7 箇所が型名のみ異なる完全同一パターン
- Rust の標準的なボイラープレート削減手法

配置先: `backend/crates/domain/src/macros.rs`（domain crate 内）。shared crate ではなく domain に置く理由: `Uuid::now_v7()`, `derive_more::Display`, `serde` 等のドメイン固有依存があるため。

### 判断 2: StringNewtype — マクロ

**選択: 宣言型マクロ（`macro_rules!`）**

理由:
- UserName/WorkflowName/TenantName は `label` と `max_length` のみ異なる完全同一パターン
- Display impl は `std::fmt::Display` を手動生成で統一（derive_more を使う TenantName と手動 impl の UserName/WorkflowName の差異を吸収）

### 判断 3: テスト Record 構築 — ヘルパー関数 + 構造体更新構文

**選択: テストモジュール内のローカルヘルパー関数**

```rust
// テストモジュール内
fn record_from(step: &WorkflowStep) -> WorkflowStepRecord {
    WorkflowStepRecord {
        id: step.id().clone(),
        // ... 全フィールドを getter から構築
    }
}

// 使用: 差異のあるフィールドだけ指定し、残りは ..record_from(&before)
let expected = WorkflowStep::from_db(WorkflowStepRecord {
    status: WorkflowStepStatus::Completed,
    decision: Some(StepDecision::Approved),
    version: before.version().next(),
    completed_at: Some(now),
    updated_at: now,
    ..record_from(&before)
});
```

理由: Rust の構造体更新構文 (`..base`) がまさにこの用途に最適。外部に公開不要なテスト内ローカル関数で十分。

### 判断 4: role.rs テスト — Phase 5 で確認後判断

role.rs の 2 クローンは rstest `#[case]` の肯定テスト/否定テストの構造重複。テスト可読性を損なわない範囲でヘルパー抽出を検討。jscpd 実行結果を見て判断する。

## Phase 分割

### Phase 1: UUID ID 型マクロの導入

`define_uuid_id!` マクロを作成し、7 箇所の UUID ID 型定義を置換する。

変更ファイル:
- `backend/crates/domain/src/macros.rs`（新規）
- `backend/crates/domain/src/lib.rs`（macros モジュール追加）
- `backend/crates/domain/src/tenant.rs`（TenantId）
- `backend/crates/domain/src/user.rs`（UserId — Default impl も追加される）
- `backend/crates/domain/src/role.rs`（RoleId）
- `backend/crates/domain/src/workflow/instance.rs`（WorkflowInstanceId）
- `backend/crates/domain/src/workflow/definition.rs`（WorkflowDefinitionId）
- `backend/crates/domain/src/workflow/step.rs`（WorkflowStepId）
- `backend/crates/domain/src/workflow/comment.rs`（WorkflowCommentId）

#### 確認事項
- [x] 型: 各 ID 型の derive 属性が全て同一か → 7 箇所確認、UserId のみ Default なし。マクロで Default を統一的に生成
- [x] パターン: `derive_more::Display` が `#[derive(derive_more::Display)]` 形式でマクロ内で使えるか → Grep で既存使用 7 箇所確認、問題なし
- [x] ライブラリ: `macro_rules!` 内で `$crate` パスは使えないが外部クレートパスは使える → `$crate::DomainError` で解決、`#[macro_use] mod macros;` で配置

#### テストリスト

ユニットテスト:
- [x] マクロで定義した型の `new()` が UUID v7 を返す
- [x] `from_uuid()` / `as_uuid()` が往復する
- [x] `Default::default()` が動作する
- [x] `Display` が UUID 文字列を返す
- [x] `Serialize`/`Deserialize` が動作する
- [x] 既存テストが全てパスする（リグレッション確認）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 2: StringNewtype マクロの導入

`define_validated_string!` マクロを作成し、UserName/WorkflowName/TenantName の 3 型を置換する。

変更ファイル:
- `backend/crates/domain/src/macros.rs`（マクロ追加）
- `backend/crates/domain/src/value_objects.rs`（UserName, WorkflowName）
- `backend/crates/domain/src/tenant.rs`（TenantName）

#### 確認事項
- [x] 型: TenantName は `derive_more::Display` 使用、UserName/WorkflowName は手動 Display impl → マクロでは手動 Display 生成で統一。機能的に等価
- [x] パターン: `$crate::DomainError` がマクロ展開先で正しく解決されるか → `#[macro_use] mod macros;` で宣言し、`$crate::DomainError` で正しく解決
- [x] ライブラリ: `format!` マクロのエラーメッセージが既存テストの期待値と一致するか → 既存テスト全件パスで確認

#### テストリスト

ユニットテスト:
- [x] マクロで定義した型の `new()` が正常値を受け入れる
- [x] 空文字列を拒否する
- [x] 空白のみを拒否する（trim 後に空）
- [x] 最大長を受け入れる
- [x] 最大長+1 を拒否する
- [x] `as_str()` / `into_string()` が正しい値を返す
- [x] `Display` が内部文字列を表示する
- [x] 既存テストが全てパスする（リグレッション確認）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 3: step.rs テストヘルパーの導入

step.rs テスト内に `record_from` ヘルパー関数を導入し、9 クローンの Record 構築を簡素化する。

変更ファイル:
- `backend/crates/domain/src/workflow/step.rs`（テストモジュール内）

#### 確認事項
- [x] パターン: WorkflowStepRecord の全フィールドと対応する getter を確認 → 全 13 フィールドに対応する getter あり
- [x] 型: 構造体更新構文 `..record_from(&before)` が WorkflowStepRecord で動作するか → 全フィールドが指定され問題なし

#### テストリスト

ユニットテスト:
- [x] 既存テスト 18 件が全てパスする（リファクタリングのみ、新規テストなし）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 4: instance.rs テストヘルパーの導入

instance.rs テスト内に `record_from` ヘルパー関数を導入し、Record 構築の冗長性を削減する。

変更ファイル:
- `backend/crates/domain/src/workflow/instance.rs`（テストモジュール内）

#### 確認事項
- [x] パターン: WorkflowInstanceRecord の全フィールドと対応する getter → 全 11 フィールドに対応する getter あり
- [x] 型: step.rs の record_from と同じパターンを踏襲できるか → 同一パターンで実装

#### テストリスト

ユニットテスト:
- [x] 既存テスト全てがパスする（リファクタリングのみ）

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### Phase 5: role.rs テスト確認 + jscpd 最終検証

jscpd をドメイン層に実行し、残存クローンを確認。role.rs の 2 クローンが残っていれば対応する。

変更ファイル:
- `backend/crates/domain/src/role.rs`（必要に応じて）

#### 確認事項
- [x] ツール: `npx jscpd` 実行 → 15 → 10 クローンに削減。残存 10 はビジネスロジックの構造的類似性で許容
- [x] パターン: role.rs の Permission::satisfies テスト → rstest の肯定/否定テスト構造重複、可読性を損なうため対応見送り

#### テストリスト

ユニットテスト:
- [x] jscpd でドメイン層のクローン数が目標以下であることを確認
- [x] 変更した場合、既存テストが全てパスする

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | TenantName も同一パターン（3 箇所目） | 既存手段の見落とし | Phase 2 の対象に TenantName を追加 |
| 2回目 | UserId のみ Default impl がない | 不完全なパス | マクロで統一的に Default 生成。破壊的変更なし（impl 追加のみ） |
| 3回目 | TenantName は derive_more::Display、UserName/WorkflowName は手動 Display | 競合 | マクロでは手動 Display を生成して統一。機能的に等価 |
| 4回目 | マクロ配置先: shared vs domain | 未定義 | domain crate 内に配置。Uuid, derive_more, serde 等のドメイン固有依存のため |
| 5回目 | ファイルサイズ 500 行目標の達成可能性 | 不完全なパス | テストヘルパーで大幅削減。テスト外部ファイル化はスコープ外（ディレクトリ構造変更を回避） |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 15 クローン全てが計画に含まれている | OK | UUID ID 型 2 クローン（Phase 1）、StringNewtype 2 クローン（Phase 2）、step.rs 9 クローン（Phase 3）、role.rs 2 クローン（Phase 5）= 15 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | Phase 5 のみ「jscpd 結果次第」だが、条件と対応を明記 |
| 3 | 設計判断の完結性 | 全ての選択肢に判断理由を記載 | OK | マクロ vs トレイト、ヘルパー vs ビルダー、配置先、Display 統一方針を記載 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象 5 項目、対象外 4 項目を理由付きで明記 |
| 5 | 技術的前提 | コードに現れない前提が考慮されている | OK | `macro_rules!` 内のパス解決、構造体更新構文の制約、derive マクロの挙動を確認 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾がない | OK | KISS 原則（3 回以上の重複は共通化が正当）、shared crate 方針に整合 |

## 検証方法

```bash
# 各 Phase 完了後
cd backend && cargo test -p ringiflow-domain

# 全 Phase 完了後
just check-all

# クローン数の検証
npx jscpd backend/crates/domain/src --min-lines 10 --min-tokens 50 --format rust --gitignore

# ファイルサイズの検証
wc -l backend/crates/domain/src/workflow/step.rs backend/crates/domain/src/workflow/instance.rs backend/crates/domain/src/value_objects.rs backend/crates/domain/src/role.rs backend/crates/domain/src/user.rs
```
