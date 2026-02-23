# #773 生成物の直接編集を防止するガード追加

## 概要

VCS 管理の生成物（openapi.yaml、.sqlx/、schema.sql、snapshots/）の対応表をルールファイルとして新規作成し、`pre-implementation.md` に「生成物の確認」ゲートセクションを追加した。

## 実施内容

### Phase 1: 生成物対応表ルールファイルの作成

- `.claude/rules/generated-files.md` を新規作成
- front matter の `paths` で生成物パスを指定し、対象ファイル編集時に自動ロードされるよう設定
- 生成物 4 件の対応表（ソースオブトゥルース、生成コマンド）を記載

### Phase 2: pre-implementation.md にガード追加

- 「生成物の確認（最優先ゲート）」セクションを「原則」と「確認の3カテゴリ」の間に追加
- 「AI エージェントへの指示」と「禁止事項」にも対応する項目を追記

### Issue 精査での追加発見

- Issue 本文には 3 件の生成物が記載されていたが、探索で `backend/schema.sql`（`just db-dump-schema` で生成）を追加で特定し、対応表に含めた

## 判断ログ

- 対応表の配置場所: `pre-implementation.md` に直接記載（Option A）ではなく、別ファイル `generated-files.md` に分離（Option B）を選択。理由: 責務分離、front matter の paths による自動ロード、単独メンテナンス
- `api.md` の OpenAPI セクション（手動編集前提の記述が残存）の更新はスコープ外とし、別 Issue で対応する方針

## 成果物

- PR: https://github.com/ka2kama/ringiflow/pull/811
- コミット: `#773 Add generated files guard to pre-implementation rules`
- 新規: `.claude/rules/generated-files.md`
- 修正: `.claude/rules/pre-implementation.md`
- 計画: `prompts/plans/773_generated-files-guard.md`
