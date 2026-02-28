# SRE 的アプローチ

## 概要

SRE（Site Reliability Engineering）は Google が提唱した、運用を体系的・定量的に改善するための工学的アプローチ。本来はサービスの信頼性を対象とするが、その核心概念は「開発プロセスの品質改善」にも適用できる。

このプロジェクトでは、SRE の概念を AI エージェント駆動の個人開発プロジェクトに適応し、`/retro` スキルで活用する。

## SRE の核心概念

### SLO / SLI（Service Level Objectives / Indicators）

サービスの品質目標（SLO）とその計測指標（SLI）。SLO は「ユーザーにとって十分な品質」を定量的に定義する。

**本プロジェクトでの適応**:

| SRE 概念 | プロジェクト適応 | 指標例 |
|---------|-----------------|--------|
| SLI | 品質指標 | 改善カテゴリ別件数、対策有効率、再発率 |
| SLO | 品質目標 | 再発率 20% 以下、対策有効率 70% 以上 |

「サービスの可用性」の代わりに「開発プロセスの品質」を測定する。

### Error Budget

SLO を満たす範囲で許容される品質逸脱の量。Error Budget が残っている間は新機能開発（リスクを取る活動）に投資でき、使い切ったら安定性向上に専念する。

**本プロジェクトでの適応**:

「攻め」（新機能開発）と「守り」（品質改善・メンテナンス）のバランスを管理する概念として使用する。

```
Error Budget 残あり（品質指標が目標内）
  → Feature 開発の比率を上げてよい

Error Budget 枯渇（品質指標が目標を下回る）
  → Maintain / Improve に注力すべき
```

| 品質指標の状態 | 攻め/守りの推奨バランス |
|-------------|---------------------|
| 目標内 | 攻め 40-60%、守り 40-60% |
| 目標ぎりぎり | 攻め 20-30%、守り 70-80% |
| 目標未達 | 攻め 0-20%、守り 80-100% |

注意: このプロジェクトはプロダクトの出荷を目的とするだけでなく、学習効果の最大化も理念とする。そのため、「攻め」には学習価値の高い技術的挑戦も含まれ、一般的な SRE のエラーバジェットとは判断基準が異なる。

### MTTR（Mean Time To Resolve）

障害発生から復旧までの平均時間。SRE では MTTR の短縮が MTBF（障害間隔）の延長より重要とされる。

**本プロジェクトでの適応**:

問題発見（改善記録の作成日）→ 対策完了（対応 Issue のクローズ日）のリードタイムとして計測する。

```
MTTR = Issue クローズ日 - 改善記録作成日
```

MTTR が長い場合の分析ポイント:
- 対策の Issue 化が遅れていないか（改善記録作成→Issue 作成のタイムラグ）
- Issue の優先度設定は適切か
- 対策の粒度が大きすぎないか（小さく分割して早期に完了できないか）

### Toil

手動で、繰り返し発生し、自動化可能で、戦術的（長期的価値を生まない）な作業。SRE チームは Toil を 50% 以下に抑えることを目標とする。

**本プロジェクトでの適応**:

開発プロセスにおける手作業のうち、自動化可能なものを特定する。

| Toil の特徴 | プロジェクトでの例 |
|------------|-----------------|
| 手動 | 定型的なファイル作成、命名規則の手動チェック |
| 繰り返し | 毎セッションの定型作業、毎コミットの確認事項 |
| 自動化可能 | just タスク、hooks、CI で代替できる |
| 戦術的 | チェックリストの手動確認、フォーマットの手動修正 |

自動化の優先度判定:

| 優先度 | 基準 |
|--------|------|
| 高 | 週に 3 回以上発生 + just/hooks で即対応可能 |
| 中 | 月に 3 回以上発生 + スクリプト作成で対応可能 |
| 低 | 判断を伴い完全自動化が困難 |

### Blameless Postmortem

インシデント後の振り返りで、個人を責めず、システム的な改善に焦点を当てる文化。

**本プロジェクトでの対応**:

既に改善記録の「カテゴリ分類」として実装済み。改善記録は個人の失敗ではなく「AI エージェントの構造的特性」として分類し、システム的な対策（成果物要件化、フロー組み込み等）を導出する。

### Incident → Problem Management

ITIL の概念。個別のインシデント対応（Incident Management）と、根本原因の解消による再発防止（Problem Management）を区別する。

**本プロジェクトでの適応**:

| ITIL 概念 | プロジェクト対応 |
|----------|---------------|
| Incident Management | 個別の改善記録（即時対応） |
| Problem Management | `/retro` でのパターン分析（構造的改善） |

個別の改善記録は「インシデント」レベルの対応。`/retro` が複数の改善記録を横断分析することで、個別対応では見えない構造的な問題（Problem）を発見し、根本的な改善につなげる。

## /retro スキルでの具体的な適用

| SRE 概念 | /retro の Step | 活用方法 |
|---------|---------------|---------|
| SLO/SLI | Step 1（改善システムの検証） | カテゴリ別・失敗タイプ別の推移を品質指標として追跡 |
| Error Budget | Step 3c（エラーバジェット的思考） | 攻め/守りのバランスを Issue ラベルから定量評価 |
| MTTR | Step 3a（再発率・MTTR 分析） | 改善記録→Issue クローズのリードタイムを計測 |
| Toil | Step 3b（Toil 分析） | セッションログから繰り返しパターンを検出 |
| Blameless Postmortem | Step 1（既存の改善記録を活用） | カテゴリ分類による構造的分析 |
| Problem Management | Step 1-5（再発検出） | 個別改善記録の横断分析でパターンを発見 |

## Google SRE Book との対応

本プロジェクトの適応は、以下の章に基づく:

| 章 | 内容 | 適応箇所 |
|----|------|---------|
| Ch.3 Embracing Risk | Error Budget の概念 | /retro Step 3c |
| Ch.4 Service Level Objectives | SLO/SLI の設計 | /retro Step 1 の品質指標 |
| Ch.5 Eliminating Toil | Toil の定義と削減 | /retro Step 3b |
| Ch.15 Postmortem Culture | Blameless Postmortem | 改善記録の分類体系 |
| Ch.28 Accelerating SREs to On-Call | 運用知識の体系化 | ナレッジベース全体 |

## 参考文献

- Beyer, B., Jones, C., Petoff, J. & Murphy, N. R. (2016) "Site Reliability Engineering: How Google Runs Production Systems", O'Reilly
  - 通称: Google SRE Book。[オンライン版](https://sre.google/sre-book/table-of-contents/)
- Beyer, B., Murphy, N. R., Rensin, D. K., Kawahara, K. & Thorne, S. (2018) "The Site Reliability Workbook", O'Reilly
  - SRE の実践ガイド。[オンライン版](https://sre.google/workbook/table-of-contents/)
- ITIL v4 (2019) "ITIL Foundation: ITIL 4 Edition", Axelos
  - Incident Management / Problem Management の概念

## 関連

- [AI 思考特性の分析ガイド](AI思考特性の分析ガイド.md) — 改善記録の分析手法
- [独自フレームワークと既知手法の対応](独自フレームワークと既知手法の対応.md) — プロジェクト全体の方法論マッピング
- [改善記録の分類体系](../../../process/improvements/README.md#分類)
- `/retro` スキル — SRE 概念の実行エンジン

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-02-10 | 初版作成（/retro スキル導入に伴い） |
