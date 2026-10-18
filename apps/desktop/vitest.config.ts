// Separate from vite.config.ts so `vue-tsc` does not type-check Vitest's config extension.
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  test: { environment: "jsdom", include: ["src/**/*.test.ts"] },
});
