import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      "@chromatofoor/ui": path.resolve(__dirname, "../../../packages/ui/src"),
    },
  },
  server: {
    port: 5173,
    proxy: {
      "/v0": "http://localhost:3000",
      "/openapi.json": "http://localhost:3000",
    },
  },
  build: {
    outDir: "../ui-dist",
    emptyOutDir: true,
    target: "es2021",
    rollupOptions: {
      output: {
        entryFileNames: "assets/workbench.js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/workbench[extname]",
      },
    },
  },
});
