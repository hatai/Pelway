# verify-pelway の機能一覧

デモの流れ（チャットで人と AI が案を出し合い、決まった内容からメールの下書きを作る）を、利用者が触る単位に分けたもの。状態は `verify-run` で実際に動かした結果で書く。

| 機能 | ファイル | `verify-run --feature` | 状態 |
|---|---|---|---|
| チャンネルの投稿と取得 | `channel-messages.md` | `decision`（起動の合図・段 2・3・5・9） | 動く |
| ハルへのメンションから A/B 案 | `mention-proposal.md` | `decision`（段 2・4・5） | 動く（LLM は台本モード） |
| 判断待ちと決定 | `decision.md` | `decision`（段 4・6〜9） | 動く |
| 決定後のメール下書き | `mail-draft.md` | `mail-draft`（段 10） | 失敗（機能なし） |
| クライアント画面からの操作 | `client-ui.md` | なし（Electron 開発版の起動確認は手動。run の合否に入れない） | 失敗（依存機能が未完成） |

機能を足すときは、同じ 4 つの見出し（`Sub-features` / `How to get to it (user POV)` / `Driving it with verify-run` / `Gotchas`）でファイルを作り、この表に 1 行足す。`verify-run` で動かせない機能は「失敗（理由）」と書き、確かめ方を弱めて通さない。
