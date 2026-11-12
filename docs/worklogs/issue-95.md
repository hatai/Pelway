# Issue #95 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 手順6（mise run pr-size）: 実装一式（`crates/pelway-llm` の src 758 行 + モックHTTPサーバー結合試験 706 行、Cargo.lock・mise.lock は linguist-generated で除外）で追加 +1464 行となり上限 800 を超えた。Issue の分割案 1)（ツールチェーン更新、+20 行弱）だけをこの round の Draft PR とし、2)・3) は Issue への分割案コメントに回した。ほか: reqwest 0.13 の TLS feature 名は `rustls`（`rustls-tls` ではない）、rig 0.46 は chat の system メッセージ本文を `[{"type":"text","text":...}]` の配列に直列化する（`json_object` 向けスキーマ注入の試験のアサーションを合わせた）。
2. 曖昧だった文: 「800行を超えそうなら、1) ツールチェーンの更新、2) trait と rig-core 接続と試験、3) 測定コマンドと合成データ、に分ける（split-plan は指揮役が書く）。」（Issue #95 の「規模の見積もり」）— 2) の trait と試験だけで既に 800 行を超える見込みのとき、実装者が 2) をさらに分割してよいかが決まっていない。
3. 仮定と解釈: 仮定: 分割単位の決定は指揮役の権限（skill `implement-issue` の停止条件と同じ）。解釈: 上限内に収まる 1) だけを Draft PR として先に進め、2)・3) の分割案は Issue コメントに書いて指揮役の split-plan を待つ。選ばなかった: 試験や説明を削って 800 行に収める（skill が禁止）、行数を無視して全量を 1 PR にする（PR の上限ルール違反）。
4. 足りなかった情報・検証手段: rig-core 0.46 の呼び出し経路（`OpenAIConfig::connect(DynHttpClient)` → `Model<Chat>::call`、OPENAI 弁話の `completion()` は Responses API に行くため `.chat()` を使う点）— docs/research/rust-llm-client-comparison.md の記述と一致したが、~/.cargo/registry のソースで確認した。2) を分割する場合の試験の切り出し単位。
5. 改善案: docs/research/rust-llm-client-comparison.md の「rig-core」の節に「0.46 の OPENAI 弁話は `completion()` が Responses API に行く。Chat Completions 限定なら `chat()` を使う」を追記する。
