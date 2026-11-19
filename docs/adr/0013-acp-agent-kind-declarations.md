---
status: accepted
---

# 外部エージェントの種別は宣言 data の固定項目で登録し、環境変数は許可リストだけを継承する

ACP v2 の Client adapter（#133 の下書き [`crates/pelway-gateway/src/acp.rs`](../../crates/pelway-gateway/src/acp.rs)）は、エージェント種別を code（enum・trait 実装）ではなく data として扱う（R1-Q2 案b）。この ADR はその data に書く項目を決める。ACP 連携自体は MVP後（[02 の外部エージェント行](../technical/v0.7/body/02-stack-comparison.md)、[ADR 0008](0008-llm-provider-and-byok.md) の BYOK 例外枠）のため、この決定は文書のみで、adapter の実装は変えない。

項目の考え方は Switch（commit 74e10ff）の実行役登録を参考にした。Switch のライセンスは Apache 2.0 に改変版 Commons Clause を足したものであるため、コードは写さず考え方だけを採る。

## 種別の宣言項目の採否

| 項目 | 採否 | 内容 |
|---|---|---|
| 識別子 | 採る | 種別の安定 ID。設定・監査・ログ（仮名化対象）の単位。提供元の名前と一致しなくてよい |
| 表示名 | 採る | UI に表示する名前（[06 の i18n](../technical/v0.7/body/06-i18n.md) に載せる） |
| 起動コマンド | 採る | task コンテナ内で stdio 子プロセスを起動するときの実行ファイルと引数。shell 文字列を連結せず argv 配列で持つ（[11](../technical/v0.7/body/11-integrations.md) の「CLIとlocal bridge」の行と同じ条件） |
| 継承してよい環境変数の許可リスト | 採る | 下の「環境変数の継承規則」参照。既定は空 |
| 対応しない OS と理由 | 採る | 動かない OS とその理由。対応 OS の列挙ではなく、確認できた不行使だけを書く |
| サインイン確認の方法 | 採る | 下の「サインイン確認」参照 |
| ACP の差分のフック | 採る | CLI ごとの違いを小さなフックの束で吸収する。フックだけを強制承認境界にしない（[11](../technical/v0.7/body/11-integrations.md) の「hooks」の行と同じ） |
| サインインのコマンド | 採らない | Switch は持つが、確認の自動化のつもりで対話的なサインインが始まる経路を作らない。サインインの開始は種別 data から行えず、利用者が手動手順（別 Issue）で行う |

## 環境変数の継承規則

- 子プロセスへ渡す環境変数は、種別の許可リストに載っているものだけとし、載っていないものは渡さない（既定拒否）。
- Pelway の秘密は許可リストに入れられない。fnox が管理する値（`PELWAY_LLM_API_KEY` 等）と資格情報 broker のトークンが該当する。登録時に静的に拒否する。秘密の拒否と `PELWAY_*` の判定は、許可リストの照合と同じ規則（Windows では大文字小文字を区別しない）で行い、大文字小文字だけ変えた名前（`pelway_llm_api_key`）で拒否を通り抜けさせない。
- Pelway 自身の動作に使う変数（`PELWAY_*`）も、明示的に許可されない限り渡さない。
- 許可リストの照合は Windows では大文字小文字を区別せずに行う。Windows の環境変数ブロックは大文字小文字を区別しないため、区別して照合すると `PATH` と `Path` を別の変数として扱えてしまう。

task コンテナには秘密を渡さないという[03](../technical/v0.7/body/03-ai-bot-runtime.md) の方針（「ホストホーム、DB資格情報、container runtime socketを渡さない」「Bot・LLM・成果物・ログへ値を出さない」）を、種別の登録面でも同じ形にする。

## サインイン確認

サインイン確認は、サインインを始めずに状態だけを見る。確認の方法が対話的なサインインフロー（ブラウザの起動や入力待ち）を始めてはならない。未サインインなら「未サインイン」と表示し、サインイン自体は利用者が手動手順（別 Issue）で行う。

## adapter との境界

この決定は acp.rs の冒頭コメントの方針を変えない。adapter（gateway）はエージェント・プロセスを起動せず、起動と隔離（環境変数・cwd・ファイルシステム・ネットワーク）は呼び出し元の task コンテナが transport を作る時点で強制する。起動コマンドと許可リストはその task コンテナ側の launcher（後続 Issue）がこの data から消費し、adapter は隔離済みの transport だけを受け取る。試験は in-process の Agent double で行い、実エージェント接続は別 Issue の手動手順とする（R1-Q2 案b）。

## Consequences

- 実エージェント（Claude Code・Codex 等）の追加は、この表の項目を埋める登録作業になる（手動手順、別 Issue）。
- 許可リストは種別ごとに保持し、追加・変更は明示的な登録変更として記録する。
- 段階の宣言と fail-closed（[ADR 0004](0004-declared-action-tiers.md)）、BYOK の例外枠（[ADR 0008](0008-llm-provider-and-byok.md)）はこの ADR の前提であり、この ADR はそれらを変更しない。
