# ADR-011: Claude Code Action 導入

## ステータス

承認済み

## コンテキスト

PRレビューはコード品質を維持するための重要なプロセスだが、以下の課題がある:

- 個人プロジェクトでは自己レビューになりがちで、見落としが発生しやすい
- プロジェクト理念（品質追求、型安全性など）を毎回意識してレビューするのは負担が大きい
- AIエージェントによる開発が進む中、一貫したレビュー基準の自動適用が望ましい

GitHub Actions 上で Claude Code を実行できる Claude Code Action の導入を検討した。

## 検討した選択肢

### 選択肢 1: Claude Code Action

Anthropic 公式の GitHub Action。PRオープン時に自動レビュー、コメントで対話的レビューが可能。

評価:
- 利点: 公式サポート、CLAUDE.md のルールを自動適用、対話的なフィードバックが可能
- 欠点: API コスト発生、レビュー精度は完璧ではない

### 選択肢 2: 他の AI レビューツール（CodeRabbit、Codiumate 等）

サードパーティの AI コードレビューサービス。

評価:
- 利点: 専用サービスとしての成熟度
- 欠点: プロジェクト固有ルール（CLAUDE.md）との連携が弱い、別サービスへの依存

### 選択肢 3: 導入しない

従来通り手動レビューのみ。

評価:
- 利点: 追加コストなし
- 欠点: 自己レビューの限界、品質基準の適用漏れリスク

### 比較表

| 観点 | Claude Code Action | 他ツール | 導入しない |
|------|-------------------|----------|-----------|
| CLAUDE.md 連携 | ◎ ネイティブ対応 | △ 限定的 | - |
| コスト | △ API従量課金 | △ サブスク/API | ◎ 無料 |
| 導入の容易さ | ◎ ワークフロー追加のみ | △ 設定が複雑 | ◎ 不要 |
| プロジェクトとの一貫性 | ◎ 同じ Claude を使用 | △ 別のモデル | - |

## 決定

**Claude Code Action を導入する。**

主な理由:

1. **CLAUDE.md との一貫性**: 開発時と同じルールでレビューが実行される
2. **プロジェクト理念の自動適用**: 品質基準・設計原則が自動でチェックされる

## 帰結

### 肯定的な影響

- PRオープン時に自動で品質チェックが実行される
- プロジェクト固有のルール（テナント削除対応、型安全性など）が見落とされにくくなる
- 学習効果の最大化（理念1）にも寄与: レビューコメントから新たな知見を得られる

### 否定的な影響・トレードオフ

- API コストが発生（ただし個人プロジェクトでは許容範囲）
- AI レビューは完璧ではないため、最終判断は人間が行う必要がある
- GitHub Secrets に API キーを保存する必要がある

### 関連ドキュメント

- 実装: [`.github/workflows/claude-auto-review.yaml`](../../.github/workflows/claude-auto-review.yaml)
- レビュー基準: [`CLAUDE.md`](../../CLAUDE.md) の「PRレビュー」セクション

---

## 補足: workflow_run イベントでのステータス報告

`workflow_run` イベントでトリガーされるワークフローは、デフォルトブランチ（main）のコンテキストで実行されるため、PR のコミットにステータスが自動で紐付かない。

この問題を解決するため、GitHub Status API を使って明示的にステータスを報告している:

```yaml
gh api "repos/{owner}/{repo}/statuses/{sha}" \
  -f state=pending|success|failure \
  -f context="Claude Auto Review" \
  -f description="..." \
  -f target_url="..." || true
```

`|| true` を付けることで、API エラー時もワークフローを継続する（ステータス報告は補助機能のため）。

これにより、Ruleset で「Claude Auto Review」を必須チェックとして設定可能になる。

参考: [Creating commit status checks](https://docs.github.com/en/rest/commits/statuses)

---

## 補足: Bot からの PR 対応

Claude Code Action はデフォルトで Bot からの PR を処理しない（セキュリティ上の理由）。

Dependabot によるセキュリティアップデート PR など、信頼できる Bot からの PR にもレビューを実行するには、`allowed_bots` パラメータを設定する:

```yaml
- uses: anthropics/claude-code-action@v1
  with:
    allowed_bots: "dependabot[bot]"  # Dependabot を許可
    # allowed_bots: "*"  # 全 Bot を許可（非推奨）
```

設定値:
- `"dependabot[bot]"`: Dependabot のみ許可
- `"dependabot[bot],renovate[bot]"`: 複数 Bot をカンマ区切りで許可
- `"*"`: 全 Bot を許可（セキュリティリスクあり）
- `""` (デフォルト): Bot を許可しない

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-25 | Bot からの PR 対応（allowed_bots）を追加 |
| 2026-01-18 | workflow_run イベントでのステータス報告を追加 |
| 2026-01-15 | 初版作成 |
