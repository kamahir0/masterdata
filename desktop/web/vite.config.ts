import { defineConfig } from "vite";
export default defineConfig({
  build: {
    rollupOptions: {
      onwarn(warning, warn) {
        // This renderer is entirely client-side; dependency RSC boundaries have no
        // meaning in a native WebView. Retain every other bundler warning.
        if (
          warning.code === "MODULE_LEVEL_DIRECTIVE" &&
          warning.message.includes('"use client"')
        )
          return;
        warn(warning);
      },
    },
  },
});
