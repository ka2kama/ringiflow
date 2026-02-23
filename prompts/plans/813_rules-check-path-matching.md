# Rules Check ワークフロー高速化: パスマッチング事前計算

## Context

Rules Check が Auto Review より遅くなることがある。原因は LLM が `.claude/rules/` 内の 19 個のルールファイル（計 ~3,600 行）を 1 つずつ読み、glob パターンマッチングを手動で行っているため。ファイル I/O の往復（ターン数）が支配的。

パスマッチングをシェルステップで事前計算し、マッチしたルールの内容をプロンプトに直接埋め込むことで、Claude のターン数を大幅に削減する。

## 対象ファイル

| ファイル | 変更内容 |
|---------|---------|
| `.github/scripts/match-rules.py` | **新規作成**: glob パスマッチングスクリプト |
| `.github/workflows/claude-rules-check.yaml` | ステップ追加 + プロンプト変更 |

## Phase 1: パスマッチングスクリプト

### `.github/scripts/match-rules.py`

入力: 変更ファイル一覧（テキストファイル、1 行 1 パス）
出力: マッチしたルールの名前リスト + 各ルールの本文（フロントマター除去済み）

処理:
1. 変更ファイル一覧を読み取る
2. `.claude/rules/*.md` を走査し、YAML フロントマターから `paths:` パターンを抽出
3. glob パターンを正規表現に変換して変更ファイルとマッチング
4. マッチしたルールの名前と本文を出力

glob-to-regex 変換ルール:
- `**/` → `(?:.+/)?`（0 個以上のディレクトリ）
- 末尾 `**` → `.*`（任意のパス）
- `*` → `[^/]*`（単一セグメント内）
- `.` → `\.`（リテラルドット）
- 対象パターン例: `**/*.rs`, `backend/apps/*/src/**/*.rs`, `justfile`, `**/*`

出力フォーマット:
```
マッチしたルール: N 件

- `.claude/rules/rust.md`
- `.claude/rules/lint.md`

### .claude/rules/rust.md

[ルール本文（フロントマター除去済み）]

### .claude/rules/lint.md

[ルール本文]
```

マッチ 0 件の場合: `<!-- no-matching-rules -->` を出力

#### 確認事項

- パターン: YAML フロントマターの形式 → `.claude/rules/api.md` 等で確認済み（`---` 区切り、`paths:` リスト、クォート付き文字列）
- パターン: `paths:` がないファイル（`problem-solving.md` 等 5 件）→ フロントマターなし、先頭が `#` で始まる → スキップ

#### テストリスト

ユニットテスト（該当なし — スクリプトの検証はワークフロー実行で実施）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## Phase 2: ワークフロー変更

### `.github/workflows/claude-rules-check.yaml`

#### 2a. 新ステップ追加（Checkout 後、Claude 実行前）

**"Get changed files" ステップ**:
```yaml
- name: Get changed files
  id: changed-files
  env:
    GH_TOKEN: ${{ github.token }}
  run: |
    PR_NUMBER=${{ github.event.workflow_run.pull_requests[0].number }}
    gh pr diff "$PR_NUMBER" --name-only > /tmp/changed-files.txt
```

**"Match rules" ステップ**:
```yaml
- name: Match rules to changed files
  id: match-rules
  run: |
    {
      echo "MATCHED_RULES<<EOF_MATCHED_RULES"
      python3 .github/scripts/match-rules.py /tmp/changed-files.txt
      echo "EOF_MATCHED_RULES"
    } >> "$GITHUB_OUTPUT"
```

両ステップとも `if: steps.check-draft.outputs.is_draft == 'false'` を付与。

#### 2b. Claude 実行ステップの条件追加

マッチ 0 件の場合は Claude を呼ばない:
```yaml
if: >
  steps.check-draft.outputs.is_draft == 'false' &&
  !contains(steps.match-rules.outputs.MATCHED_RULES, '<!-- no-matching-rules -->')
```

#### 2c. プロンプト変更

現在の手順:
```
1. gh pr diff で変更されたファイルパスを取得
2. .claude/rules/ 内の各ルールファイルを読み、paths: パターンとマッチするか確認
3. マッチしたルールファイルの内容に基づいて、変更がルールに準拠しているかチェック
```

変更後:
```
## マッチしたルール

以下のルールが今回の PR の変更ファイルにマッチしました。
ルールファイルを自分で読む必要はありません。以下の内容のみに基づいてチェックしてください。

${{ steps.match-rules.outputs.MATCHED_RULES }}

## 手順

1. `gh pr diff` で変更内容を確認
2. 上記のマッチしたルールに基づいて、変更がルールに準拠しているかチェック
3. 違反がある場合: インラインコメントで該当箇所を指摘（該当ルールを引用）
4. 全体フィードバック: `gh pr comment` でサマリーを投稿
```

#### 2d. claude_args 調整

```yaml
claude_args: |
  --model claude-sonnet-4-6
  --max-turns 20
  --allowedTools "Read,Bash(cat:*),Bash(gh pr view:*),Bash(gh pr diff:*),Bash(gh pr comment:*),Bash(git diff:*),mcp__github_inline_comment__create_inline_comment"
```

変更点:
- `--max-turns`: 40 → 20（ルールファイル読み取りが不要になったため）
- `--allowedTools`: `Bash(grep:*)` と `Bash(find:*)` を削除（ルールファイル探索が不要）

#### 確認事項

- パターン: 既存のステップ構造と `if` 条件 → ワークフローファイルで確認済み
- パターン: step output の heredoc 形式 → 既存の `pr-comments` ステップと同じパターン

#### 操作パス: 該当なし（CI ワークフローの変更、ユーザー操作なし）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

## 対象外

- ルールファイルの paths パターンの見直し（`**/*` の広すぎるマッチ等）
- Auto Review ワークフローの変更
- ルールの内容自体の変更

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | マッチ 0 件時に Claude を呼ぶ必要がない | 不完全なパス | Claude 実行ステップに `!contains(... no-matching-rules)` 条件を追加 |
| 1回目 | `--allowedTools` にルール探索用ツールが残る | シンプルさ | `Bash(grep:*)` と `Bash(find:*)` を削除 |
| 1回目 | `--max-turns 40` は過剰 | シンプルさ | 20 に削減 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | スクリプト新規作成 + ワークフロー変更の 2 ファイル |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | glob-to-regex 変換ルール、出力フォーマット、ステップ構造すべて具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | max-turns、allowedTools の変更理由を明記 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 「対象外」セクションあり |
| 5 | 技術的前提 | 前提が考慮されている | OK | ubuntu-latest の Python 3 利用可能、GitHub step output の heredoc 形式 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | 既存ワークフロー構造を維持、責務分離（Rules Check vs Auto Review）を変更しない |

## 検証方法

1. ローカルでスクリプトをテスト:
   ```bash
   echo "backend/apps/bff/src/handler/user.rs" > /tmp/test-files.txt
   python3 .github/scripts/match-rules.py /tmp/test-files.txt
   # → rust.md, api.md, structural-review.md 等がマッチすることを確認
   ```
2. PR を作成し、Rules Check ワークフローの実行を確認
3. 実行時間を以前の実行と比較
