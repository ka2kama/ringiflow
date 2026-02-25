# Auto Review に Validation パスを追加

## 概要

Claude Code Action の Auto Review に Validation チェック（書かれていないことの検出）を追加した。従来は diff ベースの Verification（バグ、セキュリティ、設計等）のみだったが、Issue の完了基準・PR 本文・計画ファイルを参照して欠落（Omission）と乖離（Divergence）を検出する機能を追加した。

## 実施内容

### 設計検討

Code Review における「書かれていないこと」のレビューの必要性について議論し、以下の構造を整理した:

- **Verification**: 書かれたコードの品質（diff ベース、受動的な認知）
- **Validation**: 書かれるべきものの過不足（仕様起点、能動的な認知）

Auto Review が Validation を行うには「何が書かれるべきか」の期待（メンタルモデル）が必要。その情報源として Issue の完了基準、PR 本文の品質確認セクション、計画ファイルを選定した。

### 設計判断

| 判断 | 方向性 | 理由 |
|------|--------|------|
| 役割分担 | 補完型（品質ゲートが主） | 品質ゲートは実装者が文脈を持った状態で実施するため精度が高い |
| 情報源 | Issue + PR 本文 + 計画ファイル | アクセス可能でコスト対効果が高い情報源 |
| 承認判断 | Validation は常に approve | 偽陽性の可能性があり、request-changes にすると不要なブロックが発生する |
| 確信度 | 各 Validation 指摘に高/中/低を付記 | レビュー指摘のトリアージコストを下げる |
| メタデータ | `validation-omission`/`validation-divergence` を追加 | 後方互換性維持（`severity-*` は Verification 専用） |

### ワークフロー変更

`.github/workflows/claude-auto-review.yaml` に以下を実装:

1. `Fetch validation context` ステップ追加（Issue 本文、PR 本文、計画ファイルを取得）
2. プロンプトに Validation チェックセクション追加（欠落・乖離の検出指示）
3. 出力フォーマットの Verification/Validation 分離
4. 承認判断ルールの更新
5. メタデータスキーマの拡張
6. `gh issue view` を allowedTools に追加

## 判断ログ

- 暗黙的依存関係（DB スキーマ → Rust 構造体 → API レスポンス → Elm デコーダ）は Grep では検出困難。静的マップの事前構築ではなく、レビュー時の動的探索とカテゴリ別チェックリストの組み合わせが現実的と判断した
- Validation 指摘の severity 上限を設けるか、確信度を別軸にするかで、確信度の別軸方式を採用。severity は問題の深刻度、確信度は検出の確かさという異なる次元を混同しないため
- review-and-merge スキルの変更は不要と判断。メタデータの `severity-*` を Verification 専用に維持することで後方互換性を確保

## 成果物

### コミット

- `Add Validation pass to Auto Review for omission/divergence detection`
- `Fix shellcheck SC2231: quote variable expansion in glob`

### ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.github/workflows/claude-auto-review.yaml` | Validation ステップ・プロンプト・メタデータの追加 |
| `prompts/plans/20260225_auto-review-validation.md` | 計画ファイル |

### PR

- [#926](https://github.com/ka2kama/ringiflow/pull/926) — Draft

## 議論の経緯

1. Code Review が差分を見る傾向にある問題提起から開始
2. 「書かれたコード」と「書かれていないコード」のレビューの認知モードの違いを分析
3. 仕様起点のレビュー、フェーズ分離、構造的チェックポイントの3つの戦略を検討
4. Auto Review への組み込みに話が進み、Grep での対応限界（暗黙的依存関係）を議論
5. 具体的な How に寄り過ぎたため、元の要望からフラットに再検討
6. Validation の3つの判断軸（役割、情報源、伝え方）を整理し合意
7. 計画モードで詳細設計を行い、実装・PR 作成まで完了
