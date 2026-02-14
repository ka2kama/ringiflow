# review-and-merge ゼロ検証追加

## 概要

`/review-and-merge` スキルの Step 5（マージ前）に未対応レビュー指摘のゼロ検証を追加した。合わせて Step 4 を拡張し、PR コメントの指摘にも対応フローを適用するようにした。Issue #497 の対応。

## 実施内容

### Step 4 の拡張

- 対象を inline comment（Review comment）のみから、PR コメント（全体フィードバック）の指摘も含むように拡大
- PR コメントへの返信テンプレートを追加。スレッド構造がないため、元コメントの URL を含めて関連性を明示する方式を採用

### Step 5 に「未対応レビュー指摘のゼロ検証」を追加

マージ前に以下の 3 項目を検証するステップを追加:

1. `reviewDecision` が APPROVED
2. 未 resolve の review threads がゼロ（inline comment）
3. claude[bot] の PR コメントに指摘を含む未返信コメントがゼロ

セッション復元の有無に関わらず常に API を再取得する構造にし、前セッションの結論を鵜呑みにする問題を構造的に解消した。

## 判断ログ

- PR コメントの対応済み判断方法について 3 案（返信有無、指摘解析、reviewDecision で代替）を検討し、Step 4 での返信義務化 + Step 5 での返信有無検証を採用した。理由: フロー組み込みにより、セッション復元時の再検証を自動化できる

## 成果物

- コミット: `#497 Add pre-merge zero-verification for unresolved review comments`
- 変更ファイル:
  - `.claude/skills/review-and-merge/SKILL.md` — Step 4 拡張、Step 5 ゼロ検証追加
  - `prompts/improvements/2026-02/2026-02-13_1148_セッション復元時のレビューコメント検証省略.md` — 次のアクションを完了済みに更新
- Draft PR: #519

## 議論の経緯

- ユーザーから「inline comment は resolve するが、PR comment の場合もある」と指摘があり、PR コメントの対応フローも検討対象に追加した
- PR コメントの対応済み判断方法について、指摘なしのサマリーにも返信が必要かという質問から、「指摘を含む PR コメントのみ返信を義務化」する方針に収束した
