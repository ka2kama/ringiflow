# ADR-028: MCP サーバー導入（PostgreSQL）

## ステータス

承認済み（2026-01-31）

置換対象: [ADR-009](009_MCPサーバー導入の見送り.md)

## コンテキスト

Claude Code を開発ツールとして使用する中で、PostgreSQL のスキーマやデータをリアルタイムに参照する需要が増加した。ADR-009 では MCP サーバーの導入を見送ったが、以下の状況変化により再検討が必要になった:

- 開発が進み、DB スキーマの参照頻度が増加
- MCP エコシステムが成熟し、安定したサーバー実装が利用可能に
- 公式 `@modelcontextprotocol/server-postgres` が 2025年7月に非推奨となり、代替パッケージの選定が必要

### 再検討のトリガー

ADR-009 で定義した再検討トリガーの1つ「Claude Code の標準機能では実現困難なワークフローが出てきた場合」に該当。MCP によるスキーマ introspection は、`psql` コマンドの手動実行より効率的なワークフローを提供する。

## 検討した選択肢

### 選択肢 1: `@zeddotdev/postgres-context-server` を導入する

Zed Industries がフォークしメンテナンスしている PostgreSQL MCP サーバー。公式パッケージの SQL injection 脆弱性を修正済み。

評価:
- 利点: アクティブにメンテナンスされている（v0.1.7）、セキュリティパッチ済み、npm ベースで導入が容易
- 欠点: Zed Industries への依存、公式パッケージではない

### 選択肢 2: `@modelcontextprotocol/server-postgres`（公式・非推奨）を導入する

Anthropic 公式の PostgreSQL MCP サーバー。2025年7月に非推奨となった。

評価:
- 利点: Anthropic 公式、ドキュメントが豊富
- 欠点: 非推奨（2025年7月）、SQL injection 脆弱性あり、メンテナンス停止

### 選択肢 3: MCP なし、justfile コマンドのみで対応する

MCP は導入せず、justfile に `db-tables`、`db-schema`、`db-query` 等のヘルパーコマンドを追加して対応する。

評価:
- 利点: 追加の依存なし、シンプル、psql コマンドの直接活用
- 欠点: MCP のスキーマ introspection が利用できない、毎回 Bash ツール経由になる

### 比較表

| 観点 | 選択肢 1（Zed fork） | 選択肢 2（公式・非推奨） | 選択肢 3（justfile のみ） |
|------|---------------------|------------------------|--------------------------|
| メンテナンス | アクティブ（v0.1.7） | 非推奨・停止 | 不要 |
| セキュリティ | パッチ済み | 脆弱性あり | psql 直接利用 |
| セットアップ | npx（簡単） | npx（簡単） | 不要 |
| スキーマ参照 | 自動 introspection | 手動クエリ | 手動クエリ |
| 依存関係 | npm パッケージ追加 | npm パッケージ追加 | 既存ツールのみ |
| 最新プラクティス | 準拠 | 違反（非推奨使用） | 準拠 |

## 決定

**選択肢 1: `@zeddotdev/postgres-context-server` を導入する** を採用する。加えて、選択肢 3 の justfile コマンドも併用する（MCP と justfile は補完関係）。

理由:

1. 最新ベストプラクティス準拠: 非推奨パッケージの回避（[latest-practices.md](../../.claude/rules/latest-practices.md)）
2. セキュリティ: SQL injection 脆弱性がパッチ済み
3. KISS: npx ベースで `.mcp.json` を1ファイル追加するだけの簡単な導入
4. 補完的アプローチ: MCP（スキーマ introspection）+ justfile（アドホックなクエリ）で用途をカバー

選択肢 2 を却下した理由:
- 非推奨パッケージの使用はプロジェクトの最新プラクティス方針に反する
- SQL injection 脆弱性が未修正

選択肢 3 だけでは不十分な理由:
- MCP のスキーマ introspection は Bash 経由の手動クエリより効率的
- ただし justfile コマンドはアドホックなクエリに有用なため併用する

### Redis MCP について

Redis MCP サーバーは導入しない。理由:
- Python 依存が必要（技術スタックの増加）
- Redis の用途がセッション管理・CSRF トークンに限定されており、参照頻度が低い
- `just redis-keys` / `just redis-get` で十分

## 帰結

### 肯定的な影響

- Claude Code が PostgreSQL スキーマをリアルタイムに参照可能になる
- justfile コマンドにより手軽な DB/Redis 操作が可能になる
- 開発効率の向上（スキーマ確認のためにコマンドを手動実行する手間が削減）

### 否定的な影響・トレードオフ

- npm パッケージへの依存が1つ増加（`@zeddotdev/postgres-context-server`）
- `.mcp.json` に接続情報がハードコードされる（開発環境のみなので許容）
- MCP サーバーのプロセスが Claude Code 実行中に常駐する

### 技術的な注意点

#### 1. `bin` フィールドの欠如

`@zeddotdev/postgres-context-server` は `package.json` に `bin` フィールドを定義していない。このため、一般的な MCP 設定パターンである `npx -y パッケージ名` では `could not determine executable to run` エラーが発生する。

この問題が上流で修正された場合は、`npx -y パッケージ名` のシンプルな形式に戻せる。

#### 2. Volta + Node.js v24 環境での ESM モジュール解決

`npx --package PKG node -e "import 'PKG'"` の形式は、Volta + Node.js v24 環境では動作しない。

原因: Node.js の ESM モジュール解決は `NODE_PATH` を無視し、`cwd`（カレントディレクトリ）からの `node_modules` 探索に依存する。`npx --package` はパッケージを一時キャッシュにインストールするが、`node` コマンドの `cwd` は変更しないため、ESM の import 文がパッケージを見つけられない。

回避策として、起動スクリプト（`scripts/mcp-postgres.sh`）を使用する:

1. 決定論的なディレクトリ（`$XDG_CACHE_HOME/ringiflow-mcp-postgres`）にパッケージをインストール
2. そのディレクトリに `cd` してから `node` を実行

```json
{
  "command": "bash",
  "args": ["scripts/mcp-postgres.sh"]
}
```

### 関連ドキュメント

- 廃止: [ADR-009: MCP サーバー導入の見送り](009_MCPサーバー導入の見送り.md)
- 設定: [`.mcp.json`](../../.mcp.json)
- 設定: [`.claude/settings.json`](../../.claude/settings.json)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-31 | 初版作成 |
| 2026-01-31 | `bin` フィールド欠如の回避策を追記、`.mcp.json` の起動コマンドを修正 |
| 2026-01-31 | Volta + Node.js v24 の ESM モジュール解決問題に対応、起動スクリプト方式に移行 |
