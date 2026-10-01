import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1（Svelte 5）
//
// 输出目录约定（per tauri.conf.json frontendDist = "../dist"）:
//   apps/cats-client/src-tauri/tauri.conf.json → frontendDist = "../dist"
//   即从 src-tauri/ 出发, ../dist = apps/cats-client/dist

export default defineConfig({
  base: "./",
  plugins: [svelte()],
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
  },
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
});