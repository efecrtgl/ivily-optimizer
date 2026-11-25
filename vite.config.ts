import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Tauri, geliştirme sırasında sabit bir port bekler. Üretimde varlıklar
// uygulamaya gömülür; çalışma zamanında hiçbir web sunucusu çalışmaz.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, host: "127.0.0.1" },
  envPrefix: ["VITE_", "TAURI_ENV_"],
  build: {
    target: "es2022",
    sourcemap: false,
    chunkSizeWarningLimit: 800,
  },
});
