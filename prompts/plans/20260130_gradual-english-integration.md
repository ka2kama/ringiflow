# 計画: 開発ワークフローへの段階的英語統合

## 目的

RingiFlow の開発ワークフローに英語ライティング練習を自然に統合する。
日本語を基本言語として維持しつつ、3つの施策を導入する。

## 施策

| # | 施策 | 対象 | 効果 |
|---|------|------|------|
| 1 | コミットメッセージの英語化 | Writing 力向上 | 短い定型文で反復練習 |
| 2 | PR タイトルの英語化 | Writing 力向上 | コミットと同様の定型文練習 |
| 3 | Insight ブロックに英語サマリー追加 | Reading 力向上 | 技術英語への反復的接触 |

## 変更ファイル

### 1. `CLAUDE.md`

#### 変更箇所 A: 言語セクション（現在7行目付近）

**現在:**
```
常に日本語で応答する。コミットメッセージ、コメント、ドキュメントも日本語で記述する。
```

**変更後:**
```
常に日本語で応答する。コメント、ドキュメントも日本語で記述する。

### 段階的英語化

開発ワークフローを通じた英語力向上のため、以下は英語で記述する:

- **コミットメッセージ**: 英語で記述する
- **PR タイトル**: 英語で記述する

それ以外（コードコメント、ドキュメント、PR 本文、応答）は引き続き日本語。
```

#### 変更箇所 B: コミットメッセージセクション（現在308行目付近）

**現在:**
```bash
git commit -m "#34 UserRepository: find_by_email を実装"
```

**変更後:**
```bash
git commit -m "#34 Implement find_by_email for UserRepository"
```

英語コミットでよく使う動詞パターンのリファレンスを追加:

| 動詞 | 用途 | 例 |
|------|------|-----|
| Implement | 新規実装 | Implement user authentication |
| Add | 追加 | Add validation to login form |
| Fix | バグ修正 | Fix null pointer in session handler |
| Update | 変更・改善 | Update error messages for clarity |
| Refactor | リファクタリング | Refactor session management logic |
| Remove | 削除 | Remove deprecated API endpoint |
| Rename | リネーム | Rename Session to Shared |

#### 変更箇所 C: PR 作成セクション（現在320行目付近）

PR タイトルの例を英語に変更:

**現在:**
```bash
gh pr create --draft --title "#34 ログイン機能を実装" ...
```

**変更後:**
```bash
gh pr create --draft --title "#34 Implement login feature" ...
```

PR 本文は引き続き日本語で記述する（変更なし）。

#### 変更箇所 D: 学習支援セクション

Insight ブロックに英語サマリーを付けるルールを追加:

```
### 英語サマリー（Insight ブロック）

Insight ブロックには、日本語の解説に加えて 1〜2 文の英語サマリーを必ず付ける:

★ Insight ─────────────────────────────────────
[日本語の教育的ポイント 2-3 点]

📝 In English: [1-2 sentence summary of the key takeaway]
─────────────────────────────────────────────────
```

### 2. `docs/04_手順書/04_開発フロー/01_Issue駆動開発.md`

コミットメッセージと PR タイトルの例を英語に更新する:

- 96行目付近: `#34 WIP: ログイン機能を実装` → `#34 WIP: Implement login feature`
- 98行目付近: `#34 ログイン機能を実装` → `#34 Implement login feature`
- 254行目付近: コミットメッセージ例を英語に変更
- その他、日本語コミット例が出現する箇所を英語に置換

説明テキスト自体は日本語のまま維持する。

## 変更不要なファイル

| ファイル | 理由 |
|---------|------|
| `.github/pull_request_template.md` | セクション名は既に英語。本文は日本語維持 |
| `.lefthook/prepare-commit-msg/add-issue-number.sh` | 言語に依存しない。英語メッセージでもそのまま動作 |
| `.claude/rules/*` | 既存ルールファイルは特定技術領域向け。英語化ルールは CLAUDE.md に集約 |
| コードコメント | 今回のスコープ外（将来的な検討事項） |

## 検証方法

1. 変更後の CLAUDE.md を通読し、一貫性を確認
2. lefthook の `prepare-commit-msg` が英語メッセージでも正常に Issue 番号を付与するか確認
3. 新しい開発セッションで Insight ブロックに英語サマリーが含まれるか確認
4. 実際にコミット・PR 作成を行い、英語フォーマットが適用されるか確認
