// pelway-sandbox: T3 実装スレッド（worktree セッション）の opencode 用
// sandbox プラグイン（Issue #151）。
//
// 役割は二つ:
//   1. shell フックで全コマンドを sandbox-shell（bwrap）経由にし、
//      書き込みをセッションの worktree と必要なキャッシュに限定する。
//      ネットワークは遮断しない（Issue #151 で hatai が許可リスト
//      非採用を指示したため）。
//   2. ファイルツール（write/edit/read 等）が worktree 外や秘匿パスに
//      アクセスするのを拒否する。シェル経由でない書き込みは bwrap を
//      通らないため、この層が必要。
//
// 適用範囲: ~/.t3/worktrees/ 配下のロケーションのみ。人間の通常セッション
// （本体チェックアウト等）には影響しない。
//
// 注意: このプラグインはプロセス内フックであり、OS 境界は sandbox-shell
// 側の bwrap が担う。プラグイン自身の無効化を防ぐため、sandbox-shell は
// worktree 内の opencode.json[c]/.opencode/ をマスクしている。

import os from "node:os"
import path from "node:path"

const HOME = os.homedir()
const WORKTREE_PREFIX = `${HOME}/.t3/worktrees/`
const SBX_SHELL = `${HOME}/.config/opencode/plugins/pelway-sandbox/sandbox-shell`

// ファイルツールの read を拒否するパス（シェル側は sandbox-shell の
// tmpfs マスクが同等の役割を持つ）。
const READ_DENY = [
  "/mnt",
  `${HOME}/.ssh`,
  `${HOME}/.gnupg`,
  `${HOME}/.aws`,
  `${HOME}/.docker`,
  `${HOME}/.kube`,
  `${HOME}/.config/gh`,
  `${HOME}/.config/opencode`,
  `${HOME}/.config/devin`,
  `${HOME}/.config/fnox`,
  `${HOME}/.config/pelway-agent`,
  `${HOME}/.local/share/devin`,
  `${HOME}/.local/bin`,
  `${HOME}/.t3/userdata`,
]

// write 系ツールが書き込んでよいのはセッションの worktree 内だけ。
// その中でも sandbox 無効化や git 構成の改変に使えるパスは拒否する。
const WRITE_TOOLS = new Set([
  "write",
  "edit",
  "patch",
  "apply_patch",
  "multiedit",
  "notebook",
])
const READ_TOOLS = new Set(["read", "glob", "grep", "list"])
const WORKTREE_DENY = /(^|\/)\.git$|(^|\/)\.git\/|(^|\/)\.opencode(\/|$)|(^|\/)\.devin(\/|$)|(^|\/)opencode\.jsonc?$/
const ENV_FILE = /(^|\/)\.env(\..*)?$/

function resolvePath(base, p) {
  if (typeof p !== "string" || p.length === 0) return null
  return path.normalize(p.startsWith("/") ? p : path.resolve(base, p))
}

function collectPaths(input) {
  const out = []
  const push = (v) => {
    if (typeof v === "string") out.push(v)
  }
  for (const k of [
    "path",
    "filePath",
    "file_path",
    "absolute_path",
    "notebook_path",
    "target_file",
    "directory",
  ]) {
    push(input?.[k])
  }
  if (Array.isArray(input?.paths)) input.paths.forEach(push)
  for (const k of ["edits", "replacements", "files"]) {
    if (Array.isArray(input?.[k])) {
      for (const e of input[k]) {
        if (e && typeof e === "object") {
          push(e.filePath ?? e.file_path ?? e.path ?? e.absolute_path)
        }
      }
    }
  }
  return out
}

function under(child, parent) {
  const rel = path.relative(parent, child)
  return rel === "" || (!rel.startsWith("..") && !path.isAbsolute(rel))
}

function deny(reason) {
  throw new Error(`pelway-sandbox: ${reason}`)
}

export default {
  id: "pelway-sandbox",
  async setup(ctx) {
    const dir = ctx?.location?.directory
    if (typeof dir !== "string" || !dir.startsWith(WORKTREE_PREFIX)) return
    const root = path.normalize(dir)

    await ctx.shell.hook("create.before", (event) => {
      event.shell = SBX_SHELL
      if (event.env && typeof event.env === "object") {
        event.env.MISE_RUST_MR_BOXINGTON = "0"
        event.env.PELWAY_SBX_ROOT = root
        for (const k of Object.keys(event.env)) {
          if (k.startsWith("T3_ACP_MCP_") || k.startsWith("T3CODE_")) {
            delete event.env[k]
          }
        }
      }
    })

    await ctx.tool.hook("execute.before", (event) => {
      const tool = event?.tool
      const input = event?.input ?? {}
      if (WRITE_TOOLS.has(tool)) {
        for (const raw of collectPaths(input)) {
          const p = resolvePath(root, raw)
          if (p === null) continue
          if (!under(p, root)) {
            deny(`${tool}: worktree 外への書き込みは禁止: ${raw}`)
          }
          if (WORKTREE_DENY.test(p) || WORKTREE_DENY.test(path.normalize(raw))) {
            deny(`${tool}: sandbox 設定・git 構成の変更は禁止: ${raw}`)
          }
        }
        return
      }
      if (READ_TOOLS.has(tool)) {
        for (const raw of collectPaths(input)) {
          const p = resolvePath(root, raw)
          if (p === null) continue
          for (const d of READ_DENY) {
            if (under(p, d)) deny(`${tool}: 秘匿パスの読み取りは禁止: ${raw}`)
          }
          if (ENV_FILE.test(p)) {
            deny(`${tool}: .env ファイルの読み取りは禁止: ${raw}`)
          }
        }
      }
    })
  },
}
