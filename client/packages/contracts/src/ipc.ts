// IPC のチャンネルの一覧。preload はここに宣言したチャンネルだけを通す（ADR 0014）。
// チャンネルを足すときは、ここ・main のハンドラ・preload・画面の模擬を同じ変更で直す。

/** 画面から main へ要求を送り、応答を待つチャンネル（ipcRenderer.invoke）。 */
export const IPC_INVOKE_CHANNELS = [] as const satisfies readonly string[];

/** main から画面へ値を送るチャンネル（webContents.send）。 */
export const IPC_EVENT_CHANNELS = [] as const satisfies readonly string[];

export type IpcInvokeChannel = (typeof IPC_INVOKE_CHANNELS)[number];
export type IpcEventChannel = (typeof IPC_EVENT_CHANNELS)[number];

export function isInvokeChannel(channel: unknown): channel is IpcInvokeChannel {
  return (IPC_INVOKE_CHANNELS as readonly unknown[]).includes(channel);
}

export function isEventChannel(channel: unknown): channel is IpcEventChannel {
  return (IPC_EVENT_CHANNELS as readonly unknown[]).includes(channel);
}

/**
 * preload が橋を出すグローバルのキー。@octanejs/electron の画面の側のフックが既定で読むキーと同じ
 * （違うキーにするときは、画面の側で setElectronBridgeKey を呼ぶ）。
 */
export const BRIDGE_GLOBAL = "__OCTANE_ELECTRON__";
