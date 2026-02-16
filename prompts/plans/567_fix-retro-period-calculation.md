# #567 レトロ Issue 自動生成の対象期間算出ロジックを修正する

## Context

週次レトロスペクティブ Issue の自動生成ワークフロー（`weekly-retro.yaml`）が、固定の「前週月曜〜日曜」ウィンドウで対象期間を算出している。実際にはレトロは 2〜3 日間隔で実施されており、固定ウィンドウとの乖離が発生。#562 では手動で期間を修正する必要があった。

`/retro` スキル自体は「前回レトロレポートの日付〜今日」で期間を算出しており、ワークフロー側もこれに揃える。

## 対象

- `.github/workflows/weekly-retro.yaml`

## 対象外

- `monthly-assess.yaml`（類似の問題があるが、別 Issue で対応すべき）
- `/retro` スキル（SKILL.md）自体の変更

---

## Phase 1: weekly-retro.yaml の対象期間算出ロジック修正

### 確認事項
- [x] ライブラリ: `actions/checkout@v6` の sparse-checkout 構文 → `ci.yaml` で使用なし、公式ドキュメントで確認要
- [x] パターン: permissions の指定方法 → `weekly-retro.yaml` L14-15 で `issues: write` のみ。`actions/checkout` には `contents: read` が必要

### 変更内容

**1. `permissions` に `contents: read` を追加**

`actions/checkout` がリポジトリを読む必要がある。現在 `issues: write` のみ指定しているため、未指定の権限は `none` になる。

```yaml
permissions:
  contents: read
  issues: write
```

**2. `actions/checkout@v6` ステップを追加（sparse-checkout）**

レポートファイルのみ必要なため、sparse-checkout で `prompts/reports` に限定する。

```yaml
- name: Checkout reports
  uses: actions/checkout@v6
  with:
    sparse-checkout: prompts/reports
    sparse-checkout-cone-mode: true
```

設計判断: `sparse-checkout-cone-mode: true`（デフォルト）を使用。`prompts/reports` ディレクトリ全体を取得するにはcone モードが適切（パスパターンではなくディレクトリ指定のため）。

**3. 対象期間算出ロジックを前回レトロ基準に変更**

```yaml
- name: Calculate period from last retro
  id: period
  run: |
    TODAY=$(date +%Y-%m-%d)

    # 前回のレトロスペクティブレポートを検索（最新順）
    LAST_RETRO=$(find prompts/reports -maxdepth 1 -name '*_レトロスペクティブ.md' 2>/dev/null | sort -r | head -1)

    if [ -n "$LAST_RETRO" ]; then
      # ファイル名から日付を抽出（YYYY-MM-DD_HHMM_レトロスペクティブ.md）
      START=$(basename "$LAST_RETRO" | cut -d'_' -f1)
    else
      # フォールバック: 7日前
      START=$(date -d "7 days ago" +%Y-%m-%d)
    fi

    {
      echo "start=${START}"
      echo "end=${TODAY}"
      echo "today=${TODAY}"
    } >> "$GITHUB_OUTPUT"
```

設計判断:
- 開始日を前回レトロ日（当日含む）にする。`/retro` スキルと同じ方式（レポート `2026-02-13` なら期間は `2026-02-13 〜` ）
- 終了日を `TODAY` に変更（旧: 前週日曜 → 新: ワークフロー実行日）
- フォールバックは「7日前」。初回実行やレポートが存在しない場合の安全策

**4. step の `id` を `week` → `period` に変更**

固定週ウィンドウではなくなったため、id 名を汎用的に。Issue 作成ステップの参照（`steps.week.outputs.*`）も `steps.period.outputs.*` に更新。

### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

検証:
- [ ] `act` または手動 `workflow_dispatch` でワークフローを実行し、生成された Issue の対象期間が前回レトロ日〜今日であることを確認
- [ ] フォールバック（レポートなしの場合）は実環境で検証困難なため、ロジックのレビューで代替

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `permissions` に `contents: read` が必要 | 未定義 | permissions セクションに `contents: read` を追加 |
| 2回目 | step id `week` が意味と乖離 | シンプルさ | `period` にリネームし参照箇所も更新 |
| 3回目 | `sparse-checkout-cone-mode` の適切な値 | 技術的前提 | cone mode（デフォルト）がディレクトリ指定に適切。明示的に `true` を指定 |
| 4回目 | `ls` の glob が日本語ファイル名を扱えるか | 技術的前提 | Ubuntu ランナーはデフォルト UTF-8。`git config core.quotePath false` で対応可能だが sparse-checkout 済みなので `ls` は worktree 上のファイルを直接参照。問題なし |
| 5回目 | `ls` が ShellCheck SC2012 に抵触（actionlint 経由で CI 失敗） | 既存手段の見落とし | `ls` → `find -maxdepth 1 -name` に変更。計画のコードスニペットも更新 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | ワークフローの全変更箇所が計画に含まれている | OK | permissions, checkout, 算出ロジック, step id, Issue 本文参照の 5 箇所 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | コードスニペットで全変更内容を具体的に記載 |
| 3 | 設計判断の完結性 | 全判断に理由記載 | OK | cone mode, 開始日方式, step id の 3 判断を記載 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | `monthly-assess.yaml` とスキル SKILL.md を対象外に明記 |
| 5 | 技術的前提 | 前提が考慮されている | OK | permissions, UTF-8, sparse-checkout を確認 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | `/retro` スキルの期間算出方式と一致 |
