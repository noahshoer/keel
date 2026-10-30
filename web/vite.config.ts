import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

const host = process.env["TAURI_DEV_HOST"];

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    host: host || false,
    // Tauri's iOS dev pattern: the device reaches HMR on the Mac's address.
    ...(host ? { hmr: { protocol: "ws", host, port: 1421 } } : {}),
  },
  build: {
    target: "safari18",
  },
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.ts"],
  },
});
