# Volta から mise への移行（プロジェクト設定）

## Context

Volta（Node.js バージョンマネージャー）がメンテナンスされなくなったため、プロジェクトレベルの Node.js バージョン管理を mise に移行する。mise は既にプロジェクトの開発環境構築ドキュメントで参照されており、ADR-002 でも `.mise.toml` による Elm ツール管理が言及されている。個人環境の移行は完了済み。

## 対象

- `frontend/package.json` — `volta` セクション削除
- `tests/e2e/package.json` — `volta` セクション削除
- `.mise.toml`（新規作成）— node, elm, elm-format のバージョンピン留め
- `frontend/README.md` — volta セクションを mise セクションに置換
- `docs/04_手順書/01_開発参画/01_開発環境構築.md` — mise を推奨に変更、Volta を削除
- `docs/05_ADR/052_Node.jsバージョン管理のmiseへの移行.md`（新規作成）

## 対象外

- CI 設定（`.github/workflows/ci.yaml`）— 既に `actions/setup-node` を直接使用しており Volta 非依存
- pnpm のバージョンピン留め — 現状も未ピン。別途必要になれば対応
- ADR-002 の変更 — ADR は不変原則。新 ADR-052 から参照する

## 設計判断

### `.mise.toml` の内容

```toml
[tools]
node = "22"
elm = "0.19.1"
"npm:elm-format" = "0.8.8"
```

- `node = "22"`: メジャーバージョンのみ指定。CI の `node-version: '22'` と一致させる。mise が最新の 22.x LTS を自動解決する。Volta の `22.16.0` のような厳密なパッチピンは開発環境では不要
- `elm = "0.19.1"`: mise レジストリから GitHub releases のネイティブバイナリを取得
- `"npm:elm-format" = "0.8.8"`: mise の npm バックエンド経由。elm-format は mise レジストリにないため npm バックエンドを使用

elm-test は `frontend/package.json` の `devDependencies` に含まれているため `.mise.toml` には含めない。

### 開発環境構築ドキュメントの方針

mise を推奨ツールとし、Volta の選択肢を削除する。nvm は代替として残す。理由: Volta がメンテナンス停止しているため新規参画者に推奨できない。

## Phase 1: Issue 作成 + ブランチ作成

GitHub Issue を作成し、ブランチを切る。

確認事項: なし（設定変更のみ）

テストリスト:

ユニットテスト（該当なし）

ハンドラテスト（該当なし）

API テスト（該当なし）

E2E テスト（該当なし）

## Phase 2: プロジェクト設定の変更

1. `.mise.toml` を project root に作成
2. `frontend/package.json` から `volta` セクションを削除
3. `tests/e2e/package.json` から `volta` セクションを削除

確認事項: なし（既知のパターンのみ）

## Phase 3: ドキュメント更新

1. `frontend/README.md` の volta セクション（L175-187）を mise の説明に置換
2. `docs/04_手順書/01_開発参画/01_開発環境構築.md` の Node.js/pnpm インストール手順を更新
3. ADR-052 を作成

確認事項:
- [x] ADR テンプレート → `docs/05_ADR/template.md` 必須セクション: ステータス、コンテキスト、検討した選択肢、決定、帰結、変更履歴

## Phase 4: 検証

1. `just check` で lint + テストが通ることを確認
2. `mise ls` でプロジェクトレベルの設定が反映されることを確認

## ブラッシュアップループの記録

| ループ | 検出したギャップ | 観点 | 対応 |
|-------|----------------|------|------|
| 1回目 | ADR-002 が `.mise.toml` で elm 管理を言及しているが、elm-test の扱いが不明確 | 不完全なパス | elm-test は devDependencies にあるため `.mise.toml` に含めないことを設計判断に明記 |
| 2回目 | 開発環境構築ドキュメントで Volta を残すか削除するかが未決定 | 曖昧 | Volta はメンテナンス停止のため削除、nvm は代替として残す方針を明記 |

## 収束確認（設計・計画）

| # | 観点 | 理想状態（To-Be） | 判定 | 確認内容 |
|---|------|------------------|------|---------|
| 1 | 網羅性 | Volta 参照がすべて計画に含まれている | OK | Grep で `volta` を検索し 5 ファイルを特定。CI（volta 不使用）とセッションログ（変更不要）を除く全ファイルが対象 |
| 2 | 曖昧さ排除 | 不確定な記述がゼロ | OK | 各ファイルの変更内容が具体的に記載されている |
| 3 | 設計判断の完結性 | バージョン指定方法、ツール選択が決定済み | OK | node のメジャーピン、elm/elm-format のバージョン、elm-test の除外を判断済み |
| 4 | スコープ境界 | 対象と対象外が明記 | OK | CI 設定、pnpm ピン留め、ADR-002 変更を対象外として明記 |
| 5 | 技術的前提 | mise の `.mise.toml` 仕様が確認済み | OK | 個人環境で `mise use -g` を実行済み。`npm:` バックエンドの動作も確認済み |
| 6 | 既存ドキュメント整合 | ADR-002 と矛盾がない | OK | ADR-002 の「`.mise.toml` で管理」方針と整合 |
