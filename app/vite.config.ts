import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": resolve(__dirname, "."),
    },
  },
  server: {
    port: 4000,
    proxy: {
      "/api": "http://localhost:3131",
      "/sse": "http://localhost:3131",
    },
  },
});
