# 計画: #625 Worktree を永続スロット方式に変更する

## Context

worktree を「Issue ごとに生成・破棄するモデル」から「永続スロット方式」に変更する。`ringiflow-1`, `ringiflow-2`, ... として固定 worktree を維持し、Issue 着手時はスロット内でブランチを切り替える。セットアップコスト（`pnpm install`、初回ビルド、DB マイグレーション）の繰り返しを排除し、ワークフローをシンプルにする。

## 対象

| ファイル | 種別 |
|---------|------|
| `scripts/worktree-create.sh` | 新規 |
| `scripts/worktree-switch.sh` | 新規 |
| `scripts/worktree-issue.sh` | 改修 |
| `scripts/cleanup.sh` | 改修 |
| `scripts/worktree-add.sh` | 廃止 |
| `justfile` | 変更 |
| `.claude/skills/next/SKILL.md` | 変更 |
| `.gitignore` | 変更 |
| `docs/60_手順書/04_開発フロー/04_並行開発（Worktree）.md` | 全面改訂 |
| `docs/70_ADR/021_並行開発環境の構成.md` | 補遺追加 |
| `docs/90_実装解説/03_並行開発環境/*.md` | 改訂 |
| `README.md` L223 | 軽微な更新 |

## 対象外

| ファイル | 理由 |
|---------|------|
| `scripts/generate-env.sh` | 変更不要。offset = slot 番号で決定的に呼び出すだけ |
| `scripts/setup-env.sh` | 変更不要。`.env` の存在チェック（L23-29）で no-op になる |
| `.claude/skills/restore/SKILL.md` | 変更不要。ブランチベースの Issue 特定なので影響なし |
| `.claude/settings.json` | 変更不要。`worktree-remove` は残存。新コマンドは安全 |

---

## Phase 1: 新規スクリプト + justfile コマンド

### `scripts/worktree-create.sh N`

永続スロットの初回作成。

1. N を検証（`^[1-9]$`）
2. `WORKTREE_PATH="${PARENT_DIR}/ringiflow-${N}"` を構築
3. 既存チェック → 存在すればエラー
4. `git fetch origin main --quiet`
5. `git worktree add --detach "$WORKTREE_PATH" origin/main`
6. `.worktree-slot` マーカーファイルを作成（内容: slot 番号）
7. `cd "$WORKTREE_PATH" && ./scripts/generate-env.sh "$N"`
8. `env -u POSTGRES_PORT -u REDIS_PORT -u DYNAMODB_PORT -u API_TEST_POSTGRES_PORT -u API_TEST_REDIS_PORT -u API_TEST_DYNAMODB_PORT -u BFF_PORT -u VITE_PORT just setup-worktree`

設計判断:
- **offset = slot 番号の決定的マッピング**: 自動検出より予測可能。slot 1 = offset 1
- **`--detach` で作成**: 「スロット作成」と「ブランチ割当」を分離。main は既にメイン worktree が使用中のため、detached HEAD が適切
- **`.worktree-slot` マーカー**: cleanup.sh が永続スロットを識別するために使用
- **`env -u` に API_TEST 変数を追加**: `worktree-add.sh` L125 で欠けていた変数もクリア

### `scripts/worktree-switch.sh N BRANCH`

スロット内のブランチ切り替え。

1. N を検証、スロットの存在確認（`.worktree-slot`）
2. 未コミット変更チェック → あれば拒否
3. 切り替え前の HEAD を記録: `PREV_HEAD=$(git -C ... rev-parse HEAD)`
4. ブランチ切り替え:
   - ローカルに存在 → `git -C "$WORKTREE_PATH" switch "$BRANCH"`
   - リモートのみ → `git -C "$WORKTREE_PATH" switch -c "$BRANCH" "origin/$BRANCH"`
   - 新規 → `git -C "$WORKTREE_PATH" switch -c "$BRANCH" origin/main`
5. DB マイグレーション: `cd "$WORKTREE_PATH" && env -u ... just db-migrate`
6. pnpm-lock.yaml 差分チェック: 差分があれば root + frontend + tests/e2e で `pnpm install`

設計判断:
- **未コミット変更の拒否**: stash 自動実行より安全。KISS を優先
- **pnpm-lock.yaml の差分チェック**: `PREV_HEAD..HEAD` で比較。失敗時（初回等）は pnpm install を実行（fail-open）
- **`git switch` のエラー伝播**: 他の worktree で使用中のブランチへの切り替えは git がエラーを返す。そのまま伝播させる

### justfile コマンド

```just
# 永続 worktree スロットを作成（初回のみ）
worktree-create n:
    ./scripts/worktree-create.sh {{n}}

# worktree スロット内のブランチを切り替え
worktree-switch n branch:
    ./scripts/worktree-switch.sh {{n}} {{branch}}
```

### 確認事項
- [ ] パターン: `worktree-add.sh` のスクリプト構造（set -euo pipefail, SCRIPT_DIR, PROJECT_ROOT） → `scripts/worktree-add.sh` L1-28
- [ ] パターン: `env -u` の変数リスト → `scripts/worktree-add.sh` L125
- [ ] ライブラリ: `git worktree add --detach <path> <commit-ish>` の動作確認 → git 公式ドキュメント
- [ ] ライブラリ: `git switch` の detached HEAD からの動作確認
- [ ] 型: `.gitignore` の `.worktree-slot` への影響 → `.gitignore` L147（`.env` パターン。`.worktree-slot` はマッチしないので追加が必要）

### テストリスト

ユニットテスト（該当なし -- シェルスクリプト）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

手動統合テスト:
- [ ] `worktree-create`: 新規スロット 1 を作成 → ディレクトリ、`.worktree-slot`、`.env`（offset 1）、Docker、DB が正常
- [ ] `worktree-create`: 既存スロットへの再作成でエラー
- [ ] `worktree-create`: 不正な引数（0, 10, abc）でエラー
- [ ] `worktree-switch`: 新規ブランチの作成と切り替え
- [ ] `worktree-switch`: 未コミット変更がある場合の拒否
- [ ] `worktree-switch`: DB マイグレーションの自動実行

---

## Phase 2: worktree-issue.sh 改修 + worktree-add 廃止

### worktree-issue.sh

引数変更: `NUMBER` → `NUMBER [SLOT]`

1. Issue タイトルからブランチ名を自動生成（L28-51 流用）
2. SLOT 解決:
   - 指定あり → そのスロットを使用
   - 指定なし、現在のディレクトリがスロット内（`.worktree-slot` 存在） → 自動検出
   - 指定なし、メイン worktree → エラー「スロット番号を指定してください。利用可能なスロット: ...」
3. `worktree-switch.sh "$SLOT" "$BRANCH"` に委譲

設計判断:
- **空きスロットの自動選択は実装しない**: ユーザーがどのスロットを使うかは意識的に選ぶべき（KISS）

### worktree-add の廃止

- justfile から `worktree-add` コマンドを削除
- `scripts/worktree-add.sh` を削除

### 確認事項
- [ ] パターン: `worktree-issue.sh` の既存ブランチ名生成ロジック → L28-51
- [ ] パターン: `.worktree-slot` からのスロット番号読み取り方法

### テストリスト

手動統合テスト:
- [ ] `just worktree-issue 625 1` -- スロット指定で動作
- [ ] スロット内で `just worktree-issue 625` -- 自動検出で動作
- [ ] メイン worktree で引数なし実行 -- エラーメッセージと利用可能スロット表示

---

## Phase 3: cleanup.sh 改修

### 変更

永続スロットの worktree は削除ではなく detached HEAD にリセットする。

判定: `.worktree-slot` マーカーの存在

`cleanup.sh` L118-134 の削除ブロックを分岐:

```bash
if [[ -f "$path/.worktree-slot" ]]; then
    # 永続スロット: リセット
    echo "      永続スロットをリセット中..."
    git -C "$path" switch --detach origin/main
    if [[ "$branch" != "main" ]]; then
        git branch -D "$branch" 2>/dev/null || true
    fi
    echo "      ✓ スロットをリセットしました（detached HEAD）"
else
    # 従来の worktree: Docker ごと削除（既存ロジック）
    ...
fi
```

未コミット変更チェック（L101-116）はそのまま維持。

### 確認事項
- [ ] パターン: `cleanup.sh` の worktree 削除ロジック → L96-136
- [ ] ライブラリ: `git -C <path> switch --detach origin/main` が worktree 内で動作するか

### テストリスト

手動統合テスト:
- [ ] 永続スロットのマージ済みブランチ → detached HEAD にリセット、worktree は残る
- [ ] 永続スロットの未コミット変更あり → スキップ
- [ ] 非永続 worktree → 従来通り削除

---

## Phase 4: スキル更新 + ドキュメント + .gitignore

### `/next` スキル

変更箇所:
- L107, L127: `just worktree-issue <Issue番号>` → `just worktree-issue <Issue番号> <スロット番号>`
- L134-136 (Step 7): スロット一覧表示 + 空きスロットの案内

### `.gitignore`

`.worktree-slot` を追加。

### 手順書（全面改訂）

`docs/60_手順書/04_開発フロー/04_並行開発（Worktree）.md`:
- 概要: 永続スロット方式の説明
- 手順: `worktree-create` / `worktree-switch` / `worktree-issue`
- ディレクトリ図: `ringiflow-1`, `ringiflow-2`
- DB マイグレーションの扱い

### ADR-021 補遺

変更履歴に「永続スロット方式への移行」を追記。補遺セクション追加。

### 実装解説 改訂

`docs/90_実装解説/03_並行開発環境/`:
- 機能解説: アーキテクチャ図、データフロー、設計判断を更新
- コード解説: 新スクリプトのコードフロー追加

### README

L223 の worktree 機能の紹介を永続スロット方式に更新。

### 確認事項
- [ ] パターン: `/next` スキルの Step 7 → `.claude/skills/next/SKILL.md` L130-137
- [ ] パターン: ADR の変更履歴フォーマット → `docs/70_ADR/021_並行開発環境の構成.md` L93-97
- [ ] パターン: 実装解説のセクション構成

### テストリスト

手動検証:
- [ ] ドキュメント内のコマンド例が実際のコマンドと一致する
- [ ] ADR の変更履歴が更新されている

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | `.worktree-slot` は `.gitignore` に追加が必要 | 未定義 | Phase 4 に `.gitignore` 更新を追加 |
| 2回目 | `env -u` に `API_TEST_*` 変数が欠けている（`worktree-add.sh` の既知問題） | 既存手段の見落とし | Phase 1 で修正 |
| 3回目 | detached HEAD からの初回 switch 時に reflog がない | エッジケース | `PREV_HEAD` を事前記録し、diff 失敗時は pnpm install を実行（fail-open） |
| 4回目 | メイン worktree で SLOT 未指定時のエラーメッセージ | 不完全なパス | Phase 2 に利用可能スロットの一覧表示を追加 |
| 5回目 | `pnpm install` が root / frontend / tests/e2e の3箇所 | 既存手段の見落とし | Phase 1 に3箇所を明記 |
| 6回目 | 他の worktree で使用中のブランチへの switch | エッジケース | git のエラーをそのまま伝播（適切なメッセージ） |
| 7回目 | `Cargo.lock` 差分の扱い | シンプルさ | cargo build が暗黙に解決 + sccache。明示的処理は不要 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue #625 の完了基準がすべて計画に含まれている | OK | 7項目すべて Phase 1-4 に対応 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各処理フローが具体的なコマンドとコードスニペットで記述 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | offset マッピング、--detach、マーカー方式、未コミット拒否、pnpm チェック方式 |
| 4 | スコープ境界 | 対象/対象外が明記 | OK | generate-env.sh, setup-env.sh, restore, settings.json を明示的に除外 |
| 5 | 技術的前提 | コードに現れない前提が考慮 | OK | --detach 動作、git switch の detached HEAD 対応、.gitignore 影響、reflog 制約 |
| 6 | 既存ドキュメント整合 | 既存ドキュメントと矛盾なし | OK | ADR-021 は補遺方式で既存決定と矛盾なし |

## 検証手順（E2E）

1. `just worktree-create 1` でスロット 1 を作成
2. `just worktree-switch 1 feature/test-branch` でブランチ切り替え
3. `cd ../ringiflow-1 && git branch --show-current` で `feature/test-branch` を確認
4. `just worktree-issue 625 1` で Issue ベースの切り替えを確認
5. `just cleanup --dry-run` でスロットが削除対象にならないことを確認
6. `just worktree-remove 1` でクリーンアップ（テスト後）
