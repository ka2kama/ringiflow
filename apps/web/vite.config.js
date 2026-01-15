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
    port: parseInt(process.env.VITE_PORT || "15173"),
    // 開発環境で CORS を回避するため Rust バックエンドにプロキシ
    proxy: {
      "/api": {
        target: `http://localhost:${process.env.BFF_PORT || "13000"}`,
        changeOrigin: true,
      },
    },
  },

  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
