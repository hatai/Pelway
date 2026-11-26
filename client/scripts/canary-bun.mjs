// canary 版の bun（bun lint のため。ADR 0014）を起動する。
// 依存の postinstall は走らせない（bunfig.toml の ignoreScripts）ので、npm の `bun` の bin は
// 置き換え前のままになる。代わりに、bun.lock の sha512 で照合して入った OS ごとの本体
// `@oven/bun-<OS>` の実行ファイルを直接起動する。見つからなければ失敗する。
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";

function target() {
  const arch = { x64: "x64", arm64: "aarch64" }[process.arch];
  const os = { linux: "linux", darwin: "darwin", win32: "windows", freebsd: "freebsd" }[process.platform];
  if (arch === undefined || os === undefined) return undefined;
  const musl = os === "linux" && process.report?.getReport().header.glibcVersionRuntime === undefined;
  return `${os}-${arch}${musl ? "-musl" : ""}`;
}

const name = `@oven/bun-${target()}`;
let binary;
try {
  const require = createRequire(createRequire(import.meta.url).resolve("bun/package.json"));
  binary = join(
    dirname(require.resolve(`${name}/package.json`)),
    "bin",
    process.platform === "win32" ? "bun.exe" : "bun",
  );
} catch {
  binary = undefined;
}
if (binary === undefined || !existsSync(binary)) {
  console.error(`canary-bun: ${name} の実行ファイルが見つからない。mise run client:bootstrap で入れ直す。`);
  process.exit(1);
}
const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
process.exit(result.status ?? 1);
