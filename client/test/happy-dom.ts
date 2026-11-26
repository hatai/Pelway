// bun test の前に読み込み、DOM（happy-dom）を globalThis に入れる。
import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();
