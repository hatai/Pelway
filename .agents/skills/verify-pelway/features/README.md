# verify-pelway の機能一覧

デモの流れ（チャットで人と AI が案を出し合い、決まった内容からメールの下書きを作る）を、利用者が触る単位に分けたもの。状態は `verify-run` で実際に動かした結果で書く。

| 機能 | ファイル | `verify-run --feature` | 状態 |
|---|---|---|---|
| チャンネルの投稿と取得 | `channel-messages.md` | `decision`（段 1・2・4） | 動く |
| ハルへのメンションから A/B 案 | `mention-proposal.md` | `decision`（段 2〜4） | 動く（LLM は台本モード） |
| 判断待ちと決定 | `decision.md` | `decision`（段 3・5・6） | 動く |
| 決定後のメール下書き | `mail-draft.md` | `mail-draft`（段 7） | 失敗（main に下書き生成が無い） |
| クライアント画面からの操作 | `client-ui.md` | なし | 失敗（起動できるクライアントが無く、通信が未結線） |

機能を足すときは、同じ 4 つの見出し（`Sub-features` / `How to get to it (user POV)` / `Driving it with verify-run` / `Gotchas`）でファイルを作り、この表に 1 行足す。`verify-run` で動かせない機能は「失敗（理由）」と書き、確かめ方を弱めて通さない。
