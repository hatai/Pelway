// pelway-sandbox: T3 実装スレッド（worktree セッション）用 sandbox プラグイン
//（Issue #151）。~/.t3/worktrees/ 配下のみ適用。
//   1. shell フックで全コマンドを sandbox-shell（bwrap）経由にする。
//   2. ファイルツールの worktree 外・秘匿パスへのアクセスを拒否する
//      （bwrap を通らない経路）。検査は実体パスで行い symlink 越しの脱走も防ぐ。

import fs from "node:fs"
import os from "node:os"
import path from "node:path"

const HOME = os.homedir()
const WORKTREE_PREFIX = `${HOME}/.t3/worktrees/`
const SBX_SHELL = `${HOME}/.config/opencode/plugins/pelway-sandbox/sandbox-shell`

// ファイルツールの read を拒否するパス（シェル側は tmpfs マスクが担う）。
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

// 実体パスを返す。末尾が存在しないときは最深の既存祖先まで解決し残りを継ぐ。
function realPath(p) {
  try {
    return fs.realpathSync(p)
  } catch {
    const parent = path.dirname(p)
    if (parent === p) return p
    return path.join(realPath(parent), path.basename(p))
  }
}

function collectPaths(input) {
  const out = []
  const push = (v) => {
    if (typeof v === "string") out.push(v)
  }
  for (const k of ["path", "filePath", "file_path", "absolute_path", "notebook_path", "target_file", "directory"]) push(input?.[k])
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
    const realRoot = realPath(root)
    // deny 対象自体が symlink の環境（~/.aws → /mnt/c/... 等）では、
    // 字句パスと実体パスの両方を拒否リストに入れて検査する。
    const readDeny = new Set(READ_DENY.concat(READ_DENY.map(realPath)))

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
          const rp = realPath(p)
          if (!under(p, root) || !under(rp, realRoot)) {
            deny(`${tool}: worktree 外への書き込みは禁止: ${raw}`)
          }
          if (
            WORKTREE_DENY.test(p) ||
            WORKTREE_DENY.test(rp) ||
            WORKTREE_DENY.test(path.normalize(raw))
          ) {
            deny(`${tool}: sandbox 設定・git 構成の変更は禁止: ${raw}`)
          }
        }
        return
      }
      if (READ_TOOLS.has(tool)) {
        for (const raw of collectPaths(input)) {
          const p = resolvePath(root, raw)
          if (p === null) continue
          const rp = realPath(p)
          for (const d of readDeny) {
            if (under(p, d) || under(rp, d)) {
              deny(`${tool}: 秘匿パスの読み取りは禁止: ${raw}`)
            }
          }
          if (ENV_FILE.test(p) || ENV_FILE.test(rp)) {
            deny(`${tool}: .env ファイルの読み取りは禁止: ${raw}`)
          }
        }
      }
    })
  },
}
