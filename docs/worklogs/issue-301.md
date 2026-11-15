# Issue #301 の作業ログ

## 対応表(全 32 件。分類は 2026-11-15 に main の実コードで再確認)

再確認の結果、Issue 本文の分類(本番コードの誤検出 3 件・テストコードの固定値 29 件)と一致した。
gdpr02_routes.rs・no_body_logs.rs は src 内だが `crates/pelway-http/src/lib.rs` の 92・97 行が
`#[cfg(test)] mod` 宣言であることを確認し、テストコードと分類した。
行番号は Issue 記載(編集前の main)のもの。理由コメントを挿入したため、編集後の行番号はずれる。

| アラート | 場所(編集前) | ルール | 分類 | 抑制の形 |
|---|---|---|---|---|
| #27 | pelway-at-rest/src/lib.rs:119 `wrap_nonce` | hard-coded-cryptographic-value | 本番誤検出 | 行末抑制+理由 3 行(os_fill で上書き) |
| #28 | pelway-at-rest/src/lib.rs:120 `payload_salt` | hard-coded-cryptographic-value | 本番誤検出 | 行末抑制+理由 3 行(os_fill で上書き) |
| #26 | pelway-at-rest/src/container.rs:107 `chunk_nonce` | hard-coded-cryptographic-value | 本番誤検出 | 行末抑制+理由 3 行(連番から一意に構成、再利用なし) |
| #29 | pelway-at-rest/src/lib.rs:234 `SALT` | hard-coded-cryptographic-value | テスト固定値 | 行末抑制+mod doc の一括理由 |
| #30 | pelway-at-rest/src/lib.rs:287 `[0x22; NONCE_LEN]` | hard-coded-cryptographic-value | テスト固定値 | 行末抑制+mod doc の一括理由 |
| #1〜#9 | pelway-at-rest/src/lib.rs:300-320 `chunk_nonce(...)` | hard-coded-cryptographic-value | テスト固定値 | 行末抑制+mod doc の一括理由 |
| #10〜#21 | pelway-contract/src/pseudonym.rs:45-71 `b"salt"` 等 | hard-coded-cryptographic-value | テスト固定値 | 行末抑制+mod 内一括コメント(62・63 行は各行 2 件で計 12 件) |
| #22 | pelway-http/src/gdpr02_routes.rs:35 `SALT` | hard-coded-cryptographic-value | テスト固定値(cfg(test) mod) | 行末抑制+doc コメント拡張 |
| #23 | pelway-http/src/lib.rs:1343 `KNOWN_SALT` | hard-coded-cryptographic-value | テスト固定値(cfg(test) 内) | 行末抑制+直前コメント拡張 |
| #24 | pelway-http/src/no_body_logs.rs:36 `SALT` | hard-coded-cryptographic-value | テスト固定値(cfg(test) mod) | 行末抑制+doc コメント拡張 |
| #25 | pelway-http/src/log.rs:461 `b"unit-salt"` | hard-coded-cryptographic-value | テスト固定値(mod tests 内) | 行末抑制+mod 内一括コメント |
| #31 | pelway-llm/tests/pure.rs:40 `assert!(..., "{debug}")` | cleartext-logging | テスト(Debug は "<redacted>" 置換) | 行末抑制+doc コメント拡張 |
| #32 | pelway-llm/tests/pure.rs:41 `assert!(!..., "{debug}")` | cleartext-logging | テスト(秘密が出ないことの検証側) | 行末抑制+doc コメント拡張 |

固定値のバイト列はすべて不変(diff はコメント追加のみ)。抑制コメントが CodeQL の Rust スキャンで
実際に効くかは、マージ後の Code Scanning で検証する(Issue「採用する方式」の予定どおり)。

## 実装者メモ(round 1)

対象 head: 45a3bd2（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え後の SHA）

1. 詰まった箇所: `scripts/agent-gh`: sandbox 内で `/etc/resolv.conf`(symlink → `/mnt/wsl/resolv.conf`)が解決できず DNS が全滅した。`/` が rw tmpfs なので、各コマンドで `mkdir -p /mnt/wsl && echo 'nameserver 192.168.1.1' > /mnt/wsl/resolv.conf` を前置して回避した。`mise run test`: mbx が read-only の `~/.cache` に書けず失敗 → `XDG_CACHE_HOME=/tmp/xdg-cache` で回避。`scripts/tests/agent-sandbox.test.sh` テスト12: `~/.t3/worktrees/` 下の mkdir が sandbox で拒否され失敗(本 Issue の変更前でも同じ失敗を確認 → 環境起因。CI の HOME は書き込み可のため通る)。
2. 曖昧だった文: 「抑制コメントが CodeQL の Rust 抽出で実際に効くかは次回スキャンで検証する」(Issue #301 本文「採用する方式」)— マージ前には確かめられないため、Issue の方針どおり行末の `// codeql[rust/<rule>]` 形式を採用し、効くかの検証は次回スキャンに委ねた。
3. 仮定と解釈: アラート行と同一行の行末が CodeQL の有効な抑制位置と仮定した(rustfmt が行末コメント付きの過長行を維持することも実験で確認)。直前行形式は使わなかった(選ばなかった解釈: 効く位置の候補が2つある場合の混在を避けた)。検証テストは行番号非依存(コード断片で一意に特定)にした。
4. 足りなかった情報・検証手段: CodeQL の Rust インライン抑制の公式ドキュメント(docs.github.com)。ネットワーク制約(scripts/agent-gh 経由の GitHub API のみ)で取得できず、`gh api` での github/codeql リポジトリ調査でも Rust の抑制サポートの明文には到達できなかった。
5. 改善案: `docs/dev-environment.md` の「実装スレッドの sandbox(Issue #151)」に「sandbox 内で DNS を使うコマンド(agent-gh 等)は /mnt/wsl/resolv.conf の用意が要る」旨と「agent-sandbox.test.sh のテスト12はネスト sandbox 内では環境起因で失敗する」旨を足す。

使ったスキル: implement-issue
