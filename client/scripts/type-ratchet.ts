// TypeScript の未導入の厳格化オプションについて、bun check のエラー件数を基準値以下に縛る。
// 基準値の形は { オプション名: { client/ からの相対パス: 件数 } }。更新は件数を下げる方向だけで、
// 増えた件数を自動で承認しない。
import { spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

export type CountsByFile = Record<string, number>;
export type Baseline = Record<string, CountsByFile>;

export interface Diagnostic {
  file: string;
  line: number;
  column: number;
  code: string;
  message: string;
  raw: string;
}

export interface CheckResult {
  status: number | null;
  stdout: string;
  stderr: string;
}

export interface RatchetIo {
  readBaseline: () => string;
  writeBaseline: (text: string) => void;
  check: (option: string) => CheckResult;
  out: (message: string) => void;
  err: (message: string) => void;
}

export interface CountChange {
  option: string;
  file: string;
  baseline: number;
  current: number;
}

export interface Comparison {
  improvements: CountChange[];
  regressions: CountChange[];
}

const DIAGNOSTIC_PATTERN = /^(.*)\((\d+),(\d+)\): error (TS\d+): (.*)$/;
const OPTION_PATTERN = /^[a-z][a-zA-Z0-9]*$/;

/** stdout の tsc 形式の診断行を読む。形式外の空でない行があれば例外にし、黙って成功にしない。 */
export function parseDiagnostics(stdout: string): Diagnostic[] {
  const seen = new Set<string>();
  const diagnostics: Diagnostic[] = [];
  for (const rawLine of stdout.split(/\r?\n/)) {
    if (rawLine.trim() === "") continue;
    const match = DIAGNOSTIC_PATTERN.exec(rawLine);
    if (match === null) {
      // tsc の長い診断は、診断の頭の行に続く字下げ行に分かれる。件数には数えずに読み飛ばす。
      if (/^\s/.test(rawLine)) continue;
      throw new Error(`bun check の stdout の形式が想定と違います: ${rawLine}`);
    }
    if (seen.has(rawLine)) continue;
    seen.add(rawLine);
    const file = match[1] ?? "";
    const line = match[2] ?? "0";
    const column = match[3] ?? "0";
    const code = match[4] ?? "";
    const message = match[5] ?? "";
    diagnostics.push({
      file: file.replaceAll("\\", "/").replace(/^\.\//, ""),
      line: Number(line),
      column: Number(column),
      code,
      message,
      raw: rawLine,
    });
  }
  return diagnostics;
}

export function countDiagnostics(diagnostics: Diagnostic[]): CountsByFile {
  const counts: CountsByFile = {};
  for (const diagnostic of diagnostics) counts[diagnostic.file] = (counts[diagnostic.file] ?? 0) + 1;
  return counts;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** 基準値 JSON を検査する。件数は正の整数だけにし、0 件の項目はファイルに残さない形を強制する。 */
export function parseBaseline(text: string): Baseline {
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch (error) {
    throw new Error(`基準値 JSON を読めません: ${error instanceof Error ? error.message : String(error)}`, {
      cause: error,
    });
  }
  if (!isObject(value)) throw new Error("基準値 JSON の先頭は object にしてください");
  const baseline: Baseline = {};
  for (const [option, filesValue] of Object.entries(value)) {
    if (!OPTION_PATTERN.test(option)) throw new Error(`基準値のオプション名が不正です: ${option}`);
    if (!isObject(filesValue)) throw new Error(`基準値 ${option} の値は object にしてください`);
    const files: CountsByFile = {};
    for (const [file, count] of Object.entries(filesValue)) {
      const normalizedFile = file.replaceAll("\\", "/");
      const isAbsolute = normalizedFile.startsWith("/") || /^[A-Za-z]:\//.test(normalizedFile);
      if (normalizedFile.trim() === "" || isAbsolute || normalizedFile.split("/").includes("..")) {
        throw new Error(`基準値 ${option} のファイルパスが不正です: ${file}`);
      }
      if (!Number.isInteger(count) || typeof count !== "number" || count <= 0) {
        throw new Error(`基準値 ${option} ${file} の件数は正の整数にしてください`);
      }
      files[normalizedFile] = count;
    }
    baseline[option] = files;
  }
  return baseline;
}

function compareStrings(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function sortedCounts(counts: CountsByFile): CountsByFile {
  return Object.fromEntries(Object.entries(counts).sort(([a], [b]) => compareStrings(a, b)));
}

/** オプション名とファイル名を安定した順に並べた JSON にする。 */
export function formatBaseline(baseline: Baseline): string {
  const sorted = Object.fromEntries(
    Object.entries(baseline)
      .sort(([a], [b]) => compareStrings(a, b))
      .map(([option, files]) => [option, sortedCounts(files)]),
  );
  return `${JSON.stringify(sorted, null, 2)}\n`;
}

/** 基準値と現在値を比べる。基準値に無いファイルは基準 0 件として悪化に数える。 */
export function compareBaseline(baseline: Baseline, current: Baseline): Comparison {
  const improvements: CountChange[] = [];
  const regressions: CountChange[] = [];
  for (const option of [...new Set([...Object.keys(baseline), ...Object.keys(current)])].sort()) {
    const baselineFiles = baseline[option] ?? {};
    const currentFiles = current[option] ?? {};
    for (const file of [...new Set([...Object.keys(baselineFiles), ...Object.keys(currentFiles)])].sort()) {
      const baselineCount = baselineFiles[file] ?? 0;
      const currentCount = currentFiles[file] ?? 0;
      const change = { option, file, baseline: baselineCount, current: currentCount };
      if (currentCount > baselineCount) regressions.push(change);
      if (currentCount < baselineCount) improvements.push(change);
    }
  }
  return { improvements, regressions };
}

/** 改善した項目だけを基準値に写す。新規ファイルや件数の増加は写さない。 */
export function applyImprovements(baseline: Baseline, current: Baseline): Baseline {
  const next: Baseline = {};
  for (const [option, baselineFiles] of Object.entries(baseline)) {
    const files: CountsByFile = {};
    for (const [file, baselineCount] of Object.entries(baselineFiles)) {
      const currentCount = current[option]?.[file] ?? 0;
      if (currentCount < baselineCount) {
        if (currentCount > 0) files[file] = currentCount;
      } else {
        files[file] = baselineCount;
      }
    }
    next[option] = files;
  }
  return next;
}

function measureOptions(options: string[], io: RatchetIo): { current?: Baseline; error?: string } {
  const current: Baseline = {};
  for (const option of options) {
    const result = io.check(option);
    let diagnostics: Diagnostic[];
    try {
      diagnostics = parseDiagnostics(result.stdout);
    } catch (error) {
      return { error: `${option}: ${error instanceof Error ? error.message : String(error)}` };
    }
    if (result.status !== 0 && result.status !== 1) {
      const detail = result.stderr.trim() === "" ? "stderr も空です" : result.stderr.trim().split(/\r?\n/).at(-1);
      return {
        error: `${option}: bun check が予期しない終了コード ${result.status ?? "不明"} で終わりました。${detail}`,
      };
    }
    if (result.status !== 0 && diagnostics.length === 0) {
      const detail = result.stderr.trim() === "" ? "stderr も空です" : result.stderr.trim().split(/\r?\n/).at(-1);
      return {
        error: `${option}: bun check が診断を出さずに失敗しました（終了コード ${result.status ?? "不明"}）。${detail}`,
      };
    }
    if (result.status === 0 && diagnostics.length > 0) {
      return { error: `${option}: bun check が診断を出しながら終了コード 0 でした。出力形式の変化として失敗にします` };
    }
    current[option] = countDiagnostics(diagnostics);
  }
  return { current };
}

function printChanges(label: string, changes: CountChange[], io: RatchetIo): void {
  for (const change of changes) {
    io.err(`${label}: ${change.option} ${change.file}: 基準値 ${change.baseline} 件、現在 ${change.current} 件`);
  }
}

function readBaseline(io: RatchetIo): Baseline | undefined {
  try {
    return parseBaseline(io.readBaseline());
  } catch (error) {
    io.err(error instanceof Error ? error.message : String(error));
    return undefined;
  }
}

function writeBaseline(io: RatchetIo, baseline: Baseline): boolean {
  try {
    io.writeBaseline(formatBaseline(baseline));
    return true;
  } catch (error) {
    io.err(`基準値を書けません: ${error instanceof Error ? error.message : String(error)}`);
    return false;
  }
}

function usage(io: RatchetIo): void {
  io.err("使い方: node scripts/type-ratchet.ts [--write] [--add=<compiler-option>]");
}

/** ラチェットを実行し、終了コードを返す。 */
export function runTypeRatchet(args: string[], io: RatchetIo): number {
  const write = args.includes("--write");
  const addArgument = args.find((arg) => arg.startsWith("--add="));
  const unknown = args.filter((arg) => arg !== "--write" && !arg.startsWith("--add="));
  if (
    unknown.length > 0 ||
    (write && addArgument !== undefined) ||
    args.filter((arg) => arg === "--write").length > 1
  ) {
    usage(io);
    return 2;
  }

  const baseline = readBaseline(io);
  if (baseline === undefined) return 1;

  if (addArgument !== undefined) {
    const option = addArgument.slice("--add=".length);
    if (!OPTION_PATTERN.test(option)) {
      io.err(`compiler option の名前が不正です: ${option}`);
      return 2;
    }
    if (Object.hasOwn(baseline, option)) {
      io.err(`${option} はすでに基準値にあります`);
      return 1;
    }
    const measured = measureOptions([option], io);
    if (measured.error !== undefined || measured.current === undefined) {
      io.err(measured.error ?? `${option} を計測できません`);
      return 1;
    }
    const counts = measured.current[option] ?? {};
    const total = Object.values(counts).reduce((sum, count) => sum + count, 0);
    if (total === 0) {
      io.err(`${option} のエラーは 0 件です。ラチェットではなく tsconfig で有効化してください`);
      return 1;
    }
    const next = { ...baseline, [option]: counts };
    if (!writeBaseline(io, next)) return 1;
    io.out(`${option} を基準値へ追加しました（${total} 件）`);
    return 0;
  }

  const options = Object.keys(baseline).sort();
  const measured = measureOptions(options, io);
  if (measured.error !== undefined || measured.current === undefined) {
    io.err(measured.error ?? "bun check を実行できません");
    return 1;
  }
  const comparison = compareBaseline(baseline, measured.current);

  if (!write) {
    printChanges("悪化", comparison.regressions, io);
    printChanges("改善（未更新）", comparison.improvements, io);
    if (comparison.regressions.length === 0 && comparison.improvements.length === 0) {
      io.out("type-ratchet: 基準値どおりです");
      return 0;
    }
    if (comparison.improvements.length > 0)
      io.err("改善分は mise run client:type-ratchet-update で基準値へ反映してください");
    return 1;
  }

  if (comparison.improvements.length > 0) {
    if (!writeBaseline(io, applyImprovements(baseline, measured.current))) return 1;
    printChanges("改善を基準値へ反映", comparison.improvements, io);
  }
  printChanges("悪化（基準値へは反映しません）", comparison.regressions, io);
  if (comparison.regressions.length > 0) return 1;
  io.out(
    comparison.improvements.length > 0
      ? "type-ratchet: 基準値を更新しました"
      : "type-ratchet: 基準値に変更はありません",
  );
  return 0;
}

function createIo(clientDir: string): RatchetIo {
  const baselinePath = join(clientDir, "type-ratchet-baseline.json");
  return {
    readBaseline: () => readFileSync(baselinePath, "utf8"),
    writeBaseline: (text) => writeFileSync(baselinePath, text),
    check: (option) => {
      const result = spawnSync("bun", ["check", `--${option}`], { cwd: clientDir, encoding: "utf8" });
      return { status: result.status, stdout: result.stdout ?? "", stderr: result.stderr ?? "" };
    },
    out: (message) => console.log(message),
    err: (message) => console.error(message),
  };
}

if (import.meta.main) {
  process.exitCode = runTypeRatchet(process.argv.slice(2), createIo(join(import.meta.dirname, "..")));
}
