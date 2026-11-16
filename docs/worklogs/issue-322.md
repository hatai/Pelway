# Issue #322 の作業ログ

## 受け入れ条件の対応表

| 受け入れ条件 | 対応 |
|---|---|
| round 6 must-fix 4 件それぞれについて、指摘の攻撃経路が塞がれていることを試験で固定する | 下の「試験名の対応表」。`cargo test -p pelway-gateway --features acp` で 30 lib + 4 inprocess_gateway + 2 mcp_edges + 1 stdio_child + 2 doctest が全て合格 |
| 信頼済み launcher のみが隔離 transport・連携 ID を構築できることを型またはアクセス制御で示す | `IsolatedTransport` の doc コメントにある 2 本の `compile_fail` doctest で固定（`bind` は `pub(crate)`、`bind_in_process` は `#[cfg(test)]`、フィールド非公開のため、クレート外の通常ビルドから構築経路が存在しないことをコンパイル不可で示す）。実行時の結び付き照合は `identity_mismatch_refuses_before_session` |
| nit 3 件の記述とコードが一致する | (a) 強制点所在: mod doc（acp.rs 冒頭）が launcher/task container 側の隔離を前提と記述、(b) 連携 ID 照合: `AcpAuditEvent::IdentityRejected` の doc を launcher 結び付け ID の照合に合わせて修正（本 PR の差分）、(c) `process` feature 不在と「adapter は AcpAgent を起動しない」記述: `crates/pelway-gateway/Cargo.toml` 39 行目の記述と `acp = [...]` の feature 定義で一致を確認（差分なし） |
| `mise run verify` が成功 | sandbox 内では環境起因で完全には通らない（下のメモ 1 参照）。コードに関わる各段（fmt-check・clippy・deny-check・hk・toolchain-check・script-test 18/19）は合格。最終判定は CI |

## round 6 must-fix と試験名の対応表

| must-fix | 攻撃経路 | 固定する試験 |
|---|---|---|
| 試験経路の構築が本番の隔離境界を迂回する | クレート外・通常ビルドから任意の連携 ID を結び付けた transport を偽造 | `IsolatedTransport` doc の 2 本の `compile_fail` doctest（`bind`・`bind_in_process` それぞれ不可呼出を固定）+ `identity_mismatch_refuses_before_session`（transport 結び付け ID と grant の照合が `initialize` 前に fail closed） |
| frame を decode 前にサイズで止めない | 巨大 frame が decode・割当に達する | `frame_limit_trips_before_trailing_newline`（同一 read 内の末尾改行前に検出）+ `giant_frame_fails_before_decode` + `frame_limit_allows_valid_frames` |
| byte 超過時に `recv()` 待ちの receiver が起きない | 滞留 byte 上限超過が待機中のセッションを起こさず無応答化 | `late_giant_wakes_waiting_receiver` + `giant_updates_close_the_session` + `flooding_updates_fail_the_session`（件数側）+ `flooding_tool_calls_fail_the_session`（SeenTools 相関表の同一路） |
| 承認証跡が未検証の値型 | `Some("")`・空白・期限切れ・`outcome_unknown` の証跡で許可 | `evidence_new_rejects_empty_fields_and_past_expiry` + `allow_without_evidence_fails_closed` + `expired_evidence_fails_closed` + `expired_evidence_fails_closed_at_allow_time` + `outcome_unknown_never_allows` + `empty_operation_name_fails_closed` |

round 7 の追加分（相関表の byte 上限・`take` 消費・allow 時の期限再検査・`#[cfg(test)]` 隔離の維持）も `seen_tools_is_bounded`・`seen_tools_byte_bounded`・`flooding_tool_calls_fail_the_session`・`expired_evidence_fails_closed_at_allow_time`・`compile_fail` doctest で固定済み。

## 決めること: launcher 実装の扱い

**別 Issue に分ける**（本 Issue には含めない）を選ぶ。

| 観点 | 本 Issue に含める | 別 Issue |
|---|---|---|
| 受け入れ条件との関係 | AC2 の「構造の保証」は sealed 構築で既に示せるため、launcher 実装なしでも達成できる | launcher 実装は AC のどの条件にも必須でない（AC2 は型・アクセス制御の話） |
| 規模 | task container の環境・ファイル・ネットワーク隔離の設計＋実装は 800 行上限を確実に超える | 独立 Issue として方式比較（採用する方式の選定）から始められる |
| 依存 | adapter 側の境界は `IsolatedTransport::bind` の `pub(crate)` に留めてあり、launcher 実装時に初めて公開 seam を決める設計になる。本 Issue で seam を先走って決める根拠が無い | launcher 側の要件（cgroup・netns・fs 制限）が固まってから seam を設計できる |

## 実装者メモ（round 1）

対象 head: fa160f2（この round の実装の実体を含むコミット。このメモを含むコミットの親。署名付け替え後の SHA）

1. 詰まった箇所: 実装 sandbox 内で 3 点。`/etc/resolv.conf`（symlink → `/mnt/wsl/resolv.conf`）がぶら下がり DNS 全滅 → コマンドごとに `mkdir -p /mnt/wsl` と nameserver 書き込みで回避。`~/.cache/mbx` が read-only で cargo が mbx 経由のまま失敗 → `XDG_CACHE_HOME=/tmp/xdg-cache-322`（dev-environment.md「mr boxington について」節の記載どおり）。`mise run verify` は環境起因で完全には通らない: `scripts/tests/agent-sandbox.test.sh` の plugin file-tool 検査（`~/.t3/worktrees` 書き込み不可でネスト sandbox では必ず失敗。issue-301 worklog と同根）、`pelway-agent-watch` の `unwritable_state_does_not_repeat_events`（並行する別セッションの verify 実行下でのみ失敗、単独実行では合格）、`conductor-watch-ensure.test.sh`（同じく単独では合格。watch プロセスを数える試験のため別セッションの verify と干渉したと推定）。
2. 曖昧だった文: 「信頼済み launcher のみが隔離 transport・連携 ID を構築できることを型またはアクセス制御で示す → 該当試験またはレビュー指摘の引用」（Issue #322 受け入れ条件）— 「示す」が実行時検査か型上の不在かで迷ったが、sealed 構築（`pub(crate)`・`#[cfg(test)]`・非公開フィールド）は型上の保証なので、実行時試験ではなく `compile_fail` doctest で「構築経路が存在しない」ことを固定する形を選んだ（選ばなかった解釈: 外部クレートからの構築を試みる結合試験。試験コード内からは `pub(crate)` が見えてしまい意図どおりに失敗しない）。
3. 仮定と解釈: launcher 実装は本 Issue の範囲外と仮定した（比較表は上の「決めること」。「別 Issue」を選んだが起票は指揮役判断に委ね、本 PR では提示のみ）。`ApprovalEvidence::new` の試験追加は round 6 の「validated type」修正を試験で固定する AC1 の範囲内と解釈し、実装の追加変更は行わなかった（round 6・7 の実装本体は PR #304 でマージ済みで、現状コードが指摘どおり境界を閉じていることを試験で確認）。
4. 足りなかった情報・検証手段: round 6 レビュー本文の位置（PR #304 のレビュー履歴が正本。Issue 本文には「round 6 レビュー本文を正本とする」とだけあり、レビュー 9 件から該当 round を抽出する必要があった）。ネスト sandbox での検証手段の制約（verify の環境起因失敗の識別）は issue-301 worklog に記録済みだが、dev-environment.md の sandbox 節には反映がなかった。
5. 改善案: `docs/dev-environment.md` の「実装担当の sandbox（agent-sandbox）」に「ネストされた sandbox 内では `scripts/tests/agent-sandbox.test.sh` の plugin file-tool 検査（テスト 12）が `~/.t3/worktrees` への書き込み不可で環境起因の失敗になる」「sandbox 内で DNS を使うコマンドは `/mnt/wsl/resolv.conf` を用意してから実行する（`/etc/resolv.conf` がぶら下がる）」を足す（issue-301 メモの改善案と同根で未適用だった分と、mbx の `XDG_CACHE_HOME` 退避の索引）。

使ったスキル: implement-issue、write-pull-request
