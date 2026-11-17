import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import path from "path";

const templatesSrc = path.resolve(__dirname, "../../../chromatofoor-gen/templates/tauri/src");

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: {
      "@workbench/App.svelte": path.join(templatesSrc, "App.svelte"),
      "@workbench/index.css": path.join(templatesSrc, "index.css"),
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
