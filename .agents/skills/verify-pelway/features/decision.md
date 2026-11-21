# 判断待ちと決定

## Sub-features

- 判断待ちの一覧: `GET /v1/workspaces/{workspace_id}/decisions?actor=<member_id>`（自分が決定者の未確定だけ。`request_message_id`・`options`・`version`・`wait_ms`）
- 決定: `POST /v1/decisions/{decision_id}/decide`、本文 `{"decision_id", "version", "option_id"}`、操作者は合成ヘッダー
- 確定後の扱い: 確定したものは一覧から消える。確定後に別の案を選ぶと 409 `already_decided`

## How to get to it (user POV)

山本（`m-yamamoto`、デモの決定者）の判断待ちに、田中の依頼から作られた A/B の決定が 1 件来る。山本が案A を選ぶと確定し、判断待ちから消える。

## Driving it with verify-run

`verify-run --feature decision` の段 3（判断待ちが現れる）、段 5（案A で決定）、段 6（判断待ちから消えたことを読み直す）。応答は `steps/NN-decisions-pending.resp`・`steps/NN-decide.resp`・`steps/NN-decisions-after.resp`。テストは `cargo test -p pelway-http --test decisions`。

## Gotchas

- 決定者は `demo-serve` が `m-yamamoto` に固定している。ほかの人で決定すると 403 `not_decider`。
- `version` は一覧の値をそのまま送る。違う値は 409 `version_conflict`。
