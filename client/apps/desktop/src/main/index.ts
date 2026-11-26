// main の入口。窓を 1 つ開き、同梱の画面を独自のスキームで読む（ADR 0014）。
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { app, BrowserWindow, net, protocol, session } from "electron";
import { resolveAppAsset } from "./app-protocol";
import {
  APP_ORIGIN,
  APP_SCHEME,
  denyAllPermissions,
  disableSpellChecker,
  hardenWebContents,
  originOf,
  secureWebPreferences,
} from "./security";

// 開発版では electron-vite の開発サーバーが画面を出す。
const devServerUrl = app.isPackaged ? undefined : process.env.ELECTRON_RENDERER_URL;
const trustedOrigin = (devServerUrl && originOf(devServerUrl)) || APP_ORIGIN;
const rendererDir = join(import.meta.dirname, "../renderer");

protocol.registerSchemesAsPrivileged([
  {
    scheme: APP_SCHEME,
    privileges: { standard: true, secure: true, supportFetchAPI: true },
  },
]);

app.on("web-contents-created", (_event, contents) => hardenWebContents(contents, trustedOrigin));

function createWindow(): void {
  const window = new BrowserWindow({
    width: 1024,
    height: 720,
    webPreferences: secureWebPreferences(join(import.meta.dirname, "../preload/index.cjs")),
  });
  void window.loadURL(devServerUrl ?? `${APP_ORIGIN}/index.html`);
}

app.whenReady().then(() => {
  disableSpellChecker(session.defaultSession);
  denyAllPermissions(session.defaultSession);
  protocol.handle(APP_SCHEME, (request) => {
    const file = resolveAppAsset(rendererDir, request.url);
    if (file === null) return new Response(null, { status: 404 });
    return net.fetch(pathToFileURL(file).toString());
  });
  createWindow();
});

app.on("window-all-closed", () => app.quit());
