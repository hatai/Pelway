import { afterEach, describe, expect, test } from "bun:test";
import { mkdirSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { findCanaryBinary } from "./canary-bun";

const TARGET = "linux-x64";
const roots: string[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

// client/node_modules/bun と、本体の包み @oven/bun-<OS> を置いた木を作る。
function makeTree() {
  const root = mkdtempSync(join(tmpdir(), "canary-bun-"));
  roots.push(root);
  const client = join(root, "client");
  mkdirSync(join(client, "node_modules", "bun"), { recursive: true });
  writeFileSync(join(client, "node_modules", "bun", "package.json"), '{"name":"bun"}');
  return { root, client };
}

function makeBinaryPackage(dir: string) {
  mkdirSync(join(dir, "bin"), { recursive: true });
  writeFileSync(join(dir, "package.json"), `{"name":"@oven/bun-${TARGET}"}`);
  writeFileSync(join(dir, "bin", "bun"), "");
  return join(dir, "bin", "bun");
}

describe("findCanaryBinary", () => {
  test("client/node_modules の中の本体を返す", () => {
    const { client } = makeTree();
    makeBinaryPackage(join(client, "node_modules", "@oven", `bun-${TARGET}`));
    expect(findCanaryBinary(client, TARGET, "linux")).toEndWith(join("@oven", `bun-${TARGET}`, "bin", "bun"));
  });

  test("本体が client/node_modules の外（親の node_modules）にしか無いときは返さない", () => {
    const { root, client } = makeTree();
    makeBinaryPackage(join(root, "node_modules", "@oven", `bun-${TARGET}`));
    expect(findCanaryBinary(client, TARGET, "linux")).toBeUndefined();
  });

  test("client/node_modules の中の包みの bin が外を指すときは返さない", () => {
    const { root, client } = makeTree();
    // 包みは中に実体で置き（解決は中で成功する）、bin だけを外へ向ける。
    // Windows でも権限の要らないジャンクションで作る（POSIX では type は無視されシンボリックリンクになる）。
    const outsideBin = join(root, "outside", "bin");
    mkdirSync(outsideBin, { recursive: true });
    writeFileSync(join(outsideBin, "bun"), "");
    const pkg = join(client, "node_modules", "@oven", `bun-${TARGET}`);
    mkdirSync(pkg, { recursive: true });
    writeFileSync(join(pkg, "package.json"), `{"name":"@oven/bun-${TARGET}"}`);
    symlinkSync(outsideBin, join(pkg, "bin"), "junction");
    expect(findCanaryBinary(client, TARGET, "linux")).toBeUndefined();
  });

  test("bun の包みが無いときは返さない", () => {
    const root = mkdtempSync(join(tmpdir(), "canary-bun-"));
    roots.push(root);
    expect(findCanaryBinary(join(root, "client"), TARGET, "linux")).toBeUndefined();
  });
});
