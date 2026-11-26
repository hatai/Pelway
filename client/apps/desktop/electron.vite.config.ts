// electron-vite の設定（ADR 0014）。main・preload・画面（renderer）を別々に組み立てる。
import { resolve } from "node:path";
import { defineConfig } from "electron-vite";
import { octane } from "octane/compiler/vite";

export default defineConfig({
  main: {
    build: {
      rollupOptions: { input: { index: "src/main/index.ts" } },
    },
  },
  preload: {
    build: {
      // sandbox: true の preload は require できるモジュールが electron などに限られるので、
      // 依存（@pelway/contracts）を含めて 1 つの CJS のファイルにまとめる。
      externalizeDeps: false,
      isolatedEntries: true,
      rollupOptions: {
        input: { index: "src/preload/index.ts" },
        external: ["electron"],
        output: { format: "cjs", entryFileNames: "[name].cjs" },
      },
    },
  },
  renderer: {
    root: "src/renderer",
    plugins: [octane()],
    build: {
      rollupOptions: {
        input: {
          index: resolve(import.meta.dirname, "src/renderer/index.html"),
        },
      },
    },
  },
});
