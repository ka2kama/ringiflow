# Issue #290: 500行超ファイルのリファクタリング戦略

## Context

Issue #290 は500行超のファイルを分割するリファクタリングタスク。探索の結果、Issue 記載の Backend 5ファイルのうち **3つは既にモジュール化済み**（ADR-039, ADR-041）であり、Issue の記述は現状と大きく乖離している。また、Issue 未記載だが閾値を超える追加ファイルも発見された。

本計画は ADR-043 を作成し、残りの分割対象と例外を整理した上で、段階的にリファクタリングを進める。

## 現状分析

### Issue 記載ファイルの状況

| ファイル | Issue 記載行数 | 現在行数 | 状態 |
|---------|-------------|---------|------|
| core-service/usecase/workflow.rs | ~1682 | 119 | **完了**（ADR-039, CQRS分割） |
| domain/src/workflow.rs | ~1295 | 43 | **完了**（ADR-039, エンティティ分割） |
| bff/client/core_service.rs | ~1211 | 30 | **完了**（ADR-041, ISP分割） |
| bff/handler/auth.rs | ~1171 | 985 | prod 442行 + test 543行 → **例外候補** |
| core-service/usecase/task.rs | ~995 | 1042 | prod 225行 + test 817行 → **例外**（ADR-041） |

### 追加で発見された閾値超過ファイル

| ファイル | 行数 | prod | test | 判定 |
|---------|------|------|------|------|
| core-service/handler/workflow.rs | 780 | 780 | 0 | **分割対象**（全てプロダクションコード） |
| bff/handler/workflow.rs | 754 | 754 | 0 | **分割対象**（全てプロダクションコード） |
| domain/value_objects.rs | 650 | 437 | 213 | **例外候補**（prod < 500） |
| frontend/src/Page/Workflow/New.elm | 1115 | — | — | **分割対象**（複合責務） |
| frontend/src/Main.elm | 832 | — | — | **例外候補**（TEA アーキテクチャの帰結） |

### 判定基準

structural-review.md の判断プロセスに基づく:

| 判定 | 基準 | 該当ファイル |
|------|------|------------|
| 分割対象 | プロダクションコードが500行超 & 複数責務混在 | workflow handler ×2, New.elm |
| 例外許容 | プロダクションコードが500行未満（テストが大きい） | auth.rs, task.rs, value_objects.rs |
| 例外許容 | アーキテクチャパターンの帰結 | Main.elm |

## このセッションのスコープ

### 対象

1. **ADR-043 作成**: 全体の分割戦略を文書化
2. **Issue #290 更新**: 完了済み項目のチェック、追加ファイルの記載、精査結果の反映

### 対象外

- 個別ファイルのリファクタリング（後続 PR で実施）
- フロントエンドの分割（別セッション）

### 理由

Issue 完了基準の最初の項目「分割対象と分割方針の ADR を作成」をまず完了させる。ADR で全体戦略を確定してから、個別ファイルに着手する。

## ADR-043 の内容

### タイトル

`043_500行超ファイルの分割戦略.md`

### 構成

1. **コンテキスト**: Issue #290 の経緯、既に完了した分割（ADR-039, 041）、追加発見ファイル
2. **責務分析の結果**: 各ファイルのプロダクション/テスト比率と判定
3. **検討した選択肢**: 分割対象ファイルごとの分割方法
4. **決定**: 分割対象、例外許容、優先順位
5. **帰結**: 後続 PR の計画

### 分割対象ファイルの方針案

#### Backend: handler/workflow.rs（core-service: 780行, bff: 754行）

既存パターン（ADR-039 の usecase/workflow.rs 分割）に倣い、ディレクトリモジュール化:

```
handler/
├── workflow.rs          # 親モジュール（mod + pub use re-export）
└── workflow/
    ├── command.rs       # 状態変更系ハンドラ（create, submit, approve, reject, cancel）
    └── query.rs         # 読み取り系ハンドラ（list, get, detail, steps）
```

- 分割基準: CQRS パターン（usecase 層と同じ分割軸）
- core-service と bff の両方に同じパターンを適用
- 消費者への影響: re-export により既存のインポートパスを維持

#### Frontend: Page/Workflow/New.elm（1115行）

承認者選択 UI をコンポーネントとして抽出:

```
Component/
└── ApproverSelector.elm  # 承認者検索＆選択コンポーネント（Model, Msg, update, view）
```

- 抽出対象: ApproverSelection 型、handleApproverKeyDown、viewApprover* 関数群
- 推定削減: ~180行 → New.elm は ~935行に（閾値付近だが責務は明確化）

### 例外許容ファイルの記録

| ファイル | prod行数 | 理由 |
|---------|---------|------|
| bff/handler/auth.rs | 442 | prod < 500、4エンドポイントが全て認証関連で高凝集 |
| core-service/usecase/task.rs | 225 | prod < 500（ADR-041 で既に判断済み） |
| domain/value_objects.rs | 437 | prod < 500、値オブジェクト集約は凝集度が高い |
| Main.elm | 832 | Elm TEA + Nested TEA ルーターの帰結、分割すると型安全性が低下 |

### 優先順位

| 順位 | ファイル | 理由 | PR |
|------|---------|------|-----|
| 1 | core-service/handler/workflow.rs | 最大のプロダクションコード（780行全て prod）| 後続 PR-1 |
| 2 | bff/handler/workflow.rs | 同上パターン適用（754行全て prod） | 後続 PR-2 |
| 3 | Page/Workflow/New.elm | 複合責務の分離（1115行） | 後続 PR-3 |

## Issue #290 の更新内容

### チェックボックス更新

完了基準:
- [ ] 分割対象と分割方針の ADR を作成 → **このセッションで完了**
- [ ] 段階的にリファクタリング（1ファイルずつ PR）
- [ ] 分割後も `just check-all` が通る

### 対象ファイル表の更新

Issue 本文を現状に合わせて更新:
- 完了済み 3ファイルにチェック
- task.rs を例外として記録
- 追加発見ファイルを追記（workflow handler ×2）

## 変更対象ファイル

| ファイル | 操作 |
|---------|------|
| `docs/05_ADR/043_500行超ファイルの分割戦略.md` | 新規作成 |
| Issue #290 本文 | `gh issue edit` で更新 |

## 検証方法

- ADR の内容が structural-review.md の判断プロセスに準拠しているか
- 既存 ADR（039, 041）との整合性
- Issue の完了基準の最初の項目が達成されるか

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Issue 記載の5ファイル中3つが既に完了済みで、Issue の前提が古い | 不完全なパス | 現状分析セクションで完了済みファイルを明記し、Issue 更新を計画に含めた |
| 2回目 | Issue 未記載の閾値超過ファイルが4つ存在 | 未定義 | 追加発見ファイルとして分析対象に含め、ADR でカバー |
| 3回目 | auth.rs, value_objects.rs はプロダクションコードが500行未満で、テストが膨らませている | 既存手段の見落とし | structural-review.md の例外許容パターン（task.rs と同じ）を適用 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | `just check-file-size` 相当の全ファイル調査を実施。Issue 記載 + 追加発見の全ファイルを分析済み |
| 2 | 曖昧さ排除 | OK | 各ファイルの prod/test 行数を実測し、判定基準（structural-review.md）に基づく判定を明示 |
| 3 | 設計判断の完結性 | OK | 分割対象 vs 例外許容の判断が全ファイルについて理由付きで記載 |
| 4 | スコープ境界 | OK | このセッション = ADR + Issue 更新、後続 = 個別ファイル PR と明記 |
| 5 | 技術的前提 | OK | Rust のディレクトリモジュール化パターン（ADR-039）、Elm のコンポーネント化パターンを確認済み |
| 6 | 既存ドキュメント整合 | OK | ADR-039, ADR-041, structural-review.md と照合済み |
