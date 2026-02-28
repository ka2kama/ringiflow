# FindResultExt パターン

## 概要

`FindResultExt` は、リポジトリの `find_by_id` 等が返す `Result<Option<T>, InfraError>` を `Result<T, CoreError>` に変換する拡張トレイト。Core Service ユースケース層の重複するエラーハンドリングパターンを 1 メソッド呼び出しに集約する。

## 背景

ユースケース層では、リポジトリからエンティティを取得する際に以下の 3 行パターンが繰り返されていた:

```rust
let step = self.step_repo.find_by_id(&step_id, &tenant_id).await
    .map_err(|e| CoreError::Internal(format!("ステップの取得に失敗: {}", e)))?
    .ok_or_else(|| CoreError::NotFound("ステップが見つかりません".to_string()))?;
```

この `map_err` + `ok_or_else` パターンは全ユースケースファイルに 20 箇所以上存在し、エラーメッセージの表記揺れ（"取得に失敗" vs "取得エラー"）も発生していた。

## 設計

### 拡張トレイト（Extension Trait）

```rust
pub(crate) trait FindResultExt<T> {
    fn or_not_found(self, entity_name: &str) -> Result<T, CoreError>;
}

impl<T> FindResultExt<T> for Result<Option<T>, InfraError> {
    fn or_not_found(self, entity_name: &str) -> Result<T, CoreError> {
        self.map_err(|e| CoreError::Internal(format!("{}の取得に失敗: {}", entity_name, e)))?
            .ok_or_else(|| CoreError::NotFound(format!("{}が見つかりません", entity_name)))
    }
}
```

### 使用例

```rust
use crate::usecase::helpers::FindResultExt;

let step = self.step_repo.find_by_id(&step_id, &tenant_id).await
    .or_not_found("ステップ")?;
```

### 自由関数ではなく拡張トレイトを選んだ理由

| 観点 | 拡張トレイト | 自由関数 |
|------|------------|---------|
| 呼び出し | `.or_not_found("ステップ")?` | `find_or_not_found(repo.find(...).await, "ステップ")?` |
| メソッドチェーン | 自然に繋がる | 関数のネストが必要 |
| Rust の慣例 | `anyhow::Context`, `color_eyre::WrapErr` と同パターン | — |

### カバーしないケース

以下のパターンは `or_not_found` でカバーしない:

- `Result<T, InfraError>`（`Option` を含まない）: 単純な `map_err` で十分
- `InfraError::Conflict` のマッチ: 楽観的ロックのエラーハンドリングは固有ロジック
- データ整合性チェック（`CoreError::Internal` で NotFound 相当を返す箇所）: ドメインの意味が異なる

## プロジェクトでの使用箇所

| ファイル | 使用箇所数 |
|---------|----------|
| `usecase/workflow/command/decision/*.rs` | ~10 |
| `usecase/workflow/command/lifecycle/*.rs` | 7 |
| `usecase/workflow/query.rs` | 4 |
| `usecase/task.rs` | 3 |
| `usecase/workflow/command/comment.rs` | 1 |

定義: `backend/apps/core-service/src/usecase/helpers.rs`

## 関連リソース

- [anyhow::Context](https://docs.rs/anyhow/latest/anyhow/trait.Context.html) — 同様の拡張トレイトパターン
- [Rust API Guidelines: Extension traits (C-EXT)](https://rust-lang.github.io/api-guidelines/flexibility.html#c-ext) — 拡張トレイトの設計指針
- 導入 Issue: [#537](https://github.com/ka2kama/ringiflow/issues/537)
