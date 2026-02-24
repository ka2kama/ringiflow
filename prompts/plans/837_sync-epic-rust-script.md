# 計画: sync-epic.sh を rust-script に移行

## Context

Issue #837（Epic #841）。`scripts/issue/sync-epic.sh`（79行）を `scripts/issue/sync-epic.rs` に移行する。perl/grep -P 依存を排除し、ロジックをテスト可能にする。

同 Epic で `instrumentation.sh`（#834）、`improvement-records.sh`（#835）、`impl-docs.sh`（#836）が既に移行済み。確立されたパターンに従う。

## 設計判断

### 1. I/O 境界の分離

純粋関数として以下の4つを抽出し、`run()` は I/O オーケストレーションのみ担当。

| 純粋関数 | 入力 | 出力 | 責務 |
|---------|------|------|------|
| `extract_epic_number` | `&str` (issue body) | `Option<u32>` | Epic 番号抽出 |
| `check_already_updated` | `&str` (epic body), `u32` (issue number) | `bool` | 冪等性チェック |
| `check_exists_unchecked` | `&str` (epic body), `u32` (issue number) | `bool` | 未チェック行の存在確認 |
| `update_checkbox` | `&str` (epic body), `u32` (issue number) | `String` | チェックボックス更新 |

代替案: trait による `GhClient` 抽象化 → 過度。4つの純粋関数で十分テスタブル。

### 2. 正規表現の設計

**Epic 番号抽出**（perl `-0777` の置き換え）:

```rust
// フォーマット1: インライン（手動記載）— Epic: #123
let pattern1 = Regex::new(r"Epic:\s*#(\d+)").unwrap();
// フォーマット2: テンプレートレンダリング（feature.yaml の type: input）— ### Epic\n\n#123
// Rust regex の \s は \n を含むため、\s+ で改行を含むマッチが可能
let pattern2 = Regex::new(r"###\s+Epic\s+#(\d+)").unwrap();
```

**部分一致防止**（grep -P の置き換え）:

```rust
// (?m) で ^ が各行頭にマッチ
// (?:\D|$) で #NNN の後に数字が続かないことを保証
let pattern = Regex::new(&format!(r"(?m)^- \[x\] .*#{}(?:\D|$)", issue_number)).unwrap();
```

### 3. チェックボックス更新（sed の置き換え）

行単位で処理し、マッチした行のみ `[ ]` → `[x]` に置換:

```rust
fn update_checkbox(epic_body: &str, issue_number: u32) -> String {
    let pattern = Regex::new(&format!(r"(?m)^- \[ \] .*#{}(?:\D|$)", issue_number)).unwrap();
    epic_body
        .lines()
        .map(|line| {
            if pattern.is_match(line) {
                line.replacen("[ ]", "[x]", 1)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

### 4. 終了コードとメッセージの互換性

元スクリプトと完全に同じ挙動を維持:

| 条件 | 終了コード | メッセージ | 出力先 |
|------|----------|----------|--------|
| 引数不足 | 1 | `使い方: sync-epic.rs ISSUE_NUMBER` | stderr |
| Issue 取得失敗/body 空 | 1 | `エラー: Issue #N が見つからないか、body が空です` | stderr |
| Epic 未設定 | 0 | `ℹ️ Issue #N に親 Epic が設定されていません（スキップ）` | stdout |
| Epic 取得失敗/body 空 | 1 | `エラー: Epic #N が見つからないか、body が空です` | stderr |
| 既に更新済み | 0 | `✓ Epic #N のタスクリストは既に更新済みです` | stdout |
| 未チェック行なし | 0 | `⚠️ Epic #N のタスクリストに #M が見つかりません` | stderr |
| 更新成功 | 0 | `✓ Epic #N のタスクリストを更新しました（#M → [x]）` | stdout |

## Phase 1: sync-epic.rs の実装・justfile 更新・旧スクリプト削除

### 確認事項
- パターン: 既存 rust-script 構造（`run() -> i32` + テスト） → `scripts/check/impl-docs.rs`
- パターン: `std::process::Command` での外部コマンド実行 → `scripts/check/instrumentation.rs` L149-165
- ライブラリ: `regex` の `Regex::new`, `.is_match()`, `.captures()` → `scripts/check/impl-docs.rs` L21, L48-50
- パターン: justfile の rust-script 呼び出し形式 → `justfile` L409, L423, L431

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | Story Issue 番号を指定して、Epic タスクリストが正常に更新される | 正常系 | 手動検証 |
| 2 | Epic 未設定の Story Issue を指定して、スキップされる | 準正常系 | 手動検証 |
| 3 | 既に更新済みの Story を指定して、冪等にスキップされる | 準正常系 | 手動検証 |
| 4 | Epic タスクリストに存在しない Issue 番号を指定して、警告が出る | 準正常系 | 手動検証 |

注: `gh` CLI 呼び出しを含む統合的な動作は手動検証。純粋ロジックはユニットテストでカバー。

### テストリスト

ユニットテスト:

**extract_epic_number**:
- [ ] インライン形式 `Epic: #123` から Epic 番号を抽出する
- [ ] スペースなし `Epic:#123` でも抽出できる
- [ ] 複数スペース `Epic:  #123` でも抽出できる
- [ ] テンプレート形式 `### Epic\n\n#123` から Epic 番号を抽出する
- [ ] Epic が設定されていない body で None を返す
- [ ] 本文中の `#123` だけでは Epic 番号として抽出しない

**check_already_updated**:
- [ ] `- [x] ... #123` がある場合に true を返す
- [ ] `- [ ] ... #123` しかない場合に false を返す
- [ ] `#123` が含まれない場合に false を返す
- [ ] 部分一致防止: `#12` で `#123` にマッチしない
- [ ] 部分一致防止: `#123` で `#1234` にマッチしない
- [ ] 行末の `#123`（後続文字なし）にマッチする

**check_exists_unchecked**:
- [ ] `- [ ] ... #123` がある場合に true を返す
- [ ] `- [x] ... #123` しかない場合に false を返す
- [ ] `#123` が含まれない場合に false を返す
- [ ] 部分一致防止: `#12` で `#123` にマッチしない

**update_checkbox**:
- [ ] `- [ ] ... #123` を `- [x] ... #123` に更新する
- [ ] 他の行はそのまま保持する
- [ ] 部分一致防止: `#12` の更新時に `#123` の行は変更しない
- [ ] 複数行がある場合、該当行のみ更新する

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

### 変更対象ファイル

| 操作 | ファイル |
|------|---------|
| 新規 | `scripts/issue/sync-epic.rs` |
| 編集 | `justfile`（L629-630: sync-epic レシピ） |
| 削除 | `scripts/issue/sync-epic.sh` |

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | テンプレート形式の正規表現で `(?m)` は不要（`^` 未使用）。`\s+` だけで改行を含むマッチが可能 | 技術的前提 | pattern2 を `r"###\s+Epic\s+#(\d+)"` にシンプル化。`(?m)` はチェックボックスパターンのみに使用 |
| 2回目 | 部分一致防止の行末ケースがテストリストに不足 | 不完全なパス | `check_already_updated` に行末ケースのテストを追加 |
| 3回目 | ドキュメント参照の更新要否 | 既存手段の見落とし | 手順書・スキルは `just sync-epic` を参照。スクリプトファイル名は直接参照されていないため更新不要 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | 全対象が計画に含まれている | OK | 4純粋関数 + 2 gh ヘルパー + run + justfile 変更 + 旧スクリプト削除。元スクリプトの全分岐をカバー |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 正規表現パターン、終了コード、メッセージ文言がすべて具体的 |
| 3 | 設計判断の完結性 | 全ての差異に判断が記載 | OK | I/O 分離、正規表現設計、部分一致防止、チェックボックス更新ロジック |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | 対象: rs 実装 + justfile + 旧スクリプト削除。対象外: ドキュメント参照更新（不要と確認） |
| 5 | 技術的前提 | 前提が考慮されている | OK | regex `\s` は `\n` を含む、`(?m)` は `^` 使用パターンのみに必要、`lines()` + `join("\n")` の改行処理 |
| 6 | 既存ドキュメント整合 | 矛盾がない | OK | Issue #837 の完了基準6項目と本計画を照合、すべて対応 |

## 検証方法

1. `rust-script ./scripts/issue/sync-epic.rs` — ユニットテスト実行（`--test` フラグ）
2. `just check-all` — 既存テスト・リントの通過確認
3. 手動検証: 既に [x] の Story（例: #836）で冪等性確認
4. 手動検証: Epic に属さない Issue でスキップ確認
