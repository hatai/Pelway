import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import {
  applyImprovements,
  compareBaseline,
  parseDiagnostics,
  runTypeRatchet,
  type Baseline,
  type CheckResult,
  type RatchetIo,
} from "./type-ratchet";

const TS4111 =
  "apps/desktop/src/main/index.ts(17,63): error TS4111: Property 'ELECTRON_RENDERER_URL' comes from an index signature, so it must be accessed with ['ELECTRON_RENDERER_URL'].";

function result(stdout: string, status = stdout.length === 0 ? 0 : 1, stderr = ""): CheckResult {
  return { status, stdout, stderr };
}

function io(baseline: Baseline, checks: Record<string, CheckResult>) {
  const writes: string[] = [];
  const errors: string[] = [];
  const io: RatchetIo = {
    readBaseline: () => `${JSON.stringify(baseline, null, 2)}\n`,
    writeBaseline: (text) => writes.push(text),
    check: (option) => checks[option] ?? result("", 0),
    out: () => {},
    err: (message) => errors.push(message),
  };
  return { io, writes, errors };
}

describe("parseDiagnostics", () => {
  test("stdout の診断行だけを数え、同一行は 1 件にする", () => {
    const diagnostics = parseDiagnostics(`${TS4111}\n${TS4111}\n`);
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]?.file).toBe("apps/desktop/src/main/index.ts");
  });

  test("Windows 形式の区切りを client/ からの相対パスに揃える", () => {
    const line =
      "apps\\desktop\\src\\main\\index.ts(17,63): error TS4111: Property 'ELECTRON_RENDERER_URL' comes from an index signature.";
    expect(parseDiagnostics(`${line}\n`)[0]?.file).toBe("apps/desktop/src/main/index.ts");
  });

  test("stderr の要約のような形式外の stdout は受け付けない", () => {
    expect(() => parseDiagnostics("Found 1 error in 1 file\n")).toThrow("bun check の stdout の形式");
  });

  test("診断に続く字下げ行は続きのメッセージとして読み飛ばす", () => {
    const multiline = `${TS4111}\n  Type 'undefined' is not assignable to type 'string'.\n`;
    expect(parseDiagnostics(multiline)).toHaveLength(1);
  });

  test("GitHub Actions の注釈行は tsc 形式の重複として読み飛ばす", () => {
    const annotation =
      "::error file=client/apps/desktop/src/main/index.ts,line=17,col=63,endLine=17,endColumn=84,title=TS4111::Property 'ELECTRON_RENDERER_URL' comes from an index signature, so it must be accessed with ['ELECTRON_RENDERER_URL'].";
    expect(parseDiagnostics(`${TS4111}\n${annotation}\n`)).toHaveLength(1);
  });

  test("GitHub Actions の注釈行だけでは診断件数にしない", () => {
    const annotation =
      "::error file=client/apps/desktop/src/main/index.ts,line=17,col=63,endLine=17,endColumn=84,title=TS4111::Property 'ELECTRON_RENDERER_URL' comes from an index signature, so it must be accessed with ['ELECTRON_RENDERER_URL'].";
    expect(parseDiagnostics(`${annotation}\n`)).toHaveLength(0);
  });
});

describe("compareBaseline", () => {
  test("基準値より増えた診断を悪化として返す", () => {
    const comparison = compareBaseline(
      { noPropertyAccessFromIndexSignature: { "a.ts": 1 } },
      { noPropertyAccessFromIndexSignature: { "a.ts": 2 } },
    );
    expect(comparison.regressions).toEqual([
      { option: "noPropertyAccessFromIndexSignature", file: "a.ts", baseline: 1, current: 2 },
    ]);
    expect(comparison.improvements).toEqual([]);
  });

  test("基準値より減った診断を改善として返す", () => {
    const comparison = compareBaseline(
      { noPropertyAccessFromIndexSignature: { "a.ts": 1 } },
      { noPropertyAccessFromIndexSignature: {} },
    );
    expect(comparison.improvements).toEqual([
      { option: "noPropertyAccessFromIndexSignature", file: "a.ts", baseline: 1, current: 0 },
    ]);
  });
});

describe("applyImprovements", () => {
  test("改善分だけを下げ、悪化した新規ファイルを書かない", () => {
    const next = applyImprovements(
      { noPropertyAccessFromIndexSignature: { "a.ts": 2, "b.ts": 1 } },
      { noPropertyAccessFromIndexSignature: { "a.ts": 1, "c.ts": 3 } },
    );
    expect(next).toEqual({ noPropertyAccessFromIndexSignature: { "a.ts": 1 } });
  });
});

describe("runTypeRatchet", () => {
  test("検査で増えていたら失敗し、基準値を書かない", () => {
    const harness = io(
      { noPropertyAccessFromIndexSignature: { "apps/desktop/src/main/index.ts": 1 } },
      { noPropertyAccessFromIndexSignature: result(`${TS4111}\n${TS4111.replace("(17,63)", "(18,63)")}\n`) },
    );
    expect(runTypeRatchet([], harness.io)).toBe(1);
    expect(harness.writes).toEqual([]);
  });

  test("検査で減っていても更新するまでは失敗し、基準値を書かない", () => {
    const harness = io(
      { noPropertyAccessFromIndexSignature: { "apps/desktop/src/main/index.ts": 1 } },
      { noPropertyAccessFromIndexSignature: result("") },
    );
    expect(runTypeRatchet([], harness.io)).toBe(1);
    expect(harness.writes).toEqual([]);
  });

  test("更新は改善分だけを書き、悪化が無ければ成功する", () => {
    const harness = io(
      { noPropertyAccessFromIndexSignature: { "apps/desktop/src/main/index.ts": 1 } },
      { noPropertyAccessFromIndexSignature: result("") },
    );
    expect(runTypeRatchet(["--write"], harness.io)).toBe(0);
    expect(JSON.parse(harness.writes[0] ?? "")).toEqual({ noPropertyAccessFromIndexSignature: {} });
  });

  test("更新は悪化分を書かず、改善を書いても悪化が残れば失敗する", () => {
    const second = TS4111.replace("apps/desktop/src/main/index.ts", "apps/desktop/src/main/other.ts");
    const harness = io(
      { noPropertyAccessFromIndexSignature: { "apps/desktop/src/main/index.ts": 2 } },
      { noPropertyAccessFromIndexSignature: result(`${TS4111}\n${second}\n`) },
    );
    expect(runTypeRatchet(["--write"], harness.io)).toBe(1);
    expect(JSON.parse(harness.writes[0] ?? "")).toEqual({
      noPropertyAccessFromIndexSignature: { "apps/desktop/src/main/index.ts": 1 },
    });
  });

  test("--add は現在 1 件以上あるオプションを現在値で追加する", () => {
    const harness = io({}, { noPropertyAccessFromIndexSignature: result(`${TS4111}\n`) });
    expect(runTypeRatchet(["--add=noPropertyAccessFromIndexSignature"], harness.io)).toBe(0);
    expect(JSON.parse(harness.writes[0] ?? "")).toEqual({
      noPropertyAccessFromIndexSignature: { "apps/desktop/src/main/index.ts": 1 },
    });
  });

  test("--add は現在 0 件のオプションを拒否し、基準値を書かない", () => {
    const harness = io({}, { noUnusedLocals: result("") });
    expect(runTypeRatchet(["--add=noUnusedLocals"], harness.io)).toBe(1);
    expect(harness.writes).toEqual([]);
    expect(harness.errors.join("\n")).toContain("0 件");
  });

  test("検査器が診断を出さずに失敗したときは fail closed にする", () => {
    const harness = io(
      { noPropertyAccessFromIndexSignature: {} },
      { noPropertyAccessFromIndexSignature: result("", 2, "unknown option") },
    );
    expect(runTypeRatchet([], harness.io)).toBe(1);
    expect(harness.writes).toEqual([]);
  });
});

describe("mise の依存構造", () => {
  test("client:type-ratchet が bootstrap と client:verify の間に入り、verify-static から回る", () => {
    const mise = readFileSync(join(import.meta.dirname, "..", "..", "mise.toml"), "utf8");
    const ratchet =
      mise.match(/\[tasks\."client:type-ratchet"\]\n(?<body>[\s\S]*?)(?=\n\[tasks\.)/)?.groups?.["body"] ?? "";
    const verify = mise.match(/\[tasks\."client:verify"\]\n(?<body>[\s\S]*?)(?=\n\[tasks\.)/)?.groups?.["body"] ?? "";
    const verifyStatic =
      mise.match(/\[tasks\.verify-static\]\n(?<body>[\s\S]*?)(?=\n\[tasks\.)/)?.groups?.["body"] ?? "";
    expect(ratchet).toContain('depends = ["client:bootstrap"]');
    expect(verify).toContain('depends = ["client:type-ratchet"]');
    expect(verify).not.toContain("bun install --frozen-lockfile");
    expect(verifyStatic).toContain('"client:verify"');
  });
});
