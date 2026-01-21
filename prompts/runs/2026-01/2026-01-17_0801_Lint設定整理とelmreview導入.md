# 2026-01-17_03: lint 設定整理と elm-review 導入

## 概要

lint 設定を整理し、Elm に elm-review を導入した。format-check と lint を統合してタスク構成を簡潔にした。

## 背景と目的

- justfile の `lint` タスクに `fmt-check` が含まれておらず、`lint` 単独実行時にフォーマットチェックが漏れる問題があった
- Elm の `lint-elm` が `elm-format --validate` のみで、本来のリントツール（elm-review）が導入されていなかった
- format-check と lint を分離する意味が薄く、統合してシンプルにしたかった

## 実施内容

### 1. elm-review の導入

- `elm-review` パッケージをインストール
- `frontend/review/` ディレクトリに設定を作成
- `NoUnused.*` ルール群と `Simplify` ルールを有効化
- 既存コードの lint エラーを修正（未使用の依存関係・インポート）
- `Ports.elm` は将来使用予定のため、TODO コメント付きで除外設定

### 2. タスク構成の統合

**justfile:**
- `fmt-check` / `fmt-check-rust` / `fmt-check-elm` を廃止
- `lint-rust` = rustfmt + clippy
- `lint-elm` = elm-format + elm-review

**package.json:**
- `format:check` を廃止し `lint` に統合

### 3. CI の簡略化

- Format check と Lint ステップを統合（Lint のみに）
- Rust の Build ステップを削除し、`cargo test --release` で代替

### 4. pre-commit-check.sh の更新

- 廃止した `fmt-check-*` タスクへの参照を削除

## 成果物

### 変更ファイル

| ファイル | 変更内容 |
|---------|---------|
| `justfile` | fmt-check 廃止、lint に統合 |
| `frontend/package.json` | format:check 廃止、lint に統合 |
| `frontend/review/` | elm-review 設定を新規作成 |
| `frontend/elm.json` | 未使用の elm/http を削除 |
| `frontend/src/Main.elm` | 未使用の import・関数を削除 |
| `frontend/src/Route.elm` | 未使用の import を削除 |
| `.github/workflows/ci.yml` | Format check ステップ削除、test --release 化 |
| `.claude/hooks/pre-commit-check.sh` | 新タスク構成に更新 |

## 設計判断と実装解説

### elm-review の採用理由

elm-review は Elm コミュニティの事実上の標準リントツール。Rust における clippy に相当する。作者の Jeroen Engels は Elm Radio のホストで、コミュニティで広く信頼されている。

### format-check と lint の統合

一般的に「lint」は静的解析全般を指し、フォーマットチェックもその一部と見なせる。分離することで `lint` 単独実行時に漏れが生じるリスクがあったため、統合してシンプルにした。

### CI での test --release

clippy と test で既にコンパイルが行われるため、別途 Build ステップを持つ意味が薄い。`cargo test --release` にすることで release ビルドの検証も兼ねる。

## 議論の経緯

### lint タスクの構成

ユーザーから、lint に format-check が抜けているのではないかという指摘があった。また、Elm の lint が format-check だけになっていて、本来のリントツールが導入されていないという問題も指摘された。

### タスク統合の検討

format-check と lint を分ける意味が薄いのではないかという議論があり、統合してシンプルにする方針に決定した。

### コミット時の品質保証

ユーザーから、コミット時に lint と test が通ることを保証したいという要望があった。WIP コミットもあるため、Claude Code に保証させる形で一旦対応することになった。

## 学んだこと

- elm-review は Elm の標準的なリントツールであり、NoUnused ルール群でコードの整理に役立つ
- タスク構成はシンプルに保ち、実行漏れのリスクを減らすべき
- Claude Code の PreToolUse フックで git commit 前のチェックを自動化できる
