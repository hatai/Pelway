import { describe, expect, test } from "bun:test";
import { resolveAppAsset } from "./app-protocol";

const ROOT = "/opt/pelway/out/renderer";

describe("resolveAppAsset", () => {
  test("同梱の資産の中のファイルを指す", () => {
    expect(resolveAppAsset(ROOT, "pelway://app/index.html")).toBe(`${ROOT}/index.html`);
    expect(resolveAppAsset(ROOT, "pelway://app/assets/main.js?x=1")).toBe(`${ROOT}/assets/main.js`);
  });

  test("ルートは index.html を指す", () => {
    expect(resolveAppAsset(ROOT, "pelway://app/")).toBe(`${ROOT}/index.html`);
  });

  test("URL の解釈で .. が消える要求は、同梱の資産の中に留まる", () => {
    expect(resolveAppAsset(ROOT, "pelway://app/../../etc/passwd")).toBe(`${ROOT}/etc/passwd`);
    expect(resolveAppAsset(ROOT, "pelway://app/%2e%2e/%2e%2e/etc/passwd")).toBe(`${ROOT}/etc/passwd`);
  });

  test("復号すると同梱の資産の外を指す要求を拒む", () => {
    expect(resolveAppAsset(ROOT, "pelway://app/..%2f..%2fetc%2fpasswd")).toBeNull();
    expect(resolveAppAsset(ROOT, "pelway://app/%2e%2e%2f%2e%2e%2fetc%2fpasswd")).toBeNull();
  });

  test("別のホストの要求を拒む", () => {
    expect(resolveAppAsset(ROOT, "pelway://other/index.html")).toBeNull();
  });
});
