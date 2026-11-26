// bun test の前に読み込み、画面の部品（.tsx）を Octane のコンパイラで画面用に変換する。
// Octane が用意する bun 向けの読み込み（octane/compiler/register）はサーバー向けで、
// 部品の試験には使えないため、画面用（environment: "client"）で呼ぶ。
// 対象は画面のコード（packages/ui と apps/desktop/src/renderer）だけ。フックを .ts に
// 書いたときもコンパイラを通す必要があるので、.ts も含める。パスの区切りは / と \（Windows）の両方を受ける。
import { plugin } from "bun";
import { readFileSync } from "node:fs";
import { createOctaneCompiler } from "octane/compiler/bundler";

const compiler = createOctaneCompiler({
  root: process.cwd(),
  environment: "client",
  hmr: false,
  dev: false,
});

plugin({
  name: "pelway-octane-client",
  setup(build) {
    build.onLoad(
      { filter: /[\\/](packages[\\/]ui|apps[\\/]desktop[\\/]src[\\/]renderer)[\\/].+\.tsx?$/ },
      ({ path }) => {
        const source = readFileSync(path, "utf8");
        const result = compiler.transform(source, path, {
          environment: "client",
        });
        const loader = path.endsWith(".tsx") ? "tsx" : "ts";
        if (result === null || result.kind === "none") {
          return { contents: source, loader };
        }
        return { contents: result.code, loader };
      },
    );
  },
});
