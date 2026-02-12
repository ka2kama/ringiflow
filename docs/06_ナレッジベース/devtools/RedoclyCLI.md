# Redocly CLI

[Redocly CLI](https://redocly.com/docs/cli/) は OpenAPI 仕様書のリンター・バンドラー・プレビューツール。
`openapi.yaml` の構文エラーやベストプラクティス違反を検出する。

## なぜ Redocly CLI を使うのか

| 比較対象 | 違い |
|---------|------|
| **Swagger Editor** | GUI ツール。CI での自動検証には不向き |
| **Spectral** | 同等の機能を持つリンター。Redocly は設定がシンプルで、ドキュメント生成機能も充実 |
| **手動レビュー** | 見落としが発生する。ツールで機械的にチェックすべき |

## インストール

npm パッケージとして提供されている。

```bash
# グローバルインストール
npm install -g @redocly/cli

# pnpm exec で実行（ルート devDependencies にインストール済み）
pnpm exec redocly lint openapi.yaml
```

確認:

```bash
redocly --version
```

## 基本的な使い方

### lint（検証）

```bash
# 単一ファイル
redocly lint openapi.yaml

# 設定ファイルを指定
redocly lint --config redocly.yaml
```

### preview（プレビュー）

```bash
# ブラウザでドキュメントをプレビュー
redocly preview-docs openapi.yaml
```

### bundle（バンドル）

複数ファイルに分割した仕様書を1つにまとめる。

```bash
redocly bundle openapi.yaml -o bundled.yaml
```

## 設定ファイル（redocly.yaml）

プロジェクトルートまたは OpenAPI ファイルと同じディレクトリに配置する。

```yaml
# redocly.yaml
apis:
  main:
    root: openapi.yaml

rules:
  # ルールを無効化
  no-server-example.com: off

  # 警告に変更（デフォルトはエラー）
  operation-4xx-response: warn
  no-unused-components: warn
```

### 主要なルール

| ルール | 説明 | デフォルト |
|--------|------|-----------|
| `no-server-example.com` | localhost や example.com の URL を禁止 | error |
| `security-defined` | すべてのエンドポイントに security を定義 | error |
| `operation-4xx-response` | 4XX レスポンスの定義を必須化 | error |
| `no-unused-components` | 未使用のコンポーネントを警告 | warn |
| `no-ambiguous-paths` | 曖昧なパス（`/users/{id}` と `/users/me` の競合等）を禁止 | error |

全ルール一覧: [Redocly Rules](https://redocly.com/docs/cli/rules/)

## OpenAPI 3.1 の注意点

OpenAPI 3.1 は JSON Schema draft 2020-12 に準拠している。
3.0 との主な違い:

### nullable の廃止

```yaml
# OpenAPI 3.0（非推奨）
current_step_id:
  type: string
  nullable: true

# OpenAPI 3.1（推奨）
current_step_id:
  type:
    - string
    - 'null'
```

**なぜ `'null'` をクォートするのか？**

YAML では `null` はリテラル値（空値）として解釈される。
クォートしないと意図した通りに動作しない可能性がある。

```yaml
# クォートなし → YAML の null リテラル
type:
  - string
  - null      # パーサーによっては無視される

# クォート付き → 文字列 "null"
type:
  - string
  - 'null'    # JSON Schema の type として正しく解釈
```

JSON Schema では `type` に `"null"` という**文字列**を指定して nullable を表現するため、
YAML で書く際はクォートが必要。

### security: [] の明示

認証不要なエンドポイントには、明示的に空の security を指定する。

```yaml
paths:
  /health:
    get:
      summary: ヘルスチェック
      security: []  # 認証不要
      responses:
        '200':
          description: OK
```

これがないと `security-defined` ルールでエラーになる。

## プロジェクトでの使い方

### ディレクトリ構成

```
openapi/
├── openapi.yaml     # OpenAPI 仕様書
└── redocly.yaml     # リンター設定
```

### 実行方法

```bash
# just コマンド経由
just lint-openapi

# 直接実行
pnpm exec redocly lint --config openapi/redocly.yaml
```

### CI での実行

GitHub Actions で `openapi/**` の変更時のみ実行:

```yaml
# .github/workflows/ci.yaml
openapi:
  name: OpenAPI
  runs-on: ubuntu-latest
  needs: changes
  if: needs.changes.outputs.openapi == 'true'
  steps:
    - uses: actions/checkout@v6
    - uses: actions/setup-node@v6
      with:
        node-version: '22'
    - uses: pnpm/action-setup@v4
      with:
        version: 10
    - run: pnpm install --frozen-lockfile
    - uses: extractions/setup-just@v3
    - run: just lint-openapi
```

## 参考

- [Redocly CLI 公式ドキュメント](https://redocly.com/docs/cli/)
- [Redocly CLI GitHub](https://github.com/Redocly/redocly-cli)
- [OpenAPI 3.1 変更点](https://www.openapis.org/blog/2021/02/16/migrating-from-openapi-3-0-to-3-1-0)
