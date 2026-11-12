import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Unit and property tests run in Node; component tests opt into jsdom with a
// `// @vitest-environment jsdom` line at the top of the file.
export default defineConfig({
  plugins: [react()],
  test: {
    include: ["src/**/*.test.{ts,tsx}"],
    environment: "node",
  },
});
