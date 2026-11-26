// 窓と WebContents のセキュリティの既定（ADR 0014「Electron のセキュリティの既定」）。
// electron は型だけを読む。実行時には渡された物だけを使うので、試験で偽物を渡せる。
import type { Session, WebContents, WebPreferences } from "electron";

/** 同梱の資産を出す独自のスキーム。file:// を使わない（チェックリスト #18）。 */
export const APP_SCHEME = "pelway";
/** 同梱の資産のオリジン。 */
export const APP_ORIGIN = `${APP_SCHEME}://app`;

/** 窓の webPreferences。チェックリストの #2・#3・#4・#6・#8・#9・#11 を満たし、綴りの検査を切る。 */
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
    // 綴りの検査は使わない。有効だと辞書を外から取りに行きうる。
    spellcheck: false,
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

/** 遷移（子フレームを含む）・新しい窓・webview を止める（チェックリスト #12・#13・#14）。 */
export function hardenWebContents(
  contents: Pick<WebContents, "on" | "setWindowOpenHandler">,
  trustedOrigin: string,
): void {
  // 遷移先は 1 番目の引数（details）の url で読む。2 番目の url 引数は Electron で非推奨。
  const blockUntrusted = (details: { url: string; preventDefault(): void }) => {
    if (!isTrustedUrl(details.url, trustedOrigin)) details.preventDefault();
  };
  contents.on("will-navigate", blockUntrusted);
  contents.on("will-redirect", blockUntrusted);
  contents.on("will-frame-navigate", blockUntrusted);
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

/**
 * 綴りの検査の辞書の一覧を空にする。既定のセッションは作られた時点で辞書を読み始め、
 * 手元に無ければ外（Google の配信サーバー）から取ってくるので、窓を開く前に呼ぶ。
 * webPreferences の spellcheck: false は画面の側の設定で、この取得は止めない。
 */
export function disableSpellChecker(session: Pick<Session, "setSpellCheckerLanguages">): void {
  session.setSpellCheckerLanguages([]);
}
