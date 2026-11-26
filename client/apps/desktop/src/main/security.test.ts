import { describe, expect, test } from "bun:test";
import { denyAllPermissions, hardenWebContents, isTrustedSender, isTrustedUrl, secureWebPreferences } from "./security";

const ORIGIN = "pelway://app";

describe("secureWebPreferences", () => {
  test("Node を画面に出さず、隔離と sandbox を有効にする", () => {
    const prefs = secureWebPreferences("/app/out/preload/index.cjs");
    expect(prefs.contextIsolation).toBe(true);
    expect(prefs.sandbox).toBe(true);
    expect(prefs.nodeIntegration).toBe(false);
    expect(prefs.nodeIntegrationInWorker).toBe(false);
    expect(prefs.webSecurity).toBe(true);
    expect(prefs.allowRunningInsecureContent).toBe(false);
    expect(prefs.webviewTag).toBe(false);
    expect(prefs.spellcheck).toBe(false);
    expect(prefs.preload).toBe("/app/out/preload/index.cjs");
  });
});

describe("isTrustedUrl", () => {
  test("同梱の資産のオリジンだけを信頼する", () => {
    expect(isTrustedUrl("pelway://app/index.html", ORIGIN)).toBe(true);
    expect(isTrustedUrl("https://example.com/", ORIGIN)).toBe(false);
    expect(isTrustedUrl("pelway://other/index.html", ORIGIN)).toBe(false);
    expect(isTrustedUrl("file:///etc/passwd", ORIGIN)).toBe(false);
    expect(isTrustedUrl("not a url", ORIGIN)).toBe(false);
  });
});

describe("isTrustedSender", () => {
  test("同梱の資産のフレームから来た IPC だけを受ける", () => {
    expect(isTrustedSender({ senderFrame: { url: "pelway://app/index.html" } }, ORIGIN)).toBe(true);
    expect(isTrustedSender({ senderFrame: { url: "https://evil.example/" } }, ORIGIN)).toBe(false);
    expect(isTrustedSender({ senderFrame: null }, ORIGIN)).toBe(false);
  });
});

type Listener = (event: { url?: string; preventDefault: () => void }, ...args: unknown[]) => void;

function fakeContents() {
  const listeners = new Map<string, Listener>();
  let openHandler: ((details: { url: string }) => { action: string }) | undefined;
  return {
    on(name: string, listener: Listener) {
      listeners.set(name, listener);
    },
    setWindowOpenHandler(handler: (details: { url: string }) => { action: "deny" }) {
      openHandler = handler;
    },
    emit(name: string, ...args: unknown[]) {
      let prevented = false;
      listeners.get(name)?.({ preventDefault: () => (prevented = true) }, ...args);
      return prevented;
    },
    // Electron 45 の遷移のイベントは、1 番目の引数 details の url が正。2 番目の url は非推奨。
    navigate(name: string, url: string, deprecatedUrl = "pelway://app/index.html") {
      let prevented = false;
      listeners.get(name)?.({ url, preventDefault: () => (prevented = true) }, deprecatedUrl);
      return prevented;
    },
    open(url: string) {
      return openHandler?.({ url });
    },
  };
}

// 偽物は試験に要るメソッドだけを持つので、型は本物の WebContents に見せかける。
const asContents = (fake: ReturnType<typeof fakeContents>) =>
  fake as unknown as Parameters<typeof hardenWebContents>[0];

describe("hardenWebContents", () => {
  test("同梱の資産以外への遷移を止める", () => {
    const contents = fakeContents();
    hardenWebContents(asContents(contents), ORIGIN);
    expect(contents.navigate("will-navigate", "https://example.com/")).toBe(true);
    expect(contents.navigate("will-redirect", "https://example.com/")).toBe(true);
    expect(contents.navigate("will-navigate", "pelway://app/index.html", "https://example.com/")).toBe(false);
  });

  test("子フレームの同梱の資産以外への遷移も止める", () => {
    const contents = fakeContents();
    hardenWebContents(asContents(contents), ORIGIN);
    expect(contents.navigate("will-frame-navigate", "https://example.com/")).toBe(true);
    expect(contents.navigate("will-frame-navigate", "pelway://app/index.html")).toBe(false);
  });

  test("新しい窓を開かせない", () => {
    const contents = fakeContents();
    hardenWebContents(asContents(contents), ORIGIN);
    expect(contents.open("https://example.com/")).toEqual({ action: "deny" });
    expect(contents.open("pelway://app/index.html")).toEqual({
      action: "deny",
    });
  });

  test("webview を作らせない", () => {
    const contents = fakeContents();
    hardenWebContents(asContents(contents), ORIGIN);
    expect(contents.emit("will-attach-webview")).toBe(true);
  });
});

describe("denyAllPermissions", () => {
  test("権限の要求と確認をすべて拒む", () => {
    let request: ((wc: unknown, permission: string, callback: (granted: boolean) => void) => void) | undefined;
    let check: ((wc: unknown, permission: string) => boolean) | undefined;
    const fakeSession = {
      setPermissionRequestHandler(handler: typeof request) {
        request = handler;
      },
      setPermissionCheckHandler(handler: typeof check) {
        check = handler;
      },
    };
    denyAllPermissions(fakeSession as unknown as Parameters<typeof denyAllPermissions>[0]);
    let granted: boolean | undefined;
    request?.(null, "notifications", (value) => (granted = value));
    expect(granted).toBe(false);
    expect(check?.(null, "media")).toBe(false);
  });
});
