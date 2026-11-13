# Issue #87 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: crates/pelway-contract/src/snapshot.rs: 旧 `compute_snapshot_hash` の第 1 引数 `payload_hash` が不透明な文字列で、payload の中身がどこにも存在しなかった。このため `snapshot::ActionPayload` 型を新設し、`ApprovalCard`/`NewCard` の `payload_hash`・`action_kind`・`recipient`・`body` を `payload` 1 つに集約した
2. 曖昧だった文: 「ハッシュの対象を、正規化した操作の中身（操作の種類、宛先、本文、添付、使うアカウント、範囲）に限る」（Issue #87 本文・やること）— 「正規化」でどの表現の差を同一視するかが決まっていない
3. 仮定と解釈: 正規化は意味を変えない表現の差に限ると解釈した。採った解釈: 本文は改行を LF に統一し末尾の空白を除去、添付と scope は並べ替え＋重複除去（集合として扱う）。選ばなかった解釈: 本文先頭空白の除去・Unicode NFC 正規化・scope の大文字小文字統一（意味を変えるリスクのため）。ほかに、送信中・送信済みカードへのコメント保存 409 は維持し、その理由の記述を hash 整合から「承認が確定したカードの議論欄を後から変えないため」に書き直した
4. 足りなかった情報・検証手段: docs/technical/v0.7/body/03-ai-bot-runtime.md の「固定承認と外部副作用」は正規化対象に「参照版、許可scope、integration版」も列挙するが、Issue の列挙（種類・宛先・本文・添付・アカウント・範囲）には無い。Issue に従い参照版・integration 版は hash 対象から外した
5. 改善案: docs/technical/v0.7/body/03-ai-bot-runtime.md の「固定承認と外部副作用」に「デモ契約（pelway-contract の `ActionPayload`）は §5.1 の列挙のうち参照版・integration版を除く 6 項目を使う」という注記を足す
