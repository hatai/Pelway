# Issue #301 の作業ログ

## 対応表(全 32 件。分類は 2026-11-15 に main の実コードで再確認)

方式は指揮役コメント(2026-11-15、hatai 判断)の (c): 抑制コメントは使わず、対応表を
リポジトリ側の記録として残し、マージ後に指揮役が API で 32 件を理由付き dismiss する
(dismissed_reason は本番コードの誤検出が false positive、テストコードの固定値が used in tests)。
抑制コメント(`// codeql[rust/...]`)が Rust に効かない実測の経緯は実装者メモ(round 1)と
Issue コメント(https://github.com/hatai/Pelway/issues/301#issuecomment-6888669465)参照。

再確認の結果、Issue 本文の分類(本番コードの誤検出 3 件・テストコードの固定値 29 件)と一致した。
gdpr02_routes.rs・no_body_logs.rs は src 内だが `crates/pelway-http/src/lib.rs` の 92・97 行が
`#[cfg(test)] mod` 宣言であることを確認し、テストコードと分類した。
行番号は Issue 記載(main)のもの。

| アラート | 場所(main) | ルール | 分類(dismissed_reason) | 理由 |
|---|---|---|---|---|
| #27 | pelway-at-rest/src/lib.rs:119 `wrap_nonce` | hard-coded-cryptographic-value | 本番誤検出(false positive) | 0 埋めは未初期化メモリを読まないためだけの初期化で、直後の os_fill が OS の乱数源から上書きする。この値は使われない |
| #28 | pelway-at-rest/src/lib.rs:120 `payload_salt` | hard-coded-cryptographic-value | 本番誤検出(false positive) | 同上(#27) |
| #26 | pelway-at-rest/src/container.rs:107 `chunk_nonce` | hard-coded-cryptographic-value | 本番誤検出(false positive) | 0 埋めは初期化のみ。nonce はチャンク連番から一意に構成され、ファイルごとの乱数鍵の下で使われるため再利用は起きない |
| #29 | pelway-at-rest/src/lib.rs:234 `SALT` | hard-coded-cryptographic-value | テスト固定値(used in tests) | mod tests の固定材料([0x44; SALT_LEN])。閉じた検証の再現にだけ使い実在の秘密ではない |
| #30 | pelway-at-rest/src/lib.rs:287 `[0x22; NONCE_LEN]` | hard-coded-cryptographic-value | テスト固定値(used in tests) | 互換ベクトル(AES_GCM_0_10_CONTAINER)と同じ材料。依存の版が変わっても保存形式が同じことの保証に固定が必要 |
| #1〜#9 | pelway-at-rest/src/lib.rs:300-320 `chunk_nonce(...)` | hard-coded-cryptographic-value | テスト固定値(used in tests) | チャンク番号 0/1 と最終フラグの組で AAD・連番・フラグの束ねを確かめるテストの入力 |
| #10〜#21 | pelway-contract/src/pseudonym.rs:45-71 `b"salt"` 等 | hard-coded-cryptographic-value | テスト固定値(used in tests) | 仮名化の挙動(salt・入力の組)を固定して確かめるテスト用 salt。本番の salt は OS の乱数源から作る(62・63 行は各行 2 件で計 12 件) |
| #22 | pelway-http/src/gdpr02_routes.rs:35 `SALT` | hard-coded-cryptographic-value | テスト固定値(used in tests) | `#[cfg(test)]` モジュール(QA-GDPR-02 検証)の合成 salt。作り方は log.rs の単体テストで確認済み |
| #23 | pelway-http/src/lib.rs:1343 `KNOWN_SALT` | hard-coded-cryptographic-value | テスト固定値(used in tests) | `#[cfg(test)]` 内。salt ファイルから読んだ salt が仮名化に使われることの検証用の 32 バイト |
| #24 | pelway-http/src/no_body_logs.rs:36 `SALT` | hard-coded-cryptographic-value | テスト固定値(used in tests) | `#[cfg(test)]` モジュール(GDPR-07 検証)の合成 salt |
| #25 | pelway-http/src/log.rs:461 `b"unit-salt"` | hard-coded-cryptographic-value | テスト固定値(used in tests) | mod tests 内。期待値を同じ salt で計算するための固定 |
| #31 | pelway-llm/tests/pure.rs:40 `assert!(..., "{debug}")` | cleartext-logging | テスト固定値(used in tests) | assert の失敗メッセージに {debug} を含めるが、SecretToken の Debug は "<redacted>" プレースホルダで秘密を含まない |
| #32 | pelway-llm/tests/pure.rs:41 `assert!(!..., "{debug}")` | cleartext-logging | テスト固定値(used in tests) | 秘密がログに出ないことを検証する側の行。`sk-...` も合成 canary で実在の秘密ではない |

固定値のバイト列はすべて不変(crates/ は main と同一。`scripts/tests/codeql-suppressions.test.sh`
が断片の不改変を機械検証する)。

## 実装者メモ(round 1)

対象 head: 45a3bd2（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え後の SHA）

1. 詰まった箇所: `scripts/agent-gh`: sandbox 内で `/etc/resolv.conf`(symlink → `/mnt/wsl/resolv.conf`)が解決できず DNS が全滅した。`/` が rw tmpfs なので、各コマンドで `mkdir -p /mnt/wsl && echo 'nameserver 192.168.1.1' > /mnt/wsl/resolv.conf` を前置して回避した。`mise run test`: mbx が read-only の `~/.cache` に書けず失敗 → `XDG_CACHE_HOME=/tmp/xdg-cache` で回避。`scripts/tests/agent-sandbox.test.sh` テスト12: `~/.t3/worktrees/` 下の mkdir が sandbox で拒否され失敗(本 Issue の変更前でも同じ失敗を確認 → 環境起因。CI の HOME は書き込み可のため通る)。
2. 曖昧だった文: 「抑制コメントが CodeQL の Rust 抽出で実際に効くかは次回スキャンで検証する」(Issue #301 本文「採用する方式」)— マージ前には確かめられないため、Issue の方針どおり行末の `// codeql[rust/<rule>]` 形式を採用し、効くかの検証は次回スキャンに委ねた。
3. 仮定と解釈: アラート行と同一行の行末が CodeQL の有効な抑制位置と仮定した(rustfmt が行末コメント付きの過長行を維持することも実験で確認)。直前行形式は使わなかった(選ばなかった解釈: 効く位置の候補が2つある場合の混在を避けた)。検証テストは行番号非依存(コード断片で一意に特定)にした。
4. 足りなかった情報・検証手段: CodeQL の Rust インライン抑制の公式ドキュメント(docs.github.com)。ネットワーク制約(scripts/agent-gh 経由の GitHub API のみ)で取得できず、`gh api` での github/codeql リポジトリ調査でも Rust の抑制サポートの明文には到達できなかった。
5. 改善案: `docs/dev-environment.md` の「実装スレッドの sandbox(Issue #151)」に「sandbox 内で DNS を使うコマンド(agent-gh 等)は /mnt/wsl/resolv.conf の用意が要る」旨と「agent-sandbox.test.sh のテスト12はネスト sandbox 内では環境起因で失敗する」旨を足す。

使ったスキル: implement-issue
