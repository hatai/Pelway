// preload の入口。宣言したチャンネルだけを通す橋を、@octanejs/electron のフックが読む
// グローバル（既定のキー）に出す（ADR 0014）。
import { BRIDGE_GLOBAL, isEventChannel, isInvokeChannel } from "@pelway/contracts";
import { contextBridge, ipcRenderer } from "electron";
import { createBridge } from "./bridge";

contextBridge.exposeInMainWorld(BRIDGE_GLOBAL, createBridge(ipcRenderer, isInvokeChannel, isEventChannel));
