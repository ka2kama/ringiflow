# impl-docs.sh を rust-script に移行

## 概要

`scripts/check/impl-docs.sh`（83行）を `scripts/check/impl-docs.rs`（280行、テスト含む）に移行した。ADR-015 の移行基準に該当する Bash 4+ 依存機能（`declare -A` 連想配列、`BASH_REMATCH`）を Rust の型安全な構造に置き換えた。

## 実施内容

### Phase 1: rust-script 実装

- `regex` クレートを採用（ファイル名パターンマッチ + キャプチャグループによるトピック抽出）
- `glob` クレートでディレクトリ・ファイル探索（improvement-records.rs と同パターン）
- `validate_dir` を純粋関数として実装（`DirFiles` 構造体を受け取り、ファイルシステムアクセスなし）
- Bash の `declare -A topics_feature` / `declare -A topics_code`（2 つの連想配列）を `HashMap<String, HashSet<DocType>>` に集約
- 8 つのユニットテストを同梱

### Phase 2: justfile 更新と旧スクリプト削除

- `justfile` の `check-impl-docs` タスクを `rust-script` 呼び出しに更新
- `scripts/check/impl-docs.sh` を削除
- `docs/90_実装解説/` 内のリンク切れ（`.sh` → `.rs`）を修正

## 判断ログ

- 特筆すべき判断なし（先行移行で確立済みのパターンに従った定型的な移行）

## 成果物

コミット:
- `0f53f15` — `#836 Migrate impl-docs.sh to rust-script`

作成:
- `scripts/check/impl-docs.rs`

更新:
- `justfile`
- `docs/90_実装解説/PR697_Observability構造的担保/01_Observability構造的担保_コード解説.md`

削除:
- `scripts/check/impl-docs.sh`
