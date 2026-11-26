// 窓と WebContents のセキュリティの既定（ADR 0014「Electron のセキュリティの既定」）。
// electron は型だけを読む。実行時には渡された物だけを使うので、試験で偽物を渡せる。
import type { Session, WebContents, WebPreferences } from "electron";

/** 同梱の資産を出す独自のスキーム。file:// を使わない（チェックリスト #18）。 */
export const APP_SCHEME = "pelway";
/** 同梱の資産のオリジン。 */
export const APP_ORIGIN = `${APP_SCHEME}://app`;

/** 窓の webPreferences。チェックリストの #2・#3・#4・#6・#8・#9・#11 を満たす。 */
export function secureWebPreferences(preload: string) {
  return {
    preload,
    contextIsolation: true,
    sandbox: true,
    nodeIntegration: false,
    nodeIntegrationInWorker: false,
    nodeIntegrationInSubFrames: false,
    webSecurity: true,
    allowRunningInsecureContent: false,
    experimentalFeatures: false,
    webviewTag: false,
  } as const satisfies WebPreferences;
}

/**
 * url の「スキーム://ホスト[:ポート]」。読めない url は null。
 * Node の URL は独自のスキーム（pelway:）の origin を "null" にするので、origin を使わずに組み立てる。
 */
export function originOf(url: string): string | null {
  try {
    const parsed = new URL(url);
    return parsed.host === "" ? null : `${parsed.protocol}//${parsed.host}`;
  } catch {
    return null;
  }
}

/** url のオリジンが信頼するオリジンと同じとき true。読めない url は信頼しない。 */
export function isTrustedUrl(url: string, trustedOrigin: string): boolean {
  return originOf(url) === trustedOrigin;
}

export interface IpcEventLike {
  senderFrame: { url: string } | null;
}

/** IPC の送り元のフレームが同梱の資産のときだけ true（チェックリスト #17）。 */
export function isTrustedSender(event: IpcEventLike, trustedOrigin: string): boolean {
  const frame = event.senderFrame;
  return frame !== null && isTrustedUrl(frame.url, trustedOrigin);
}

/** 遷移・新しい窓・webview を止める（チェックリスト #12・#13・#14）。 */
export function hardenWebContents(
  contents: Pick<WebContents, "on" | "setWindowOpenHandler">,
  trustedOrigin: string,
): void {
  const blockUntrusted = (event: { preventDefault(): void }, url: string) => {
    if (!isTrustedUrl(url, trustedOrigin)) event.preventDefault();
  };
  contents.on("will-navigate", blockUntrusted);
  contents.on("will-redirect", blockUntrusted);
  contents.on("will-attach-webview", (event) => event.preventDefault());
  contents.setWindowOpenHandler(() => ({ action: "deny" }));
}

/** 権限の要求と確認をすべて拒む（チェックリスト #5）。使う権限ができたら、ここで個別に許す。 */
export function denyAllPermissions(
  session: Pick<Session, "setPermissionRequestHandler" | "setPermissionCheckHandler">,
): void {
  session.setPermissionRequestHandler((_webContents, _permission, callback) => callback(false));
  session.setPermissionCheckHandler(() => false);
}
