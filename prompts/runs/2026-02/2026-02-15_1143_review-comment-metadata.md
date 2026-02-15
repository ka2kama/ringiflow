# セッションログ: レビューコメントメタデータの構造化

## 概要

Issue #521 の実装。Claude Code Action（Auto Review と Rules Check）のレビューコメントに、機械判定可能なメタデータを埋め込む構造を追加した。`/review-and-merge` スキルがメタデータを使って指摘の有無を機械的に判定できるようになる。

## 実施内容

### Phase 1: Auto Review のメタデータ埋め込み

`.github/workflows/claude-auto-review.yaml` のプロンプトに「メタデータ埋め込み（必須）」セクションを追加。

- メタデータのフォーマット、配置場所、フォーマット例を明記
- 指摘件数のカウント方法を説明
- 重大度別（Critical, High, Medium, Low）の件数を記録

### Phase 2: Rules Check のメタデータ埋め込み

`.github/workflows/claude-rules-check.yaml` のプロンプトに「メタデータ埋め込み（必須）」セクションを追加。

- Rules Check では Critical のみを使用（ルール違反はすべて修正必須）
- 既存の `<!-- rules-check-result:pass/fail -->` マーカーと併用

### Phase 3: `/review-and-merge` スキルの Step 3 修正

レビューコメント取得時にメタデータを抽出し、サマリーに含めるよう修正。

- メタデータ抽出の手順を追加（grep による抽出）
- サマリー提示のフォーマットを更新（重大度別件数、対応要否を表示）
- 後方互換性の確保（メタデータがない場合は従来通り本文を解釈）

### Phase 4: `/review-and-merge` スキルの Step 5 修正

検証 #3 をメタデータによる判定に変更。

- 従来: コメント内容を読んで AI が指摘事項を含むか判定（AI の解釈に依存）
- 変更後: メタデータから指摘件数を抽出して機械的に判定（確実性の向上）
- 判定ロジックの疑似コードを追加

## 判断ログ

### メタデータフォーマットの選択

HTML コメント（Key-Value 形式）を採用した。

理由:

1. Rules Check で既に `<!-- rules-check-result:pass/fail -->` を使用している（整合性）
2. Markdown レンダリング時に非表示になり、人間の可読性を損なわない
3. 単純な正規表現・grep で抽出可能
4. 将来的な項目追加が容易（Key-Value 形式）

代替案:

- YAML フロントマター: Markdown 本文の先頭に配置する必要があり、既存コメントとの統合が困難
- JSON: 可読性が低く、手動編集時にエラーが発生しやすい

### 配置場所

コメント本文の末尾に配置。

理由: 既存の `<!-- rules-check-result:pass/fail -->` と同じ位置で、人間が読むメインコンテンツとメタデータを分離できる。

## 成果物

### 修正ファイル

- `.github/workflows/claude-auto-review.yaml` — メタデータ埋め込み指示を追加（79行）
- `.github/workflows/claude-rules-check.yaml` — メタデータ埋め込み指示を追加（74行）
- `.claude/skills/review-and-merge/SKILL.md` — Step 3 と Step 5 の修正（66行）

### 計画ファイル

- `prompts/plans/groovy-wiggling-cook.md` → `prompts/plans/521_review-comment-metadata.md` にリネーム予定

### PR

- Draft PR #538: https://github.com/ka2kama/ringiflow/pull/538
- コミット: 843f677

## 検証方法

手動テスト（E2E）:

1. Auto Review のメタデータ埋め込み確認: 新しい PR で CI 完了後、PR コメントにメタデータが埋め込まれているか確認
2. Rules Check のメタデータ埋め込み確認: ルール違反を含む PR で CI 完了後、PR コメントにメタデータが埋め込まれているか確認
3. `/review-and-merge` スキルでの解析確認: メタデータありの PR で実行し、メタデータが正しく抽出・表示されるか確認
4. 後方互換性確認: 古い PR（メタデータなし）で実行し、フォールバックが動作するか確認
