// canary 版の bun（bun lint のため。ADR 0014）を起動する。mise の Node で動かす（`node scripts/canary-bun.ts lint`）。
// 依存の postinstall は走らせない（bunfig.toml の ignoreScripts）ので、npm の `bun` の bin は
// 置き換え前のままになる。代わりに、bun.lock の sha512 で照合して入った OS ごとの本体
// `@oven/bun-<OS>` の実行ファイルを直接起動する。見つからなければ失敗する。
import { spawnSync } from "node:child_process";
import { existsSync, realpathSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, sep } from "node:path";

/** この機械の本体の包みの名前の後ろの部分（例: linux-x64、linux-x64-musl、darwin-aarch64）。 */
export function currentTarget(): string | undefined {
  const arch = ({ x64: "x64", arm64: "aarch64" } as Record<string, string>)[process.arch];
  const os = ({ linux: "linux", darwin: "darwin", win32: "windows", freebsd: "freebsd" } as Record<string, string>)[
    process.platform
  ];
  if (arch === undefined || os === undefined) return undefined;
  const report = process.report?.getReport() as { header?: { glibcVersionRuntime?: string } } | undefined;
  const header = report?.header;
  const musl = os === "linux" && header?.glibcVersionRuntime === undefined;
  return `${os}-${arch}${musl ? "-musl" : ""}`;
}

/**
 * clientDir/node_modules の中にある `@oven/bun-<target>` の実行ファイルのパスを返す。
 * clientDir/node_modules の外（親の node_modules、NODE_PATH、外を指すシンボリックリンク）で見つかった本体は
 * bun.lock で照合していないので返さない。見つからなければ undefined。
 */
export function findCanaryBinary(clientDir: string, target: string, platform: string): string | undefined {
  try {
    const modules = realpathSync(join(clientDir, "node_modules")) + sep;
    const fromClient = createRequire(join(clientDir, "package.json"));
    const fromBun = createRequire(fromClient.resolve("bun/package.json"));
    const pkg = fromBun.resolve(`@oven/bun-${target}/package.json`);
    const binary = join(dirname(pkg), "bin", platform === "win32" ? "bun.exe" : "bun");
    return existsSync(binary) && realpathSync(binary).startsWith(modules) ? binary : undefined;
  } catch {
    return undefined;
  }
}

if (import.meta.main) {
  const target = currentTarget();
  const binary =
    target === undefined ? undefined : findCanaryBinary(join(import.meta.dirname, ".."), target, process.platform);
  if (binary === undefined) {
    console.error(
      `canary-bun: @oven/bun-${target} の実行ファイルが見つからない。mise run client:bootstrap で入れ直す。`,
    );
    process.exit(1);
  }
  const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });
  process.exit(result.status ?? 1);
}
