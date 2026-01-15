/**
 * Vite 設定
 *
 * 技術詳細: [Vite](../../docs/05_技術ノート/Vite.md)
 */

import { defineConfig } from "vite";
import elmPlugin from "vite-plugin-elm";

export default defineConfig({
  plugins: [elmPlugin()],

  server: {
    port: 5173,
    // 開発環境で CORS を回避するため Rust バックエンドにプロキシ
    proxy: {
      "/api": {
        target: "http://localhost:3000",
        changeOrigin: true,
      },
    },
  },

  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
