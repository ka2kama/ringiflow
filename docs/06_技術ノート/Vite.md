# Vite（ヴィート）

## 概要

Vite（ヴィート）は次世代フロントエンドビルドツール。
ES モジュールのネイティブサポートにより、高速な開発体験を提供する。

主な特徴:
- **高速な起動**: バンドル不要で開発サーバーを起動
- **高速な HMR（ホットモジュールリプレースメント）**: 変更ファイルのみを更新
- **シンプルな設定**: デフォルト設定が実用的

## 他ツールとの比較

| ツール | 起動速度 | 設定の複雑さ | Elm サポート |
|--------|----------|--------------|--------------|
| Vite（ヴィート） | 速い | 低い | プラグイン |
| Webpack（ウェブパック） | 遅い | 高い | loader |
| Parcel（パーセル） | 速い | 最低 | 限定的 |
| esbuild（イーエスビルド） | 最速 | 中程度 | なし |

Vite は速度と設定のバランスが良く、vite-plugin-elm による Elm（エルム）サポートも安定している。

## 基本設定

```javascript
import { defineConfig } from "vite";
import elmPlugin from "vite-plugin-elm";

export default defineConfig({
  plugins: [elmPlugin()],
  server: {
    port: 15173,
  },
  build: {
    outDir: "dist",
  },
});
```

`defineConfig()` は TypeScript の型補完を有効にするヘルパー関数。

## vite-plugin-elm

Elm ファイルを JavaScript モジュールとして import 可能にするプラグイン。

```javascript
// main.js
import { Elm } from "./Main.elm";

const app = Elm.Main.init({
  node: document.getElementById("app"),
  flags: { apiBaseUrl: "/api" }
});
```

### 動作モード

| モード | 動作 |
|--------|------|
| 開発時 | デバッグ情報付きでコンパイル、HMR 対応 |
| 本番時 | 最適化コンパイル（`--optimize` フラグ） |

### オプション

```javascript
elmPlugin({
  debug: true,    // デバッグモードを強制（開発時は自動で有効）
  optimize: true, // 最適化を強制（本番時は自動で有効）
})
```

通常はデフォルト値で十分。

## 開発サーバーのプロキシ設定

開発環境では、フロントエンド（15173）とバックエンド（13000）が別ポートで動作する。
CORS の問題を回避するため、Vite のプロキシ機能を使用する。

### リクエストフロー

```
ブラウザ → localhost:15173/api/users
        → Vite プロキシ
        → localhost:13000/api/users
        → バックエンド
```

### 設定例

```javascript
server: {
  proxy: {
    "/api": {
      target: "http://localhost:13000",
      changeOrigin: true,
    },
  },
},
```

| オプション | 説明 |
|-----------|------|
| `target` | プロキシ先 URL |
| `changeOrigin` | Origin ヘッダーを target のホストに書き換え |

### 本番環境との違い

本番環境ではプロキシは不要。以下のいずれかで対応:
- 同一ドメインでフロントエンド/バックエンドを提供
- CORS ヘッダーを適切に設定

## ビルド設定

```javascript
build: {
  outDir: "dist",      // 出力ディレクトリ
  emptyOutDir: true,   // ビルド前にディレクトリを空にする
},
```

### デプロイ

```bash
pnpm run build        # dist/ を生成
# dist/ の内容を CDN/S3 にアップロード
```

## 環境変数管理

### justfile 経由の環境変数読み込み

このプロジェクトでは、ポート番号などの設定をルートの `.env` ファイルで管理し、justfile の `set dotenv-load := true` で読み込む。

```javascript
// vite.config.js
server: {
  port: parseInt(process.env.VITE_PORT),
  proxy: {
    "/api": {
      target: `http://localhost:${process.env.BFF_PORT}`,
      changeOrigin: true,
    },
  },
},
```

### 開発サーバーの起動方法

環境変数を正しく読み込むため、必ず `just` コマンドを使用する。

```bash
# 正しい方法
just dev-web     # フロントエンド開発サーバー
just dev         # 全体（バックエンド + フロントエンド）

# 避けるべき方法
cd frontend && pnpm run dev  # .env が読み込まれない
```

`pnpm run dev` を直接実行すると `.env` が読み込まれず、以下のエラーが発生する:

```
TypeError: Invalid URL
  code: 'ERR_INVALID_URL',
  input: 'http://localhost:undefined'
```

### 環境変数一覧

| 変数名 | 用途 | 例 |
|--------|------|-----|
| `VITE_PORT` | Vite 開発サーバーのポート | 15173 |
| `BFF_PORT` | BFF サーバーのポート（プロキシ先） | 13000 |

## プロジェクトでの使用

### ファイル構成

```
apps/web/
├── vite.config.js    # Vite 設定
├── index.html        # エントリーポイント
├── src/
│   ├── main.js       # JS エントリー
│   └── Main.elm      # Elm エントリー
└── dist/             # ビルド出力
```

### 設定ファイル

`apps/web/vite.config.js`

## 関連リソース

- [Vite 公式ドキュメント](https://vitejs.dev/)
- [vite-plugin-elm](https://github.com/hmsk/vite-plugin-elm)

---

## 変更履歴

| 日付 | 変更内容 |
|------|---------|
| 2026-01-15 | 初版作成 |
