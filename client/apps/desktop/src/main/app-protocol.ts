// 独自のスキーム（pelway://app/）の要求を、同梱の資産のファイルに対応させる。
import { join, normalize, sep } from "node:path";
import { APP_ORIGIN, originOf } from "./security";

/**
 * 要求の url を rendererDir の中のファイルの絶対パスにする。
 * 別のホストの要求と、rendererDir の外を指す要求には null を返す。
 */
export function resolveAppAsset(rendererDir: string, requestUrl: string): string | null {
  let url: URL;
  try {
    url = new URL(requestUrl);
  } catch {
    return null;
  }
  if (originOf(requestUrl) !== APP_ORIGIN) return null;
  let pathname: string;
  try {
    pathname = decodeURIComponent(url.pathname);
  } catch {
    return null;
  }
  const relative = pathname === "/" ? "index.html" : pathname.replace(/^\/+/, "");
  const root = normalize(rendererDir);
  const resolved = normalize(join(root, relative));
  return resolved.startsWith(root + sep) ? resolved : null;
}
