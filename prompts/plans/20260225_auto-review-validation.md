# Auto Review に Validation（書かれていないことの検出）を追加

## Context

現在の Auto Review は diff ベースの Verification（バグ、セキュリティ、設計等）のみを実施している。
「書かれていないこと」— 実装すべきだが実装されていない（欠落）、仕様と実装が一致していない（乖離）— はレビューの対象外。
品質ゲート（実装者セルフチェック）が Validation を担っているが、自動レビューによる補完がない。

Auto Review に Validation パスを追加し、Issue の完了基準・PR 本文・計画ファイルを参照して欠落・乖離を検出する。

## 設計判断

会話で合意済みの方針:

| 判断 | 方向性 |
|------|--------|
| 役割 | 補完型 — 品質ゲートが主、Auto Review は漏れを補う |
| 情報源 | Issue の完了基準 + PR 本文 + 計画ファイル |
| 伝え方 | Verification と Validation の指摘を分離。Validation は approve 維持 |

## 変更対象

`.github/workflows/claude-auto-review.yaml`（1ファイルのみ）

## 変更内容

### Phase 1: ワークフローステップの追加

`Build review context`（L114-155）と `Run Claude Code Review`（L157）の間に新ステップを追加。

#### ステップ: Fetch validation context

以下の情報を取得してプロンプトに注入する:

1. PR 本文（`gh pr view --json body`）
2. PR 本文から Issue 番号を抽出（`Closes #NNN` パターン）
3. 該当 Issue の本文を取得（`gh issue view`）
4. 計画ファイルの検出と内容取得（`prompts/plans/{Issue番号}_*.md`）

```yaml
- name: Fetch validation context
  id: validation-context
  if: steps.check-draft.outputs.is_draft == 'false'
  env:
    GH_TOKEN: ${{ github.token }}
  run: |
    PR_NUMBER=${{ github.event.workflow_run.pull_requests[0].number }}

    # PR 本文を取得
    PR_BODY=$(gh pr view "$PR_NUMBER" --repo "${{ github.repository }}" --json body --jq '.body // ""')

    # Closes #NNN パターンから Issue 番号を抽出
    ISSUE_NUMBERS=$(echo "$PR_BODY" | grep -oP '(?i)(?:closes|close|fixes|fix)\s+#\K\d+' || true)

    {
      echo "VALIDATION_CONTEXT<<EOF_VALIDATION"

      echo "### PR 本文"
      echo ""
      echo "$PR_BODY"
      echo ""

      if [ -n "$ISSUE_NUMBERS" ]; then
        for NUM in $ISSUE_NUMBERS; do
          echo "### Issue #${NUM}（完了基準の参照元）"
          echo ""
          gh issue view "$NUM" --repo "${{ github.repository }}" --json title,body \
            --jq '"タイトル: \(.title)\n\n\(.body // "(本文なし)")"' 2>/dev/null || echo "(取得失敗)"
          echo ""

          # 計画ファイルの検出
          for FILE in prompts/plans/${NUM}_*.md; do
            if [ -f "$FILE" ]; then
              echo "### 計画ファイル: $(basename "$FILE")"
              echo ""
              head -c 10000 "$FILE"
              FILESIZE=$(wc -c < "$FILE")
              if [ "$FILESIZE" -gt 10000 ]; then
                echo ""
                echo "(... ${FILESIZE} bytes 中 10000 bytes を表示。全文は cat prompts/plans/$(basename "$FILE") で確認可能 ...)"
              fi
              echo ""
            fi
          done
        done
      else
        echo "(Issue 参照なし — Validation チェックは PR 本文の品質確認セクションのみで実施)"
      fi

      echo "EOF_VALIDATION"
    } >> "$GITHUB_OUTPUT"
```

サイズ考慮:
- Issue 本文: 2-5KB（通常）
- 計画ファイル: 先頭 10KB に制限（テストリスト・操作パスを含むのに十分）
- 合計追加: 最大 ~20KB。既存の diff（最大 50KB）と合わせても許容範囲

### Phase 2: allowedTools の拡張

```diff
- --allowedTools "Bash(cat:*),Bash(gh pr view:*),Bash(gh pr list:*),Bash(gh pr diff:*),Bash(gh pr checks:*),Bash(gh pr comment:*),Bash(gh pr review:*),Bash(git log:*),Bash(git diff:*),Bash(git show:*),Bash(git status:*),mcp__github_inline_comment__create_inline_comment"
+ --allowedTools "Bash(cat:*),Bash(gh pr view:*),Bash(gh pr list:*),Bash(gh pr diff:*),Bash(gh pr checks:*),Bash(gh pr comment:*),Bash(gh pr review:*),Bash(gh issue view:*),Bash(git log:*),Bash(git diff:*),Bash(git show:*),Bash(git status:*),mcp__github_inline_comment__create_inline_comment"
```

追加: `Bash(gh issue view:*)` — Claude が追加の Issue（親 Epic 等）を自主的に参照するため。

### Phase 3: プロンプトの変更

#### 3a. Validation コンテキストの注入

「レビューモード」セクションの後に追加:

```markdown
## Validation コンテキスト

以下は PR に関連する Issue、PR 本文、計画ファイルの情報です。
Validation チェックの入力として使用してください。

${{ steps.validation-context.outputs.VALIDATION_CONTEXT }}
```

#### 3b. レビュー観点の再構成

既存の「必須チェック」を「Verification チェック」に改名し、「Validation チェック」を追加:

```markdown
## レビュー観点（CLAUDE.md を補完）

### Verification チェック（コードの品質）

1. バグ・正確性: 実行時エラー、ロジックミス、エッジケースの見落とし
2. セキュリティ: 脆弱性、認証/認可の問題、機密データの露出
3. パフォーマンス: 明らかなボトルネック、N+1クエリ、メモリリーク
4. 型システムの活用: Rust/Elmの強みを活かしているか。状態によって有効なフィールドが異なる場合に型安全ステートマシン（ADR-054）が適用されているか
5. テスト: 新機能・バグ修正に対応するテストがあるか
6. 設計: 責務の混在、依存関係の方向違反、過度な複雑さ

注: ルール準拠チェック（.claude/rules/）は別ワークフローで実施。

### Validation チェック（書かれていないことの検出）

Validation コンテキストを参照し、以下を確認する。
Issue や計画ファイルが存在しない場合（ドキュメント修正等）、PR 本文の検証のみ実施する。

#### 欠落（Omission）の検出

Issue の完了基準に対して:
- 完了基準の各項目に対応する実装が diff に含まれているか
- 実装計画の Phase に対応する変更がすべて含まれているか

計画ファイルのテストリストに対して:
- テストリストの各項目に対応するテストコードが存在するか
- 操作パスの全分類（正常系・準正常系・異常系）がテストでカバーされているか

#### 乖離（Divergence）の検出

Issue・計画ファイルとの一致:
- 実装が完了基準の意図する動作と異なっていないか
- 計画で「対象外」とされた範囲に変更が含まれていないか

PR 本文の品質確認セクションの検証:
- 「設計・ドキュメント」で挙げられたファイルパスが実際に変更されているか
- 「テスト」の記載内容と実際のテストファイルの対応
- 「N/A（理由）」の理由の妥当性

#### 注意事項

- Validation は確信度が Verification より低い場合がある。確信度を明示すること
- 完了基準の「解釈の幅」を考慮し、字句一致ではなく意図レベルで判断する
- 実装者が意図的にスコープを調整している可能性を考慮する
```

#### 3c. フィードバック方法の更新

既存のフィードバック方法に、出力フォーマットの指示を追加:

```markdown
## フィードバック方法

以下の順序で実行する:

1. インラインコメント: コード固有の問題は `mcp__github_inline_comment__create_inline_comment` で該当箇所にコメント
2. 全体フィードバック（必須）: `gh pr comment` で PR コメントを投稿。以下の構成で記載する:

### 全体フィードバックの構成

```
## Verification（コード品質）

[Verification の指摘・サマリー]

## Validation（完了基準との整合）

[欠落・乖離の指摘。各指摘に確信度を付記]

### 確認済み
- [突合して問題なかった項目を簡潔に列挙]

### 欠落
- [severity] [指摘内容]（確信度: 高/中/低）

### 乖離
- [severity] [指摘内容]（確信度: 高/中/低）
```

Validation の指摘がない場合は「確認済み」のみ記載する。
Issue 参照がない場合は Validation セクション自体を「Issue 参照なし — PR 本文の品質確認セクションのみ検証」と記載する。

3. 承認/却下: `gh pr review` で承認判断を実行
```

#### 3d. 承認判断の更新

```markdown
## 承認判断

| カテゴリ | 重大度 | アクション |
|---------|--------|-----------|
| Verification: Critical/High | マージ前に必ず修正が必要 | `gh pr review --request-changes` |
| Verification: Medium/Low | 改善推奨だがマージ可能 | `gh pr review --approve` + コメント |
| Validation: 全重大度 | 確認推奨だがマージ可能 | `gh pr review --approve` + コメント |
| None | 問題なし | `gh pr review --approve` |

Validation 指摘は常に approve する。理由:
- Validation は「書かれていないこと」の検出であり、偽陽性の可能性がある
- 品質ゲート（実装者セルフチェック）が主であり、Auto Review は補完的な役割

ただし、Validation で完了基準のセキュリティ関連項目が完全に欠落している場合は、
Verification の Critical として扱い request-changes とする。

学習のための解説コメントは承認判断に影響しない。
問題がなければ、躊躇なく承認してください。
```

#### 3e. メタデータの拡張

```markdown
## メタデータ埋め込み（必須）

PR コメント投稿時には、本文の末尾に以下の形式でメタデータを埋め込むこと:

```html
<!-- review-metadata
type: auto-review
severity-critical: <数値>
severity-high: <数値>
severity-medium: <数値>
severity-low: <数値>
validation-omission: <数値>
validation-divergence: <数値>
action-required: true | false
-->
```

| フィールド | 型 | 説明 |
|-----------|-----|------|
| `severity-*` | int | Verification 指摘の件数（既存） |
| `validation-omission` | int | Validation: 欠落の指摘件数 |
| `validation-divergence` | int | Validation: 乖離の指摘件数 |
| `action-required` | boolean | Verification の Critical/High が 1 件以上なら true |

`action-required` の判定は Verification 指摘のみに基づく（後方互換性維持）。
```

## 対象外

- **review-and-merge スキルの変更**: メタデータの新フィールドは後方互換。既存の `severity-*` パーシングに影響なし。将来の改善で Validation 件数の表示を追加可能
- **claude-rules-check.yaml の変更**: 別ワークフローの責務。今回の対象外
- **品質ゲート（手順書）の変更**: 既存の品質ゲートプロセスは変更しない
- **Issue の作成**: この計画は会話内の設計検討。Issue 化はユーザーの判断に委ねる

## 確認事項

- ライブラリ: GitHub Actions `$GITHUB_OUTPUT` のヒアドキュメント記法 → ワークフロー L105-112 の既存パターンに従う
- パターン: メタデータスキーマ → `prompts/plans/521_review-comment-metadata.md` との整合確認
- パターン: PR テンプレートの Issue セクション → `.github/pull_request_template.md` L1-3（`Closes #123` パターン）

## 検証方法

1. **構文検証**: ワークフロー YAML の構文チェック（`actionlint` or GitHub Actions の構文チェック）
2. **動作検証**: テスト用 PR を作成し、Auto Review が以下を行うことを確認:
   - Issue の完了基準を参照したレビューコメントが出力される
   - Verification と Validation が分離された PR コメントが投稿される
   - メタデータに `validation-omission` と `validation-divergence` が含まれる
   - Validation 指摘のみの場合に approve が維持される
3. **後方互換性**: review-and-merge スキルが既存のメタデータフィールドを正しくパースできること
