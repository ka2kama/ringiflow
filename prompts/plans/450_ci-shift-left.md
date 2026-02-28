# #450 CI で初めて検出される問題のシフトレフト

## Context

品質チェックの実行が手動（`just check` / `just check-all`）に依存しており、Git フック（構造）に組み込まれていない。CI の Claude Auto Review で検出されていた「静的に検出可能な問題」（カテゴリ1）をローカルにシフトレフトする。

## 対象・対象外

対象:
- Git フック（pre-commit, pre-push）の追加
- 静的チェックスクリプトの `just check` 組み込み（check-rule-files.sh、リンク切れチェック、実装解説命名チェック）

対象外:
- カテゴリ2（ローカル Claude の自己規律）— 別途検討
- カテゴリ3（レビュアー視点）— CI AI レビューの役割

## 設計判断

### 1. pre-commit: フォーマットチェックのみ

コミット毎に実行されるため、高速（数秒以内）であることが必須。lefthook の `glob` + `{staged_files}` で staged files のみを対象に rustfmt / elm-format を実行する。

### 2. pre-push: lint + unit test（DB 不要のサブセット）

CI 実測データ（Rust Integration: 235s、Rust Test: 153s、Rust Lint: 136s）から、`just check` 全体（3〜5分）をフックにすると `--no-verify` で迂回され形骸化するリスクが高い。

DB 不要のサブセット（lint + unit test、~1-2 分）に限定する。具体的には `check-parallel.sh` に `--skip-db` オプションを追加し、以下を除外:
- `test-rust-integration`（DB 必要）
- `sqlx-check`（DB 必要）
- `schema-check`（DB 必要）

代替案: `just check` 全体 → 形骸化リスクで不採用。pre-push なし → シフトレフト不十分で不採用。

### 3. リンク切れチェック: 内部 Markdown リンクのパス存在のみ

`[text](path)` 形式の相対パスリンクを対象。HTTP(S) リンクはネットワーク依存で遅いため対象外。アンカー（`#section`）の存在確認は複雑なため MVP では対象外。

### 4. 実装解説の命名チェック

`docs/90_実装解説/README.md` の命名規則に準拠しているかを検証する:
- ディレクトリ名: `NN_<機能名>/`
- ファイル名: `NN_<トピック>_機能解説.md` / `NN_<トピック>_コード解説.md`
- 機能解説とコード解説がペアで存在すること

## チェック階層

```
pre-commit（毎コミット、数秒）
├── rustfmt --check（staged .rs files）
└── elm-format --validate（staged .elm files）

pre-push（毎プッシュ、~1-2 分）= just check-pre-push
└── check-parallel.sh --skip-db
    ├── Rust レーン: lint-rust → test-rust → openapi-check
    └── Non-Rust レーン: lint-elm, test-elm, build-elm, lint-shell,
                          lint-ci, lint-openapi, lint-improvements,
                          lint-rules, check-doc-links, check-impl-docs,
                          check-unused-deps, check-file-size, check-duplicates

just check（手動、~3-5 分）
└── check-parallel.sh（引数なし）
    ├── Rust レーン: lint-rust → test-rust → test-rust-integration
    │               → sqlx-check → schema-check → openapi-check
    └── Non-Rust レーン:（同上）

just check-all（プッシュ前に手動、~5-10 分）
└── check + audit + test-api + test-e2e
```

## 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `lefthook.yaml` | pre-commit にフォーマットチェック追加、pre-push 追加 |
| `scripts/check-parallel.sh` | `--skip-db` オプション追加、新チェック 3 つ追加 |
| `justfile` | `check-pre-push`, `lint-rules`, `check-doc-links`, `check-impl-docs` レシピ追加 |
| `scripts/check-doc-links.sh` | 新規: ドキュメント内リンク切れチェック |
| `scripts/check-impl-docs.sh` | 新規: 実装解説の命名規則チェック |

## 実装計画

### Phase 1: pre-commit フォーマットチェック

lefthook.yaml に rustfmt-check と elm-format-check を追加する。

```yaml
pre-commit:
  parallel: true
  commands:
    no-yml-extension:
      # 既存（変更なし）
    rustfmt-check:
      glob: "*.rs"
      run: rustfmt +nightly --edition 2024 --check {staged_files}
    elm-format-check:
      glob: "*.elm"
      run: elm-format --validate {staged_files}
```

#### 確認事項
- [x] lefthook の `glob` + `{staged_files}` 構文 → 公式ドキュメントで確認済み
- [x] rustfmt のフラグ → `rustfmt +nightly --edition 2024 --check`（justfile の `fmt-rust` パターン）
- [x] elm-format のフラグ → `elm-format --validate`（justfile の `lint-elm` パターン）

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `lefthook run pre-commit` でフォーマット違反が検出される
- [ ] 対象ファイルがない場合にスキップされる

### Phase 2: 静的チェックスクリプト 3 つの追加

#### Phase 2a: check-rule-files.sh の just check 組み込み

justfile に `lint-rules` レシピを追加し、`check-parallel.sh` の Non-Rust レーンに組み込む。

#### Phase 2b: ドキュメント内リンク切れチェック

`scripts/check-doc-links.sh` を作成:
- `docs/` と `prompts/` 配下の `.md` ファイルを走査
- `[text](path)` 形式の相対パスリンクを抽出
- リンク先のファイル/ディレクトリが存在するか検証
- HTTP(S) リンクはスキップ
- `.claude/` 配下の Markdown も対象（CLAUDE.md から多数のルールファイルを参照）

#### Phase 2c: 実装解説のファイル命名規則チェック

`scripts/check-impl-docs.sh` を作成:
- `docs/90_実装解説/` のサブディレクトリを走査（README.md は除外）
- ディレクトリ名が `NN_<機能名>/` パターンに合致するか
- 各ディレクトリ内のファイル名が `NN_<トピック>_{機能解説,コード解説}.md` パターンに合致するか
- 機能解説とコード解説がペアで存在するか

#### 確認事項
- [x] check-rule-files.sh の実行方法 → `./scripts/check-rule-files.sh`（引数なし）
- [ ] docs/ 配下の Markdown リンク形式 → Grep で確認
- [x] 実装解説の命名規則 → `NN_<トピック>_機能解説.md` / `NN_<トピック>_コード解説.md`（README.md で確認済み）
- [ ] 現在の実装解説ディレクトリ構造 → ls で確認

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `just lint-rules` が正常に実行される
- [ ] `just check-doc-links` が正常に完了する（既存リンクがすべて有効）
- [ ] `just check-doc-links` が意図的に壊したリンクを検出する
- [ ] `just check-impl-docs` が正常に完了する（既存ファイルが命名規則に準拠）
- [ ] `just check-impl-docs` がペア欠如を検出する

### Phase 3: pre-push フックと check-parallel.sh の --skip-db 対応

1. `check-parallel.sh` に `--skip-db` オプションを追加。`--skip-db` 時は Rust レーンから `test-rust-integration`, `sqlx-check`, `schema-check` を除外する
2. justfile に `check-pre-push` レシピを追加: `./scripts/check-parallel.sh --skip-db`
3. lefthook.yaml に pre-push フックを追加: `run: just check-pre-push`

#### 確認事項
- [x] check-parallel.sh の構造 → Rust/Non-Rust の 2 レーン並列実行（確認済み）
- [x] lefthook の pre-push 構文 → `run: just check-pre-push`

#### テストリスト

ユニットテスト（該当なし）
ハンドラテスト（該当なし）
API テスト（該当なし）
E2E テスト（該当なし）

手動検証:
- [ ] `just check-pre-push` が DB なしで正常に完了する
- [ ] `just check` が引き続き全チェックを実行する（デグレなし）
- [ ] `lefthook run pre-push` が `just check-pre-push` を実行する

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | pre-push で `just check` 全体を使う場合の実行時間（3〜5分）と形骸化リスク | 不完全なパス | CI 実測データで判断。DB 不要のサブセット（~1-2 分）に限定。`check-parallel.sh --skip-db` で実現 |
| 2回目 | リンク切れチェックで HTTP リンクとアンカーの扱いが未定義 | 曖昧 | 内部リンクのパス存在のみに限定と明記 |
| 3回目 | チェック階層（pre-commit / pre-push / check / check-all）の全体像が不明確 | 未定義 | チェック階層セクションを追加し、各層の内容を明示 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Issue の全対策が計画に含まれている | OK | Git フック 2 種 + スクリプト 3 つが全て Phase に対応。チェック階層で全体像を提示 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | リンク切れチェック範囲、pre-push 範囲、チェック階層を明確化 |
| 3 | 設計判断の完結性 | 全ての選択肢に判断理由がある | OK | pre-push 範囲（サブセット vs 全体 vs なし）を CI 実測データで判断 |
| 4 | スコープ境界 | 対象と対象外が両方明記 | OK | カテゴリ 2, 3 を対象外として明記 |
| 5 | 技術的前提 | lefthook の構文が確認済み | OK | glob + staged_files、pre-push 構文を公式ドキュメントで確認 |
| 6 | 既存ドキュメント整合 | ADR-014 等と矛盾がない | OK | ADR-014（lefthook 導入）の方針を踏襲、拡張のみ |
