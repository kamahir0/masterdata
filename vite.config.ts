import { defineConfig } from "vite";
export default defineConfig({
  base: "./",
  server: { port: 5173, strictPort: true },
  build: {
    target: "es2023",
    rolldownOptions: { checks: { moduleLevelDirective: false } },
  },
});
