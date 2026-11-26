import { describe, expect, test } from "bun:test";
import { createBridge } from "./bridge";

type Handler = (event: unknown, ...args: unknown[]) => void;

function fakeIpc() {
  const handlers = new Map<string, Handler>();
  const invoked: unknown[][] = [];
  return {
    invoked,
    handlers,
    invoke(channel: string, ...args: unknown[]) {
      invoked.push([channel, ...args]);
      return Promise.resolve("ok");
    },
    on(channel: string, handler: Handler) {
      handlers.set(channel, handler);
    },
    removeListener(channel: string, handler: Handler) {
      if (handlers.get(channel) === handler) handlers.delete(channel);
    },
  };
}

const allow = (declared: string) => (channel: unknown) => channel === declared;

describe("createBridge", () => {
  test("宣言したチャンネルの invoke だけを main に送る", async () => {
    const ipc = fakeIpc();
    const bridge = createBridge(ipc, allow("app:ping"), allow("app:tick"));
    await expect(bridge.invoke("app:ping", 1)).resolves.toBe("ok");
    await expect(bridge.invoke("octane:shell.openExternal", "https://evil.example/")).rejects.toThrow();
    expect(ipc.invoked).toEqual([["app:ping", 1]]);
  });

  test("宣言していないチャンネルの on を拒む", () => {
    const ipc = fakeIpc();
    const bridge = createBridge(ipc, allow("app:ping"), allow("app:tick"));
    expect(() => bridge.on("octane:window.state", () => {})).toThrow();
    expect(ipc.handlers.size).toBe(0);
  });

  test("on の listener には値だけを渡し、IpcRendererEvent を渡さない", () => {
    const ipc = fakeIpc();
    const bridge = createBridge(ipc, allow("app:ping"), allow("app:tick"));
    const received: unknown[][] = [];
    const off = bridge.on("app:tick", (...args) => received.push(args));
    const event = { sender: { send() {} } };
    ipc.handlers.get("app:tick")?.(event, 1, "two");
    expect(received).toEqual([[1, "two"]]);
    off();
    expect(ipc.handlers.size).toBe(0);
  });
});
