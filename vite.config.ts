import { defineConfig } from "vite";
import { readFileSync } from "node:fs";

const { version } = JSON.parse(
  readFileSync(new URL("./package.json", import.meta.url), "utf8"),
);
export default defineConfig({
  base: "./",
  define: { __APP_VERSION__: JSON.stringify(version) },
  server: { port: 5173, strictPort: true },
  build: {
    target: "es2023",
    rolldownOptions: { checks: { moduleLevelDirective: false } },
  },
});
