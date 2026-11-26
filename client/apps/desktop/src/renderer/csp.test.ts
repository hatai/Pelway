import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";

test("画面の HTML の CSP が同梱のスクリプトだけを許す", () => {
  const html = readFileSync(join(import.meta.dir, "index.html"), "utf8");
  const match = html.match(/<meta\s+http-equiv="Content-Security-Policy"\s+content="([^"]+)"/);
  expect(match).not.toBeNull();
  const directives = new Map(
    (match?.[1] ?? "").split(";").map((part) => {
      const [name = "", ...values] = part.trim().split(/\s+/);
      return [name, values] as const;
    }),
  );
  expect(directives.get("default-src")).toEqual(["'self'"]);
  expect(directives.get("script-src")).toEqual(["'self'"]);
  expect(directives.get("object-src")).toEqual(["'none'"]);
  // frame-ancestors は <meta> の CSP では効かないので書かない（CSP の仕様）。
  expect(directives.has("frame-ancestors")).toBe(false);
});
