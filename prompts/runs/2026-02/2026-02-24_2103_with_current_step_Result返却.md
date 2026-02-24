# #854 with_current_step を Result 返却に変更

## 概要

`WorkflowInstance::with_current_step` メソッドの戻り値を `Self` から `Result<Self, DomainError>` に変更し、Pending 状態からのみ InProgress への遷移を許可するようにした。`_ =>` ワイルドカードパターンを削除し、型安全ステートマシンパターンの一貫性を確保した。

## 実施内容

### メソッド本体の変更

- 戻り値: `Self` → `Result<Self, DomainError>`
- Pending アーム: `Self { ... }` → `Ok(Self { ... })`
- `_ =>` アーム: 暫定フォールバックを `Err(DomainError::Validation(...))` に置換
- エラーメッセージ: `"ステップ設定は承認待ち状態でのみ可能です（現在: {}）"` — 他メソッドと同一パターン
- FIXME(#854) コメント削除

### 呼び出し元の更新

- 本番コード（1箇所）: `submit.rs` に `.map_err(|e| CoreError::BadRequest(e.to_string()))?` を追加
- テストビルダー（1箇所）: `workflow_test_builder.rs` に `.unwrap()` を追加
- テストコード（約50箇所）: 12ファイルで `.unwrap()` を追加

### テスト追加

- `test_下書きからのステップ設定はエラー`: Draft 状態から `with_current_step` を呼ぶと `DomainError` が返ることを検証

## 判断ログ

- エラーメッセージ形式: 既存メソッドの `"〇〇は××状態でのみ可能です（現在: {}）"` パターンに準拠
- テスト追加は異常系1件のみ: 正常系（Pending → InProgress）は既存テストで十分にカバーされている

## 成果物

### コミット

- `9003dad` #854 Change with_current_step to return Result and restrict to Pending state

### PR

- [#887](https://github.com/ka2kama/ringiflow/pull/887) (Draft)

### 更新ファイル

| ファイル | 変更内容 |
|---------|---------|
| `backend/crates/domain/src/workflow/instance.rs` | メソッド変更 + エラーテスト追加 + 既存テスト `.unwrap()` 追加 |
| `backend/apps/core-service/src/usecase/workflow/command/lifecycle/submit.rs` | 本番コード `.map_err()?` + テスト `.unwrap()` |
| `backend/apps/core-service/src/test_utils/workflow_test_builder.rs` | `.unwrap()` 追加 |
| テスト 10 ファイル | `.unwrap()` の機械的追加 |
| `prompts/plans/854_with-current-step-result.md` | 計画ファイル |
