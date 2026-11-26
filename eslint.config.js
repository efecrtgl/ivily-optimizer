// Arayüz için statik denetim. Proje TypeScript kullanmaz; tanımsız değişkenler,
// kullanılmayan kod ve Svelte'e özgü hatalar bu yapılandırmayla yakalanır.
import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import globals from "globals";

export default [
  { ignores: ["dist/", "node_modules/", "src-tauri/", "dist-release/"] },
  js.configs.recommended,
  ...svelte.configs.recommended,
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
      globals: { ...globals.browser },
    },
    rules: {
      "no-unused-vars": ["error", { argsIgnorePattern: "^_", caughtErrors: "none" }],
      eqeqeq: ["error", "smart"],
      "no-var": "error",
      "prefer-const": ["error", { destructuring: "all" }],
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.js"],
    rules: {
      // `let { ... } = $props()` Svelte 5'in önerdiği yazımdır.
      "prefer-const": "off",
      // İşlev içinde yerel olarak kullanılan Map/Set'ler bilinçli olarak
      // reaktif değildir; SvelteMap/SvelteSet gereksiz izleme ekler.
      "svelte/prefer-svelte-reactivity": "off",
      // Anahtarsız {#each} blokları mevcut kodda yaygın; anahtar eklemek
      // DOM yeniden kullanımını değiştirdiği için her liste ayrı ayrı
      // değerlendirilmelidir (bkz. docs/DENETIM.md).
      "svelte/require-each-key": "off",
    },
  },
  {
    files: ["scripts/**/*.{js,mjs}", "*.config.js"],
    languageOptions: { globals: { ...globals.node } },
  },
  {
    files: ["**/*.test.js"],
    languageOptions: { globals: { ...globals.node } },
  },
];
