# ハルへのメンションから A/B 案

## Sub-features

- メンションからのタスク化: 本文に `@ハル` を含む投稿で runner-lite がタスクを起こす（bot 自身の投稿・メンション無しでは起こさない）
- 進捗の 1 行: 「依頼を受け付けました。A/B 案を作成しています。」
- A/B 案の投稿: 「案を作成しました。どちらにするか判断をお願いします。」に続けて `案A: ...`・`案B: ...`

## How to get to it (user POV)

田中が #案件B で「@ハル 納期延長の案を2つ作って」と発言すると、ハル（`m-hal`、bot）が受付の 1 行を返し、続けて A/B 案を投稿する。

## Driving it with verify-run

`verify-run --feature decision` の段 2（メンションの投稿）と段 4（ハルの投稿が 2 件以上）。案の中身は `steps/NN-messages-after-mention.resp` に残る。テストは `cargo test -p pelway-http --test runner_mention`。

## Gotchas

- `demo-serve` の LLM は台本モード（`ScriptGateway`）に固定してある。案は毎回「納期を2週間延ばす案」「範囲を絞って納期を守る案」。実 LLM での案の生成はこのスキルでは確かめない（外部への実 I/O と費用が発生するため）。
- A/B 案は非同期に投稿される。投稿の応答（200）の時点ではまだ無い。`verify-run` は判断待ちが現れるまで最大 10 秒待ってから読む。
