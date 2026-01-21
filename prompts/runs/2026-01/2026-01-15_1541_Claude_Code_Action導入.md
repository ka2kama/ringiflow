# Claude Code Action 導入

## 概要

Claude Code Action を導入し、PR オープン時の自動レビューと対話的レビュー機能を追加した。

## 背景と目的

- 個人プロジェクトでは自己レビューになりがちで、見落としが発生しやすい
- プロジェクト理念（品質追求、型安全性など）を毎回意識してレビューするのは負担が大きい
- AI エージェントによる開発が進む中、一貫したレビュー基準の自動適用が望ましい

## 実施内容

1. Claude Code Action の調査
   - PRレビュー機能の有無と実装方法を確認
   - 自動レビューと対話的レビューの2つの方法があることを把握

2. ワークフローファイルの作成
   - `.github/workflows/claude-review.yml` を作成
   - PR オープン/更新時の自動レビュー
   - `@claude` メンションによる対話的レビュー
   - 同時実行制御（concurrency）でコスト管理

3. CLAUDE.md にレビュー基準セクションを追加
   - レビュー観点を明文化
   - 使い方を記載

4. ADR の作成
   - `docs/04_ADR/011_Claude_Code_Action導入.md`
   - 選択肢の比較と採用理由を記録

5. 手順書への追記
   - `docs/03_手順書/05_GitHub設定.md` にセクション10を追加
   - GitHub App インストール手順
   - API キー設定手順
   - 動作確認・トラブルシューティング

## 成果物

### 作成ファイル

| ファイル | 内容 |
|---------|------|
| `.github/workflows/claude-review.yml` | PRレビューワークフロー |
| `docs/04_ADR/011_Claude_Code_Action導入.md` | 導入決定の記録 |

### 更新ファイル

| ファイル | 変更内容 |
|---------|---------|
| `CLAUDE.md` | 「PRレビュー」セクション追加 |
| `docs/03_手順書/05_GitHub設定.md` | 「10. Claude Code Action 設定」セクション追加 |

## 設計判断と実装解説

### ワークフロー設計

**2つのジョブに分離した理由:**

```yaml
jobs:
  auto-review:     # PRオープン/更新時
  interactive-review:  # @claude メンション時
```

- トリガー条件が異なる（`pull_request` vs `issue_comment`）
- コスト制御の `--max-turns` を別々に設定可能（自動: 5、対話: 10）
- ログの追跡が容易

**同時実行制御:**

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.event.issue.number }}
  cancel-in-progress: true
```

同一 PR での重複実行を防止し、無駄な API コストを削減。

### 手順書とワークフローの役割分担

CLAUDE.md の「暗黙知ゼロ」原則に従い:

- **自動化可能** → ワークフロー（レビュー実行自体）
- **自動化不可** → 手順書（GitHub App インストール、API キー設定）

## 議論の経緯

Claude Code Action 導入による PR レビュー可否について調査を依頼された。調査結果を報告したところ、手動部分があるので手順書も必要だという指摘があり、手順書を作成した。

## 学んだこと

- Claude Code Action は CLAUDE.md のルールをネイティブに読み込み、プロジェクト固有の基準でレビューを実行できる
- GitHub Actions の `concurrency` 設定でコスト管理が可能
- 手動設定が必要な部分は手順書に明文化することで「暗黙知ゼロ」を維持

## 次のステップ

1. GitHub App のインストール
2. ANTHROPIC_API_KEY の設定
3. 動作確認（テスト用 PR で確認）
