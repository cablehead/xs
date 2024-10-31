import { defineConfig } from "vite";
import deno from "@deno/vite-plugin";
import solid from "vite-plugin-solid";

// https://vite.dev/config/
export default defineConfig({
  plugins: [deno(), solid()],
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:3021",
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/api/, ""),
      },
    },
  },
});
