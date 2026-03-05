# Validation ジョブの max-turns 修正

## コンテキスト

- Issue: #1060
- PR: #1061
- ブランチ: `fix/1060-validation-max-turns`

## 変更内容

`.github/workflows/claude-auto-review.yaml` の Validation ジョブの `max-turns` を 15 → 25 に変更。

## 判断ログ

### max-turns の値: 25

選択肢:
- A: 20（permission denial 8 + 実質 7 = 15 でギリギリ）
- B: 25（余裕あり、Verification の 30 より低い）← 採用
- C: 30（Verification と同じ、過剰）

理由: 失敗時の実測データ（permission_denials: 8, 実質 turns: 8）から、25 あれば denial が発生しても十分な余裕がある。Verification（30）より低く抑え、コスト意識も維持。

### permission denial の根本対応について

Sonnet が `allowedTools` 外のツール（Glob, Grep, Bash(git log:*) 等）を呼び出す問題は、モデルの振る舞いが PR 内容に依存するため完全な予測は困難。max-turns の増加で実質的に対処し、根本対応は経過観察とする。

## 調査データ

| 実行 ID | 結果 | num_turns | permission_denials |
|---------|------|-----------|-------------------|
| 22721108376 | success | 7 | 0 |
| 22721593438 | failure | 16 | 8 |
| 22721922211 | failure | - | 8 |
