# Claude Code Action PRレビュープロンプト設計

Claude Code Action で自動 PR レビューを行う際のプロンプト設計についてまとめる。

権限設定については [ClaudeCodeAction_権限設定.md](ClaudeCodeAction_権限設定.md) を参照。

## ベストプラクティス（2026年時点）

### 1. GitHub コンテキストを明示的に渡す

```yaml
prompt: |
  REPO: ${{ github.repository }}
  PR NUMBER: ${{ github.event.pull_request.number }}
```

Claude Code Action は PR ブランチを自動でチェックアウトするが、コンテキスト情報を明示することで指示が明確になる。

### 2. 重大な問題に集中する

一般的なベストプラクティスでは「重大な問題のみ指摘」が推奨される。

```yaml
prompt: |
  以下の観点で **重大な問題のみ** を指摘してください:
  - バグ・正確性
  - セキュリティ
  - パフォーマンス

  ## 指摘しないこと（nitpick 禁止）
  - コードスタイルの好み
  - 「こうした方がより良い」程度の提案
```

**利点**: レビューコメントが膨大にならない、開発者の負担軽減

**注意**: プロジェクト固有の方針（学習目的など）がある場合は調整が必要

### 3. 承認基準を明確にする

```yaml
prompt: |
  | 重大度 | 基準 | アクション |
  |--------|------|-----------|
  | Critical/High | マージ前に必ず修正が必要 | `gh pr review --request-changes` |
  | Medium/Low | 改善推奨だがマージ可能 | `gh pr review --approve` + コメント |
  | None | 問題なし | `gh pr review --approve` |
```

### 4. Single Source of Truth を維持する

プロジェクト固有のルールは CLAUDE.md に記述し、プロンプトからは参照のみ行う。

```yaml
# 良い例: 参照 + 補完
prompt: |
  CLAUDE.md に記載されたプロジェクト理念と品質基準に基づいてレビューしてください。

  ## レビュー観点（CLAUDE.md を補完）
  ...

# 悪い例: 重複記述
prompt: |
  ## プロジェクト理念
  1. 学習効果の最大化: ...  # CLAUDE.md と重複
  2. 品質の追求: ...
```

**重複記述の問題**:
- DRY 原則違反
- 保守性の低下（変更時に複数箇所を更新）
- Single Source of Truth の破壊

### 5. フィードバック方法を指定する

```yaml
prompt: |
  ## フィードバック方法
  - **コード固有の問題**: `mcp__github_inline_comment__create_inline_comment` でインラインコメント
  - **全体的なフィードバック**: `gh pr comment` でPRコメント
```

インラインコメントは該当コードの直下に表示されるため、文脈が明確になる。

### 6. 進捗トラッキングを有効化する

```yaml
- uses: anthropics/claude-code-action@v1
  with:
    track_progress: true
```

レビュー状況を PR コメントでリアルタイム表示する。

## プロジェクト固有の調整

### 学習目的のプロジェクト

一般的なベストプラクティスでは nitpick を禁止するが、学習目的のプロジェクトでは設計判断の解説が価値を持つ。

```yaml
prompt: |
  ### 学習機会の提供

  設計判断を伴う重要な箇所では、学習のため簡潔に解説する:
  - なぜその実装が良い/悪いのか
  - 代替案があればトレードオフとともに提示

  すべての箇所に解説は不要。学びになるポイントのみ。

  学習のための解説コメントは承認判断に影響しない。
```

**ポイント**: 解説コメントと承認判断を分離する

### 型システム重視のプロジェクト（Rust/Elm）

```yaml
prompt: |
  ## 型システムの活用
  - 型で表現できるものを文字列/整数で代用していないか
  - 不正な状態が表現可能になっていないか
  - 安易な unwrap/expect がないか
```

## トリガーイベントの設定

```yaml
on:
  pull_request:
    types: [opened, synchronize, ready_for_review, reopened]
```

| イベント | 説明 |
|---------|------|
| `opened` | PR 新規作成時 |
| `synchronize` | PR 更新時（新しいコミット） |
| `ready_for_review` | ドラフトから Ready に変更時 |
| `reopened` | クローズ後の再オープン時 |

## 参考リンク

- [Claude Code Action リポジトリ](https://github.com/anthropics/claude-code-action)
- [Solutions Guide](https://github.com/anthropics/claude-code-action/blob/main/docs/solutions.md)
- [Custom Automations](https://github.com/anthropics/claude-code-action/blob/main/docs/custom-automations.md)
