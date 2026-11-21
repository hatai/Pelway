# チャンネルの投稿と取得

## Sub-features

- チャンネル一覧（メンバーと種別つき）: `GET /v1/workspaces/{workspace_id}/channels?actor=<member_id>`
- 投稿: `POST /v1/channels/{channel_id}/messages`。本文は `{"client_msg_id", "body", "thread_root"}`。操作者は合成ヘッダー `x-pelway-member-id`・`x-pelway-member-kind: human`
- 取得と差分: `GET /v1/channels/{channel_id}/messages?actor=<member_id>&after_seq=<seq>`（`seq` より後だけ）

## How to get to it (user POV)

田中（`m-tanaka`）が #案件B（`ch-demo-anken-b`）を開き、過去の発言 3 件を読んでから発言する。発言は `seq` の順に並び、あとから来た発言は `after_seq` で差分だけ読む。

## Driving it with verify-run

`verify-run --feature decision` の段 1（一覧で seed を確かめる）、段 2（投稿）、段 3（同じ `client_msg_id` の再送が同じ `message_id`）、段 5（`after_seq` で読み戻し、依頼が 1 件だけ）、段 9（決定後に `after_seq=0` で取り直す）。応答は `steps/NN-channels.resp`・`steps/NN-mention.resp`・`steps/NN-mention-resend.resp`・`steps/NN-messages-after-mention.resp`・`steps/NN-final-messages.resp`。テストは `cargo test -p pelway-http --test channels`。

## Gotchas

- GET 系は `actor` クエリが必須。無いと 400 `bad_request`。
- GET でも `Content-Length` ヘッダーが要る。`curl` の素の GET（`Content-Length` 無し）は 400 `bad_request` になる。`verify-run` は全要求に `Content-Length` を付ける。
- 同じ `client_msg_id`・同じ本文の再送は同じメッセージを返し、記録は増えない。違う本文を送ると 409。run ごとに `cm-verify-<run_id>` を使う。
- 証拠の `.req`・`.resp` では `body` を `<redacted len=N>` に伏せている。本文には demo-check の canary を埋めてあり、証拠に出たら run が FAIL になる。
- 保存はメモリだけ。プロセスを止めると投稿は消える（後片付けで消すべき一時状態はこれだけ）。
