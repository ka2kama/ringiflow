# 計画: #845 改善記録のサニタイズを lefthook lint で自動検証する

## Context

改善記録におけるユーザー発言の直接引用（サニタイズ漏れ）が 3 回再発（02-09, 02-15, 02-19）。行動規範形式の対策（docs.md の自己チェック）では防止できないことが実証済み。lefthook pre-commit でのフロー組み込みにより、構造的にサニタイズ違反を防止する。

## 対象

- `scripts/check/sanitize-improvements.sh`（新規作成）
- `lefthook.yaml`（pre-commit に追加）
- `justfile`（lint レシピ追加）
- `scripts/check/parallel.sh`（Non-Rust レーンに追加）
- `process/improvements/README.md`（サニタイズルール追加）
- 既存の改善記録 ~15 ファイル（サニタイズ違反の修正）

## 対象外

- `improvement-records.rs` への統合（フォーマット検証とサニタイズ検証は別の関心事）
- `prompts/runs/` のサニタイズ検証（スコープ外）
- docs.md の自己チェックリスト更新（自動検証が補完するため現状維持）

## 設計判断

**Shell vs Rust**: Shell を採用。パターンが grep ベースで単純、pre-commit での高速起動が重要、`stale-annotations.sh` と同じパターン。フォーマット検証（Rust）とサニタイズ検証（Shell）は別の関心事。

**偽陽性対策**: recall（真陽性の見逃し防止）を優先。2 件のエッジケース（`ユーザーの「y」`、`ユーザーの発言に対して「何をすべきか」`）は修正時に言い換えて解消。

---

## Phase 1: lint スクリプトの実装

`scripts/check/sanitize-improvements.sh` を新規作成する。

### 検出パターン

| # | パターン（ERE） | 検出対象 |
|---|----------------|---------|
| 1 | `ユーザー[がのはから].*[「『][^」』]+[」』]` | ユーザー帰属 + カギ括弧引用 |
| 2 | `[「『][^」』]+[」』]と(言っ\|述べ\|指摘し\|発言し\|要求し\|依頼し\|質問し\|確認し\|聞い\|尋ね\|答え\|返し\|主張し)` | 引用 + 発話動詞 |
| 3 | `[「『][^」』]+[」』]という(指摘\|発言\|要求\|依頼\|質問\|意見\|フィードバック\|コメント)` | 引用 + 帰属名詞 |

### 動作モード

- 引数あり: 指定ファイルのみ（pre-commit の `{staged_files}` 用）
- 引数なし: `git ls-files` で `process/improvements/????-??/*.md` 全体

### コードブロック除外

`awk '/^```/{inside=!inside; next} !inside{print}'` でコードブロック外の行のみを対象にする。

### 確認事項

- パターン: `stale-annotations.sh` の構造（set -euo pipefail, errors 配列, exit code） → `scripts/check/stale-annotations.sh`
- パターン: lefthook 連携スクリプトの `{staged_files}` 受取り方法 → `lefthook.yaml` の rustfmt-check
- ライブラリ: `grep -E` の日本語 UTF-8 対応 → 既存スクリプトが日本語パターンで動作している実績あり

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 違反のないファイルを渡す → exit 0 | 正常系 | ユニット |
| 2 | Pattern 1-3 違反を含むファイル → exit 1 | 正常系 | ユニット |
| 3 | コードブロック内の違反パターン → exit 0（スキップ） | 準正常系 | ユニット |
| 4 | README.md を渡す → スキップ | 準正常系 | ユニット |
| 5 | 引数なしで全ファイルチェック → 全ファイル対象 | 正常系 | ユニット |

### テストリスト

ユニットテスト（テスト用一時ファイルで検証）:
- [ ] 違反なしのファイル → exit 0、成功メッセージ
- [ ] Pattern 1 違反（`ユーザーから「xxx」と指摘`） → exit 1、エラーにファイル名・行を含む
- [ ] Pattern 2 違反（`「xxx」と指摘し`） → exit 1
- [ ] Pattern 3 違反（`「xxx」という指摘`） → exit 1
- [ ] 『』二重カギ括弧 → exit 1
- [ ] コードブロック内の Pattern 1 → exit 0（スキップ）
- [ ] README.md → exit 0（スキップ）
- [ ] process/improvements/ 以外のファイル → exit 0（スキップ）

ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

---

## Phase 2: 既存違反の修正

Phase 1 のスクリプトで検出される既存違反（~15 ファイル）を修正する。直接引用を技術的な要約に言い換える。

### 修正方針

| 違反パターン | 修正方法 |
|-------------|---------|
| `ユーザーから「xxx」と指摘` | `ユーザーから xxx を指摘された` |
| `ユーザーの指摘: 「xxx」` | `ユーザーの指摘: xxx` |
| `ユーザーの「y」` | `ユーザーの承認操作` |

### 確認事項

- 修正前後で元の意味が保たれていることを各ファイルで確認
- 修正後に `./scripts/check/sanitize-improvements.sh` を実行 → exit 0

### 操作パス: 該当なし（ドキュメント修正のみ）

### テストリスト

ユニットテスト:
- [ ] `./scripts/check/sanitize-improvements.sh` 引数なし実行 → exit 0

ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

---

## Phase 3: lefthook + justfile + parallel.sh 統合

### 変更内容

**lefthook.yaml** pre-commit セクション:
```yaml
    sanitize-improvements:
      glob: "process/improvements/**/*.md"
      run: ./scripts/check/sanitize-improvements.sh {staged_files}
```

**justfile**（`lint-improvements` レシピの直後に追加）:
```just
# 改善記録のサニタイズ違反検出（ユーザー発言の直接引用）
lint-improvements-sanitize:
    ./scripts/check/sanitize-improvements.sh
```

**parallel.sh**（line 39 `just lint-improvements` の直後）:
```bash
    just lint-improvements-sanitize
```

### 確認事項

- パターン: lefthook の `glob` + `{staged_files}` → `lefthook.yaml` の rustfmt-check が同じ構造
- パターン: `parallel.sh` の Non-Rust レーンへの追加位置 → line 39 `just lint-improvements` の直後

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | 違反ファイルをステージしてコミット → pre-commit でブロック | 正常系 | 手動検証 |
| 2 | 正常ファイルをステージしてコミット → 通過 | 正常系 | 手動検証 |
| 3 | `just lint-improvements-sanitize` → 全ファイルチェック | 正常系 | 手動検証 |

### テストリスト

手動検証:
- [ ] `just lint-improvements-sanitize` → exit 0
- [ ] `just check` → 正常終了（parallel.sh 経由）

ユニットテスト: 該当なし
ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

---

## Phase 4: ドキュメント更新

`process/improvements/README.md` に「サニタイズルール」セクションを追加する。

### 追加内容

「## 記載内容」セクションの後に追加:

```markdown
## サニタイズルール

改善記録でユーザーの発言を記載する際は、直接引用（カギ括弧「」『』による引用）せず、技術的内容に要約する。

| 禁止される表現 | 修正例 |
|---------------|--------|
| `ユーザーから「設定確認した？」と指摘` | `ユーザーから設定確認の不足を指摘された` |
| `「xxx」と言った` | `xxx との指摘があった` |
| `「xxx」という意見` | `xxx という観点の指摘` |

`scripts/check/sanitize-improvements.sh` が lefthook pre-commit および `just check` で自動検証する。

改善の経緯: [改善記録のサニタイズ漏れ再発](2026-02/2026-02-19_0234_改善記録のサニタイズ漏れ再発.md)
```

### 確認事項: なし（既知のパターンのみ）
### 操作パス: 該当なし（ドキュメントのみ）

### テストリスト

ユニットテスト: 該当なし
ハンドラテスト: 該当なし
API テスト: 該当なし
E2E テスト: 該当なし

---

## 検証方法

1. `./scripts/check/sanitize-improvements.sh` を単体実行 → exit 0
2. `just lint-improvements-sanitize` → exit 0
3. `just check-all` → 全テスト通過

---

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Pattern 1 に 2 件の偽陽性エッジケースあり（`ユーザーの「y」`、概念引用） | 競合・エッジケース | Phase 2 で一緒に言い換えて解消。recall 優先の設計方針 |
| 2回目 | 既存違反修正前に parallel.sh 追加すると全 push 失敗 | 不完全なパス | Phase 2（違反修正）→ Phase 3（統合）の順序に |
| 3回目 | 『』（二重カギ括弧）の検出漏れ | 状態網羅漏れ | パターンの文字クラスを `[「『]`/`[」』]` に拡張 |
| 4回目 | コードブロック内の正規パターン一致リスク | 競合・エッジケース | awk でコードブロック除外を追加 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | スクリプト新規・lefthook・justfile・parallel.sh・README・既存違反修正 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 3 パターンが ERE で一意に定義、ファイル構造・統合ポイントが具体的 |
| 3 | 設計判断の完結性 | 全差異に判断が記載 | OK | Shell vs Rust、偽陽性対策、Phase 順序 |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | 対象/対象外セクションあり |
| 5 | 技術的前提 | 前提が考慮されている | OK | grep -E UTF-8、lefthook glob + staged_files、awk 標準機能 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | docs.md サニタイズルール、README.md 分類フォーマットと矛盾なし |
