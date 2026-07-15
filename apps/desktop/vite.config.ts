// defineConfig comes from vitest/config rather than vite so the `test` block
// below is typed.
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";

const DEV_PORT = 1420;

export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: { "@": path.resolve(__dirname, "./src") },
  },

  // Assets are bundled locally and never loaded from a remote origin, so a
  // relative base keeps the bundle portable inside the installer.
  base: "./",

  server: {
    port: DEV_PORT,
    // Fail loudly rather than serving on a port tauri.conf.json is not
    // pointing at.
    strictPort: true,
    host: "127.0.0.1",
  },

  clearScreen: false,

  envPrefix: ["VITE_", "AUDIS_"],

  build: {
    // WebView2 is evergreen Chromium, so a modern baseline is safe.
    target: "chrome120",
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    minify: process.env.TAURI_ENV_DEBUG ? false : "esbuild",
    outDir: "dist",
    emptyOutDir: true,
  },

  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
});
