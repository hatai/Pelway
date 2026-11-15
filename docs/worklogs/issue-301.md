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

## 実装者メモ(round 2)

対象 head: 8633f40（round 2 の実装の実体を含むコミット。このメモを含むコミットの親。履歴にマージコミットを含むため署名付け替え無し）

1. 詰まった箇所: なし(修正依頼の内容が具体的だった)。round 1 の抑制コメントの削除は、8 ファイルの origin/main との差分がコメント追加のみであることを事前確認して `git checkout origin/main --` で戻した。
2. 曖昧だった文: 「抑制コメントが残っていない → diff で確認」(Issue #301 指揮役コメント 2026-11-15 の受け入れ条件)— round 1 で足した理由コメント(本番 3 件の理由 3 行・mod doc 拡張・doc コメント拡張)も抑制コメントの付属物と解釈して全部削除し、crates/ を main と同一に戻した(選ばなかった解釈: 「実在の秘密ではない」等の doc 拡張だけは説明として残す)。
3. 仮定と解釈: 検証テストは「8 ファイルに `codeql[` が無いこと」をファイル単位で検証する形に転換した(行単位の不在確認までは不要で、ファイル単位で十分条件を確かめると判断)。断片不改変の検査は round 1 と同じ形式で残した。
4. 足りなかった情報・検証手段: 特になし(指揮役コメントが方式・受け入れ条件・dismiss の手順まで与えていた)。
5. 改善案: `docs/dev-environment.md` の CI 周りに「Code Scanning はデフォルトセットアップで動く。Rust のインライン抑制は効かない(2026-11-15 実測)。アラートの解消は指揮役が hatai 名義で API dismiss する(理由は worklog の対応表)」を足す。

使ったスキル: implement-issue、pr-watch(round 1 のチェック待ちと digest)

## 実装者メモ(round 3)

対象 head: b2dcdd8（main の取り込み(マージコミット)。このメモを含むコミットの親）

1. 詰まった箇所: 2 点。mise.toml の衝突解消に使った mise shim の python3 が、衝突マーカー入り mise.toml のパースエラーで起動せず `/usr/bin/python3` で処理した。マージコミットを素の `git commit` で作ってしまい author が hatai になったため、`scripts/agent-gh dev git commit --amend --reset-author` で pelway-dev[bot] に修正した(round 2 のマージコミット ab489f9 は hatai 名義のまま。履歴書き換えはしない)。
2. 曖昧だった文: 「.kodiak.toml は main 側を採用すること」(修正依頼)— ブランチ側に .kodiak.toml が存在しなかったため、main 側の追加をそのまま受け入れる形で自然に解消した(実際の衝突は mise.toml の script-test 行の 1 箇所のみ)。
3. 仮定と解釈: 「抑制コメント削除後の crates/ は main と同一のはず」— 同一であることを前提にマージし、crates/ は自動解消(衝突なし)で main 側(pelway-gateway の削除等を含む)が入った。8 ファイルの抑制コメント不在は `scripts/tests/codeql-suppressions.test.sh` がマージ後も `ok` で確認。
4. 足りなかった情報・検証手段: なし(修正依頼が手順を特定していた)。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「6. 検査してコミットし、行数を確かめてから push する」に「main の取り込み(マージコミット)も `scripts/agent-gh dev git commit` で作る(素の `git commit` は author が hatai 名義になる。マージなら `--amend --reset-author` で直せる)」を足す。

使ったスキル: implement-issue

## 実装者メモ(round 4)

対象 head: 27d0fa1（main の取り込み(マージコミット)。このメモを含むコミットの親）

1. 詰まった箇所: 素の `git merge` の自動コミットが hatai 名義になった(round 3 と同じ失敗)→`scripts/agent-gh dev git commit --amend --reset-author` で pelway-dev[bot] に修正。マージ自体は衝突なしで自動解消(Kodiak の衝突判定は #320 マージ直後の一時的なものと推定)。
2. 曖昧だった文: 「main を取り込んで衝突を解消(両側の行を保持)」(修正依頼)— 実際にはテキスト衝突は発生せず、HEAD 側の `script-test` 行(`codeql-suppressions.test.sh` と `worklog-curation.test.sh` の両方を含む)がそのまま保持された。#320(#316 のスキル取り込み)は script-test 行を触っていなかった。
3. 仮定と解釈: push により Codex・Claude 両担当の approve(head 5ea4fa9)は dismiss される(dismiss stale 有効は round 3 の実績)ため、取り込み後に両担当へ再レビューを依頼する。automerge ラベルの付け直しは指揮役が行う(修正依頼の指定どおり実装側では付けない)。
4. 足りなかった情報・検証手段: Kodiak が「merge conflict」と判定した正確な理由(コメントに詳細が無く、取り込み時点では MERGEABLE だったため推定のまま)。
5. 改善案: `scripts/agent-gh` に author/committer を App 名義にする `git merge` ラッパー(または implement-issue への明記)を足す(round 3 の改善案と同じ根: 素の git のマージコミットが hatai 名義になる)。

使ったスキル: implement-issue
