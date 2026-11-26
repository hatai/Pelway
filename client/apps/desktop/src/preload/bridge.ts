// 画面に出す IPC の橋。@octanejs/electron のフック（useInvoke・useIpcEvent など）が読む
// invoke/on の形にそろえるが、通すのは @pelway/contracts で宣言したチャンネルだけにする
// （ADR 0014。チェックリスト #20）。

type Listener = (...args: unknown[]) => void;
type IpcHandler = (event: unknown, ...args: unknown[]) => void;

export interface IpcRendererLike {
  invoke(channel: string, ...args: unknown[]): Promise<unknown>;
  on(channel: string, handler: IpcHandler): void;
  removeListener(channel: string, handler: IpcHandler): void;
}

export interface PelwayBridge {
  invoke<T = unknown>(channel: string, ...args: unknown[]): Promise<T>;
  on(channel: string, listener: Listener): () => void;
}

export function createBridge(
  ipc: IpcRendererLike,
  isInvokeChannel: (channel: unknown) => boolean,
  isEventChannel: (channel: unknown) => boolean,
): PelwayBridge {
  return {
    invoke<T>(channel: string, ...args: unknown[]): Promise<T> {
      if (!isInvokeChannel(channel)) {
        return Promise.reject(new Error(`IPC channel is not declared: ${String(channel)}`));
      }
      return ipc.invoke(channel, ...args) as Promise<T>;
    },
    on(channel: string, listener: Listener): () => void {
      if (!isEventChannel(channel)) {
        throw new Error(`IPC channel is not declared: ${String(channel)}`);
      }
      // IpcRendererEvent（event.sender を含む）は画面に渡さない。
      const handler: IpcHandler = (_event, ...args) => listener(...args);
      ipc.on(channel, handler);
      return () => ipc.removeListener(channel, handler);
    },
  };
}
