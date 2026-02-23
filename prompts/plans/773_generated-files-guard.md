# #773 生成物（openapi.yaml）を直接編集しようとするパターンの対策

## Context

#769 の実装中、utoipa から自動生成される `openapi/openapi.yaml` を手動で編集してしまった。`just openapi-generate` で上書きされ、無駄な作業が発生した。根本原因は「ファイルが生成物かどうかを確認するプロセスが存在しない」こと。

改善記録: `process/improvements/2026-02/2026-02-22_2108_生成物の直接編集.md`

## 対象

- `.claude/rules/generated-files.md`（新規作成）
- `.claude/rules/pre-implementation.md`（修正）

## 対象外

- `.claude/rules/api.md` の OpenAPI セクション更新（Code First 移行後の記述に未対応だが、別 Issue で対応）

## Phase 1: 生成物対応表ルールファイルの作成

`.claude/rules/generated-files.md` を新規作成する。

front matter の `paths` で生成物のパスを指定し、生成物に触れるコンテキストで自動ロードされるようにする。

```markdown
---
paths:
  - "openapi/**"
  - "backend/.sqlx/**"
  - "backend/schema.sql"
  - "backend/apps/bff/tests/snapshots/**"
---

# VCS 管理の生成物

VCS にコミットされているが、手動編集してはならない生成物の一覧。

## 対応表

| 生成物 | ソースオブトゥルース | 生成コマンド |
|--------|-------------------|-------------|
| `openapi/openapi.yaml` | Rust 構造体の utoipa アノテーション | `just openapi-generate` |
| `backend/.sqlx/` | SQL クエリマクロ | `just sqlx-prepare` |
| `backend/schema.sql` | PostgreSQL スキーマ（マイグレーション） | `just db-dump-schema` |
| `backend/apps/bff/tests/snapshots/*.snap` | テスト実行結果 | テスト実行 → snapshot 更新 |

## ルール

- 対応表のファイルを直接編集しない。ソースオブトゥルース側を変更してから生成コマンドを実行する
- 新しい生成パイプラインを追加した場合は対応表を更新する

改善の経緯: [生成物の直接編集](../../process/improvements/2026-02/2026-02-22_2108_生成物の直接編集.md)
```

#### 確認事項

- パターン: 既存ルールファイルの front matter → `api.md`, `repository.md` 等で確認済み
- 文体: `rule-skill-writing.md` の指示的・簡潔な文体に従う

#### 操作パス: 該当なし（操作パスが存在しない）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: pre-implementation.md にガードセクション追加

`.claude/rules/pre-implementation.md` の「原則」セクションの後、「確認の3カテゴリ」セクションの前に「生成物の確認」を追加する。

追加内容:

```markdown
## 生成物の確認（最優先ゲート）

ファイルを変更する前に「このファイルは生成物か？」を確認する。

確認手順:
1. 変更対象ファイルが [VCS 管理の生成物](generated-files.md) の対応表に含まれるか確認する
2. 生成物の場合: ソースオブトゥルース側を変更し、生成コマンドを実行する
3. 生成物でない場合: 以下の3カテゴリの確認に進む
```

また、既存の「AI エージェントへの指示」と「禁止事項」セクションにも追記する:

AI エージェントへの指示に追加:
```
- ファイル変更前: 変更対象が [VCS 管理の生成物](generated-files.md) に含まれるか確認する
```

禁止事項に追加:
```
- 生成物を直接編集すること（ソースオブトゥルースを変更して再生成する）
```

#### 確認事項

- 型: `pre-implementation.md` の現在の構造 → 読み込み済み（94行）

#### 操作パス: 該当なし（操作パスが存在しない）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## 設計判断

### 対応表の配置場所

選択肢:
- A: `pre-implementation.md` に直接記載
- B: 別ファイル `generated-files.md` を作成してリンク（推奨）

B を選択した理由:
- 責務分離: 「実装前の確認手順」と「生成物の一覧管理」は異なる責務
- front matter の `paths` で生成物パスにマッチさせ、対象ファイル編集時に自動ロードできる
- 対応表の単独メンテナンスが可能

### api.md の不整合

`api.md` の「1. OpenAPI 仕様書の更新」セクションは手動編集前提の記述が残っている。Code First 移行後は utoipa アノテーション変更 + 再生成が正しいフロー。ただし、#773 の Want（生成物確認プロセスの欠如対策）とは別の問題（ドキュメント整合性）のため、別 Issue で対応する。

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `backend/schema.sql` が Issue 本文の対応表に含まれていない | 網羅性 | 探索で発見し対応表に追加 |
| 2回目 | `api.md` の OpenAPI セクションが手動編集前提で `generated-files.md` と矛盾しうる | アーキテクチャ不整合 | スコープ外とし別 Issue で対応する方針を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の対応内容がすべて計画に含まれている | OK | (1) 対応表の明記 → Phase 1、(2) pre-implementation ガード追加 → Phase 2 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 追加位置・内容が具体的に記載済み |
| 3 | 設計判断の完結性 | 配置場所の判断が記載されている | OK | Option A/B の検討と選択理由を明記 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: 2ファイル、対象外: api.md 更新 |
| 5 | 技術的前提 | front matter パターンが確認されている | OK | api.md, repository.md 等で確認済み |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 改善記録の対策と一致。api.md の不整合は別 Issue |

## 検証

- `just check-all` でリント・テストが通ること（ルールファイルの追加・修正はコードに影響しないが、念のため確認）
