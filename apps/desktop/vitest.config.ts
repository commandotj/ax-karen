// Separate from vite.config.ts so `vue-tsc` does not type-check Vitest's config extension.
import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    coverage: {
      provider: "v8",
      include: ["src/**"],
      exclude: ["src/**/*.test.ts"],
      // The gate only moves up; see RFC 0017.
      thresholds: { lines: 100 },
    },
  },
});
