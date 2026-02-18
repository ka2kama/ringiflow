# workflow command モジュール分割

## 概要

`backend/apps/core-service/src/usecase/workflow/command.rs`（3501行）をドメイン責務に基づき 3 つのサブモジュールに分割した。ADR-043 の 500 行閾値に対し、プロダクションコード ~1097 行で約 2 倍超過していたため。

純粋なリファクタリングであり、振る舞いの変更はない。

## 実施内容

### 分割構造

| モジュール | 責務 | メソッド数 | prod行数 |
|-----------|------|-----------|---------|
| `lifecycle.rs` | ワークフローの作成・申請・再申請 | 5 | ~444 |
| `decision.rs` | 承認者のステップ判断 | 6 | ~555 |
| `comment.rs` | コラボレーション | 2 | ~108 |
| `command.rs`（親） | mod 宣言 + 共有テストヘルパー | - | ~80 |

### テスト不整合の検証・修正

サブエージェントを使ってコード移動を実施したところ、テストコードがオリジナルから乖離する問題が発生した。具体的には:

- アサーションスタイルの変更（`assert_eq!(result, expected)` → 個別フィールド検証）
- テストデータの変更（コメント文字列、`comment: None` → `Some(...)`）
- 不要な定義の追加
- 変数名の変更

ユーザーの指摘により全テストをオリジナルと突合し、6 テストの不整合を修正した。

→ 改善記録: [サブエージェントによるテストコード改変](../../../process/improvements/2026-02/2026-02-14_2118_サブエージェントによるテストコード改変.md)

## 判断ログ

- 3 分割の粒度は計画通り。2 分割だと lifecycle+decision が ~792 行で閾値超過、4 分割は同一責務の過分割
- `decision.rs` のプロダクションコードが 555 行で 500 行閾値をやや超過するが、approve/reject/request_changes は同一の「承認者の判断」責務であり、これ以上の分割は過分割と判断
- テストヘルパーは親 `command.rs` の `#[cfg(test)] pub(super) mod test_helpers` に配置。`single_approval_definition_json` が lifecycle と decision の両方で使用されるため
- テスト内の import パスは `crate::` パスで統一（`super::super::super::` の冗長さを回避）

## 成果物

- コミット: `3c47701` — `#493 Refactor workflow command module into domain-based submodules`
- Draft PR: #508
- 計画ファイル: `prompts/plans/493_split-workflow-command-module.md`
- 改善記録: `prompts/improvements/2026-02/2026-02-14_2118_サブエージェントによるテストコード改変.md`

## 議論の経緯

- ユーザーがテスト修正後に「本当に問題ないか？」と問いかけ、全テストのオリジナルとの突合検証を実施。結果、6 テストで不整合を発見し修正した
- サブエージェントによるコード生成の信頼性に関する教訓を得た
