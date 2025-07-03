import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(() => ({
  plugins: [solid()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`, 'agent', and 'crates'
      ignored: ["**/src-tauri/**", "**/agent/**", "**/crates/**"],
    },
  },

  build: {
    target: ["chrome120", "edge120", "firefox117", "safari17", "es2023"],
  },

  resolve: {
    dedupe: ["solid-js"],
  },
}));
