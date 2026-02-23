# #835 improvement-records.sh を rust-script に移行

## コンテキスト

`scripts/check/improvement-records.sh`（133行）は改善記録の標準フォーマット準拠をバリデーションするスクリプト。Bash 配列で有効値リストを管理し、sed で値抽出を行っている。ADR-015 の移行基準（複雑なロジック、ユニットテスト不可）に該当するため、rust-script に移行する。

先行事例: `scripts/check/instrumentation.rs`（#834）で rust-script パターンが確立済み。

## 対象

- `scripts/check/improvement-records.rs` — 新規作成
- `justfile` — `lint-improvements` タスクを更新
- `scripts/check/improvement-records.sh` — 削除

対象外:
- `scripts/check/parallel.sh` — `just lint-improvements` を呼んでいるだけなので変更不要
- バリデーションロジックの拡張（既存と同等の機能のみ）

## 設計判断

### enum の設計

有効値を Rust の enum として定義し、`FromStr` を実装する。enum バリアントは英語、日本語文字列との対応は `FromStr` / `Display` で管理する。

```rust
#[derive(Debug, PartialEq)]
enum Category {
    ReferenceOmission,       // 参照漏れ
    SinglePathVerification,  // 単一パス検証
    ImmediateAction,         // 即座の対策
    LackOfPerspective,       // 視点不足
    ContextCarryover,        // コンテキスト引きずり
    KnowledgeExecutionGap,   // 知識-実行乖離
}

#[derive(Debug, PartialEq)]
enum FailureType {
    KnowledgeGap,  // 知識ギャップ
    ExecutionGap,  // 実行ギャップ
    ProcessGap,    // プロセスギャップ
}

#[derive(Debug, PartialEq)]
enum Nature {
    Technical,  // 技術的
    Process,    // プロセス的
    Cognitive,  // 思考的
}
```

各 enum に:
- `FromStr` — バリデーション（不正値でエラーメッセージ生成）
- `all_values() -> &[&str]` — エラーメッセージ用の有効値一覧

### 値抽出: regex 不要、文字列操作で十分

Issue 本文では `regex` クレートの使用を提案しているが、実際の値抽出は:
1. プレフィックス除去（`- カテゴリ: `）
2. 括弧以降の除去（`（` or `(`）
3. 末尾空白除去

これは `str::strip_prefix` + `str::find` + `str::trim` で明快に表現できる。regex はオーバーキル（KISS）。

```rust
fn extract_value(line: &str, prefix: &str) -> String {
    let value = line.strip_prefix(prefix).unwrap_or(line);
    let value = match value.find(&['（', '('][..]) {
        Some(pos) => &value[..pos],
        None => value,
    };
    value.trim().to_string()
}
```

### ファイル探索: glob クレート

Shell の `process/improvements/????-??/*.md` パターンを `glob` クレートで直接再現する。

### 依存クレート

```toml
[dependencies]
glob = "0.3"
pretty_assertions = "1"
```

## Phase 1: rust-script の実装・justfile 更新・旧スクリプト削除

### 確認事項

- パターン: `instrumentation.rs` の構造（shebang、cargo ブロック、run() → main()、#[cfg(test)]）→ `scripts/check/instrumentation.rs`
- パターン: justfile の lint タスクの呼び出し形式 → `justfile` L408-409
- ライブラリ: `glob` クレートの API → Grep 既存使用 or docs.rs

### 操作パス

| # | 操作パス | 分類 | テスト層 |
|---|---------|------|---------|
| 1 | `just lint-improvements` を実行し、全改善記録のバリデーション結果を確認する | 正常系 | 手動検証 |
| 2 | バリデーションエラーのある改善記録が検出される | 準正常系 | ユニットテスト |

### テストリスト

ユニットテスト:
- [ ] `extract_value` — プレフィックスと値を正しく分離する
- [ ] `extract_value` — 全角括弧以降を除去する
- [ ] `extract_value` — 半角括弧以降を除去する
- [ ] `extract_value` — 括弧がない場合は値全体を返す
- [ ] `extract_value` — 末尾の空白を除去する
- [ ] `Category::from_str` — 有効なカテゴリ値を受け入れる
- [ ] `Category::from_str` — 無効な値でエラーを返す
- [ ] `FailureType::from_str` — 有効な失敗タイプ値を受け入れる
- [ ] `FailureType::from_str` — 無効な値でエラーを返す
- [ ] `Nature::from_str` — 有効な問題の性質値を受け入れる
- [ ] `Nature::from_str` — 無効な値でエラーを返す
- [ ] `validate_file` — 正常なファイルでエラーなし
- [ ] `validate_file` — `## 分類` セクションがない場合にエラー
- [ ] `validate_file` — カテゴリ行がない場合にエラー
- [ ] `validate_file` — 無効なカテゴリ値でエラー
- [ ] `validate_file` — 失敗タイプ行がない場合にエラー
- [ ] `validate_file` — 無効な失敗タイプ値でエラー
- [ ] `validate_file` — 問題の性質が未記載の場合に警告（エラーではない）
- [ ] `validate_file` — 無効な問題の性質値でエラー

ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `just lint-improvements` が単体で動作し、既存シェルスクリプトと同じ検出結果を出力する
- [ ] `just check-all` が正常に通過する

### 作業

1. `scripts/check/improvement-records.rs` を新規作成
   - shebang + cargo ブロック（glob, pretty_assertions）
   - enum 定義（Category, FailureType, Nature）+ FromStr 実装
   - `extract_value` 関数
   - `validate_file` 関数（ファイル内容を受け取り、エラー・警告を返す）
   - `run()` 関数（glob でファイル探索 → validate_file → 結果出力）
   - `main()` → `std::process::exit(run())`
   - `#[cfg(test)] mod tests` — テストリストの全項目
2. `justfile` の `lint-improvements` タスクを更新（`./scripts/check/improvement-records.sh` → `rust-script ./scripts/check/improvement-records.rs`）
3. 手動検証: `just lint-improvements` で既存スクリプトと同じ結果を確認
4. `scripts/check/improvement-records.sh` を削除

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | Issue は regex クレート使用を提案しているが、実際の値抽出は文字列操作で十分 | シンプルさ | regex を不採用とし、理由を設計判断に記載 |
| 2回目 | parallel.sh の変更要否が不明確 | スコープ境界 | parallel.sh は `just lint-improvements` を呼んでおり変更不要、対象外に明記 |

## 収束確認（設計・計画）

| # | 観点 | 判定 | 確認内容 |
|---|------|------|---------|
| 1 | 網羅性 | OK | 完了基準 6 項目（rs 作成、enum 定義、同等出力、テスト、justfile 更新、sh 削除）すべてを Phase 1 の作業でカバー |
| 2 | 曖昧さ排除 | OK | enum の設計、値抽出の方式、ファイル探索方式を具体的に記載 |
| 3 | 設計判断の完結性 | OK | regex 不採用の判断を理由付きで記載 |
| 4 | スコープ境界 | OK | 対象（3 ファイル）と対象外（parallel.sh、ロジック拡張）を明記 |
| 5 | 技術的前提 | OK | glob クレートの API、rust-script の構造は先行事例で確認済み |
| 6 | 既存ドキュメント整合 | OK | ADR-015（移行基準）、ADR-056（rust-script 許容）と整合 |
