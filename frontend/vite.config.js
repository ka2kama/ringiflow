/**
 * Vite 設定
 *
 * ポート番号はルートの .env ファイルで設定（justfile の dotenv-load で読み込み）
 * 技術詳細: [Vite](../docs/06_ナレッジベース/frontend/Vite.md)
 */

import { defineConfig } from "vite";
import elmPlugin from "vite-plugin-elm";

export default defineConfig({
  plugins: [elmPlugin()],

  // SPA モード: 存在しないパスは index.html にフォールバック
  appType: "spa",

  server: {
    port: parseInt(process.env.VITE_PORT),
    // 開発環境で CORS を回避するため Rust バックエンドにプロキシ
    proxy: {
      "/api": {
        target: `http://localhost:${process.env.BFF_PORT}`,
        changeOrigin: true,
      },
    },
  },

  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
