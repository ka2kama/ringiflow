# Issue 駆動開発

## 概要

このプロジェクトでは GitHub Projects + Issue 駆動で開発を進める。
タスクを Issue として管理し、PR と紐づけることで変更の追跡性を確保する。

採用理由: [ADR-012: Issue 駆動開発の採用](../../05_ADR/012_Issue駆動開発の採用.md)

## リソース

| リソース | URL |
|---------|-----|
| Project Board | https://github.com/users/ka2kama/projects/1 |
| Milestones | https://github.com/ka2kama/ringiflow/milestones |
| Issues | https://github.com/ka2kama/ringiflow/issues |

## 開発フロー

```mermaid
flowchart LR
    A["Issue 作成"] --> B["ブランチ作成"]
    B --> C["Draft PR 作成"]
    C --> D["設計"]
    D --> E["実装（TDD）"]
    E --> F["Ready for Review"]
    F --> G["レビュー確認"]
    G --> H["マージ"]
    H --> I["Issue 自動クローズ"]
```

### 1. Issue を確認または作成

```bash
# Issue 一覧を確認
gh issue list

# 新しい Issue を作成
gh issue create --title "機能名" --body "説明" --milestone "Phase 1: MVP" --label "backend"
```

Issue には以下を含める:
- 概要: 何を実装するか
- 完了基準: 何ができたら完了か（チェックリスト形式）
- 参照: 関連する要件ID、設計書へのリンク

### 2. ブランチを作成

```bash
# Issue 番号に基づいてブランチを作成
git checkout -b feature/34-user-auth
```

命名規則:
- `feature/{issue番号}-{機能名}` — 新機能
- `fix/{issue番号}-{バグ名}` — バグ修正

### 3. Draft PR を作成

ブランチ作成後、すぐに Draft PR を作成する。

```bash
# 空コミットで Draft PR を作成
git commit --allow-empty -m "#34 WIP: ログイン機能を実装"
git push -u origin HEAD
gh pr create --draft --title "#34 ログイン機能を実装" --body "Closes #34"
```

**Draft PR の目的:**
- 作業中であることを可視化
- PR Description に設計メモや検討事項を記録
- 必要に応じて `@claude` メンションで相談

**注意:** Draft PR では自動レビューは実行されない。Ready for Review に変更した時点でレビューが走る。

採用理由: [ADR-013: Draft PR 運用の導入](../../05_ADR/013_Draft_PR運用の導入.md)

### 4. 設計

**実装前に必ず設計フェーズを経る。** コードを書く前に「何を作るか」「どう作るか」を明確にする。

```mermaid
flowchart TB
    A["基本設計確認"] --> B["詳細設計作成"]
    B --> C["実装計画作成"]
    C --> D["Issue 更新"]
```

#### 4.1 基本設計確認

既存の基本設計書（`docs/02_基本設計書/`）を確認し、アーキテクチャ上の位置づけを把握する。

- 新しいコンポーネントが必要か
- 既存コンポーネントとの関係は
- データフローはどうなるか

基本設計の変更が必要な場合は、設計書を先に更新する。

#### 4.2 詳細設計作成

機能の詳細設計を `docs/03_詳細設計書/` に作成する。

含める内容:
- アーキテクチャ図（Mermaid）
- インターフェース定義（trait、API）
- データ構造
- シーケンス図（必要に応じて）

**API を含む機能の場合:**

OpenAPI 仕様書（`openapi/openapi.yaml`）を更新する。OpenAPI が Single Source of Truth。

- 新しいエンドポイントを追加
- リクエスト/レスポンススキーマを定義
- エラーレスポンスを定義

#### 4.3 実装計画作成

Issue 本文に実装計画を追記する。

**フォーマット:**

```markdown
## 実装計画

TDD（Red → Green → Refactor）で MVP を積み上げる。

### Phase 1: コンポーネント名

**テストリスト:**
- [ ] 正常系のテスト
- [ ] 異常系のテスト
- [ ] 境界値のテスト

### Phase 2: 次のコンポーネント名
...
```

**Phase 分割の基準:**
- 依存順（下位レイヤーから上位レイヤーへ）
- 詳細: [TDD 開発フロー > MVP 積み上げ方式](./02_TDD開発フロー.md#mvp-積み上げ方式)

**テストリストの書き方:**
- 1テスト = 1つの振る舞い
- 正常系 → 異常系 → 境界値の順
- 詳細: [TDD 開発フロー > テストリスト](./02_TDD開発フロー.md#テストリスト)

**参考例:** [Issue #34: ユーザー認証](https://github.com/ka2kama/ringiflow/issues/34)

#### 4.4 設計成果物のコミット

設計フェーズで作成・更新したドキュメントをコミットする。

```bash
git add docs/03_詳細設計書/ openapi/
git commit -m "#34 ログイン機能の詳細設計を追加"
```

設計と実装を分けてコミットすることで:
- 設計レビューを先に受けられる
- 実装中に設計変更があった場合、差分が明確になる

#### 設計フェーズのスキップ

以下の場合は設計フェーズを簡略化できる:
- 単純なバグ修正
- 既存パターンの踏襲（設計済み機能の追加実装）
- ドキュメント修正

### 5. 実装（TDD）

TDD（テスト駆動開発）で実装を進める。詳細は [TDD 開発フロー](./02_TDD開発フロー.md) を参照。

```
Red → Green → Refactor を繰り返す
```

#### コミットの粒度

**セーブポイントを積み上げるようにコツコツとコミットする。**

| タイミング | 例 |
|-----------|-----|
| テストが通ったとき | `UserRepository: find_by_email のテストを追加` |
| リファクタリング完了時 | `UserRepository: エラーハンドリングを整理` |
| 1つの機能単位が完成したとき | `UserRepository を実装` |

**良いコミット:**
- 小さく、1つの目的に集中
- テストが通る状態でコミット（壊れた状態を残さない）
- 後から履歴を追いやすい

**避けるべきコミット:**
- 「WIP」のまま長時間放置
- 複数の無関係な変更を1つにまとめる
- テストが落ちる状態でコミット

コミットメッセージの先頭に Issue 番号を含める（GitHub 上でリンクされる）。

```bash
git commit -m "#34 UserRepository: find_by_email を実装"
```

**自動付与:** lefthook がブランチ名（`feature/34-xxx`）から Issue 番号を抽出して自動で先頭に追加する。手動で書く必要はない。

#### Issue の進捗更新

**Phase やタスクが完了したら、Issue のチェックボックスを都度更新する。**

```bash
# Issue の本文を更新（チェックボックスを [x] に変更）
gh issue edit 34 --body "$(cat <<'EOF'
## 実装計画

### Phase 1: UserRepository ✅

**テストリスト:**
- [x] メールアドレスでユーザーを取得できる
- [x] 存在しないメールアドレスの場合 None を返す
...
EOF
)"
```

**なぜ都度更新が重要か:**
- 進捗が可視化され、チームメンバーや将来の自分が状況を把握できる
- 完了した作業と残りの作業が明確になる
- Project Board の進捗と Issue の詳細が一致する

### 6. Ready for Review

実装が完了したら、Draft PR を Ready for Review に変更する。

```bash
gh pr ready
```

これにより Claude Code Action の自動レビューが実行される。

### 7. レビュー確認

Claude Code Action による自動レビューが実行される。

1. レビューコメントを確認
2. 指摘があれば修正をコミット
3. 修正不要な指摘は理由を返信してから resolve

レビュー方針: [CLAUDE.md の PRレビュー](../../../CLAUDE.md#prレビュー)

### 8. マージ

レビュー確認後、手動でマージする。

```bash
gh pr merge --squash --delete-branch
```

**注意:** `--auto` は使用しない。レビュー結果を確認してからマージすること。

## Milestone

Phase ごとに Milestone を作成している。Issue 作成時に適切な Milestone を設定する。

| Milestone | 状態 |
|-----------|------|
| Phase 0: 基盤構築 | 完了 |
| Phase 1: MVP | 進行中 |
| Phase 2: 機能拡張 | 未着手 |
| Phase 3: エンタープライズ機能 | 未着手 |
| Phase 4: 高度な機能・最適化 | 未着手 |

```bash
# Milestone の進捗を確認
gh api repos/ka2kama/ringiflow/milestones --jq '.[] | "\(.title): \(.open_issues) open, \(.closed_issues) closed"'
```

## Label

| Label | 用途 | 色 |
|-------|------|-----|
| `backend` | Rust / API 関連 | 青 |
| `frontend` | Elm / UI 関連 | 緑 |
| `infra` | Docker / Terraform / AWS | 紫 |
| `docs` | ドキュメント | 水色 |
| `priority:high` | 優先度: 高 | 赤 |
| `priority:medium` | 優先度: 中 | 黄 |
| `priority:low` | 優先度: 低 | 緑 |

## Project Board

Project Board はカンバン形式でタスクを可視化する。

| カラム | 意味 |
|--------|------|
| No Status | 未分類 |
| Todo | 着手前 |
| In Progress | 作業中 |
| Done | 完了 |

Issue を作成すると自動的に Project に追加される（`--project "RingiFlow"` オプション使用時）。

## Issue の粒度

- 大きすぎる Issue は分割する（目安: 1日〜数日で完了できる単位）
- 小さなタスクは Issue 内のチェックリストで管理する

良い例:
```markdown
## 完了基準

- [ ] POST /auth/login でログインできる
- [ ] POST /auth/logout でログアウトできる
- [ ] GET /auth/me で現在のユーザー情報を取得できる
- [ ] フロントエンドでログイン画面が動作する
```

## よく使うコマンド

```bash
# Issue 一覧
gh issue list

# Issue 詳細
gh issue view 34

# Issue を作成して Project に追加
gh issue create --title "タイトル" --milestone "Phase 1: MVP" --label "backend" --project "RingiFlow"

# PR 一覧
gh pr list

# PR の状態確認
gh pr checks

# Milestone 一覧
gh api repos/ka2kama/ringiflow/milestones
```

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-17 | lefthook による Issue 番号の自動付与を追加 |
| 2026-01-17 | コミットメッセージ・PR タイトルの先頭に Issue 番号を含める形式に統一 |
| 2026-01-17 | Issue の進捗更新ルールを追加 |
| 2026-01-17 | Draft PR 運用を導入（ADR-013） |
| 2026-01-17 | ブランチ作成を設計の前に移動（設計成果物をコミットするため） |
| 2026-01-17 | 設計フェーズを追加、TDD 開発フローへのリンクを追加、コミット粒度を追加、レビュー確認ステップを追加 |
| 2026-01-16 | 初版作成 |
