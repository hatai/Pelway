// pelway-agent-notify (OpenCode v2 plugin shape) — VERSION 3.5
// Loader (opencode v2.2.6) requires: export default { id: string, setup: fn } or { id, effect: fn }.
// setup(api) receives api.event.subscribe({ types }) (async iterable of v2 events) and api.location.
// v2 events look like { id, type, created, location?: { directory }, data: { sessionID, ... } }.
// Behaviour kept from v1: map lifecycle events -> notify.sh --event <finished|failed|waiting_permission|waiting_input>,
// detached spawn, errors swallowed; notify.sh decides dry-run/send and writes agent-notify.log.
//
// v3.5: pass --session-dir (the session's own working directory) so notify.sh v3.4 attributes the
// event to the session's worktree instead of skipping with session-dir-unknown.
// Directory lookup order (measured OpenCode 2.2.6 shapes; checked on every event):
//   (1) data.location.directory  — current session.created shape
//   (2) data.info.directory      — legacy shape
//   (3) event.location.directory — envelope
// session.created also stores sid -> dir in a map; later events fall back to it because
// session.execution.* data only carries { sessionID }.
// Session id: data.sessionID or (data.info ?? data).id. Child session: data.parentID or (data.info ?? data).parentID.
// No ownDir/process.cwd() fallback for --session-dir: when the directory is unknown the flag is
// still passed with an empty value, and notify.sh skips (session-dir-unknown) rather than risk
// notifying from the wrong worktree. --cwd keeps receiving this plugin's own directory as before.
import { spawn } from "node:child_process"
import { homedir } from "node:os"

const NOTIFY = `${homedir()}/.config/pelway-agent-notify/notify.sh`

// v2 event type -> pelway event name
const MAP: Record<string, string> = {
  "session.execution.succeeded": "finished",
  "session.execution.failed": "failed",
  "permission.asked": "waiting_permission",
  "form.created": "waiting_input",
}
// v1 names, kept only as a fallback in case a v1-compat bridge forwards them
const LEGACY: Record<string, string> = {
  "session.idle": "finished",
  "session.error": "failed",
  "permission.asked": "waiting_permission",
  "question.asked": "waiting_input",
}

// first non-empty string among the candidates
function pick(...vals: unknown[]): string {
  for (const v of vals) if (typeof v === "string" && v) return v
  return ""
}

// session's working directory: (1) data.location.directory (2) data.info.directory (3) event.location.directory
function sessionDir(event: any, data: any): string {
  return pick(data?.location?.directory, data?.info?.directory, event?.location?.directory)
}

function notify(ev: string, cwd: string, sid: string, sdir: string) {
  try {
    spawn(
      "bash",
      [
        NOTIFY,
        "--tool",
        "opencode",
        "--event",
        ev,
        "--cwd",
        cwd,
        "--session",
        sid,
        // flag is always passed; notify.sh treats an empty value as session-dir-unknown and skips
        "--session-dir",
        sdir,
      ],
      {
        detached: true,
        stdio: "ignore",
      },
    )
      .on("error", () => {})
      .unref()
  } catch {}
}

export default {
  id: "pelway.agent-notify",
  setup(api: any) {
    if (process.env.PELWAY_AGENT_NOTIFY === "off") return
    const fallbackDir: string = api?.location?.directory ?? process.cwd()
    const children = new Set<string>()
    const dirs = new Map<string, string>() // sid -> session working directory, learned from session.created
    const ac = new AbortController()
    const types = [...new Set([...Object.keys(MAP), "session.created"])]
    ;(async () => {
      const stream = api?.event?.subscribe?.({ types }, { signal: ac.signal })
      if (!stream || typeof stream[Symbol.asyncIterator] !== "function") return
      for await (const event of stream as AsyncIterable<any>) {
        try {
          const data = event?.data ?? event?.properties ?? {}
          const info = data.info ?? data
          const sid: string = pick(data.sessionID, info?.id)
          const dir: string = sessionDir(event, data)
          if (event?.type === "session.created") {
            // remember the session's directory for its later execution events
            if (sid && dir) dirs.set(sid, dir)
            // skip sub-agent (child) sessions so only root completions notify
            const parent: unknown = data.parentID ?? info?.parentID
            if (parent && sid) children.add(sid)
            continue
          }
          if (sid && dir) dirs.set(sid, dir)
          if (sid && children.has(sid)) continue
          const ev = MAP[event?.type] ?? LEGACY[event?.type]
          if (!ev) continue
          const cwd: string = event?.location?.directory ?? fallbackDir
          notify(ev, cwd, sid, dir || dirs.get(sid) || "")
        } catch {}
      }
    })().catch(() => {})
    return () => ac.abort()
  },
}
