# Claude Code Review の自己キャンセル問題を修正

## 概要

Claude Code Action の interactive-review ジョブが、自身のコメント（エラー報告）によってキャンセルされる問題を修正した。

## 背景と目的

PR #46 で `@claude` メンションによる設計レビューを依頼した際、以下の問題が発生した:

1. ユーザーが `@claude` でコメント
2. interactive-review ジョブが開始
3. Claude Code Action がエラーコメントを投稿
4. そのコメントが新しい `issue_comment` イベントをトリガー
5. `cancel-in-progress: true` により実行中のジョブがキャンセル
6. 新しいジョブは `@claude` を含まないためスキップ

結果として、レビューが途中でキャンセルされ完了しない。

## 実施内容

### 原因分析

ワークフローの concurrency 設定:

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event_name }}-${{ ... }}
  cancel-in-progress: true
```

Claude bot のコメントも `issue_comment` イベントをトリガーするため、同じ concurrency グループに属し、前の実行をキャンセルしていた。

### 解決策

interactive-review ジョブの条件に bot ユーザーの除外を追加:

```yaml
if: >
  (github.event_name == 'issue_comment' || ...) &&
  contains(github.event.comment.body, '@claude') &&
  github.event.comment.user.login != 'claude[bot]' &&
  github.event.comment.user.login != 'github-actions[bot]'
```

## 成果物

| 種類 | ファイル |
|------|---------|
| 修正 | `.github/workflows/claude-review.yml` |

## 学んだこと

- GitHub Actions の `cancel-in-progress` は同じ concurrency グループの実行をすべてキャンセルする
- Bot が投稿するコメントも `issue_comment` イベントをトリガーする
- Bot による無限ループや自己キャンセルを防ぐには、コメント作成者のチェックが必要

## 次のステップ

- PR をマージ後、PR #46 で再度 `@claude` メンションして動作確認
