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

問題点:
1. Bot のコメントもジョブ条件でスキップされるが、ワークフロー自体は開始される
2. ワークフローが開始されると concurrency グループに影響を与える
3. 結果として、Bot のコメントによる新しい run が前の run をキャンセル

### 解決策（2段階）

**第1段階（PR #47）:** ジョブ条件に Bot 除外を追加
- 効果なし（ワークフローは開始されるため）

**第2段階:** ワークフローを分割

auto-review と interactive-review を別ファイルに分離:

| ファイル | 目的 | トリガー |
|---------|------|---------|
| `claude-auto-review.yml` | 自動レビュー | `pull_request` |
| `claude-interactive.yml` | 対話的レビュー | `issue_comment`, `pull_request_review_comment` |

これにより:
- 異なるワークフロー名 = 異なる concurrency グループ
- Bot のコメントが auto-review をキャンセルすることはない
- interactive-review は `comment.id` を含む concurrency グループで自己キャンセルを防止

## 成果物

| 種類 | ファイル |
|------|---------|
| 削除 | `.github/workflows/claude-review.yml` |
| 新規 | `.github/workflows/claude-auto-review.yml` |
| 新規 | `.github/workflows/claude-interactive.yml` |

## 学んだこと

- GitHub Actions の concurrency はワークフローレベルで適用される
- ジョブ条件でスキップされても、ワークフロー自体は開始され concurrency に影響する
- 異なるイベントタイプを扱うワークフローは分割した方が管理しやすい
- `cancel-in-progress: false` + コメント ID を concurrency に含めることで、各コメントを独立して処理可能

## 次のステップ

- PR をマージ後、PR #46 で再度 `@claude` メンションして動作確認
