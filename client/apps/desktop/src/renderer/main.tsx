// 画面の入口。
import { createRoot } from "octane";
import { App } from "./App";

const root = document.getElementById("root");
if (root === null) throw new Error("#root is missing in index.html");
createRoot(root).render(App);
