# Issue #168 の作業ログ

## 経緯（round 1: コピーレフト検出で停止）

1. `mise.toml` の `[tools]` に `"aqua:EmbarkStudios/cargo-deny" = "0.20.2"` を追加し、`mise install` で `mise.lock` に URL と checksum を記録した（cargo-llvm-cov・octocov と同じ aqua 経路。コミット済みの差分として round 2 で push 予定）。
2. `cargo deny init` の雛形と 0.20.2 の公式リファレンス（`docs/src/checks/advisories/cfg.md` @ tag 0.20.2）で設定項目を確認した。
3. プローブ用の deny.toml（advisories: yanked=deny、licenses: allow=[]、bans: multiple-versions=warn / wildcards=deny、sources: unknown-registry/unknown-git=deny）で現状の依存ツリーを検査した。`cargo deny` の評価対象は 764 クレート（Cargo.lock 835 件のうち、ターゲット固有・feature-gated・dev-only などグラフに入らない 72 件を除く。webauthn-authenticator-rs@0.6.2（MPL-2.0、pelway-auth の dev-dependency）も現在はグラフ外）。
4. ライセンスの棚卸しの結果、コピーレフト系が見つかったため、Issue の未決事項（「コピーレフト系のライセンスが依存ツリーにあれば、allow リストに入れず、該当のクレートと経路を worklog と Issue のコメントに書いて止まる」）に従い、allow リストを確定させずに停止した。deny.toml は未作成。

## ポリシーの判断と理由（round 1 時点の方針。round 2 で確定）

- **vulnerability は失敗**: 0.20.2 では既定で全 vulnerability advisory がエラー（`vulnerability` フィールドは v2 で廃止。設定不要）。
- **yanked は失敗**: 既定は `warn` のため `yanked = "deny"` を明示する。
- **unmaintained / unsound は失敗させない**: Issue は「警告にとどまる」と書くが、0.20.2 の `[advisories]` に lint level の warn は存在しない（`unmaintained` / `unsound` は `all` / `workspace` / `transitive` / `none` のスコープ選択のみ。`none` は診断を出さず失敗もしない）。「検査が失敗しない」ことを優先し `unmaintained = "none"` / `unsound = "none"` とする。既知の該当 advisory は ignore エントリ（理由 + フォロー Issue 番号付き）と `unused-ignored-advisory = "allow"` で deny.toml 内に台帳として残す予定。新しい unmaintained で CI が落ちない代わりに、検査出力に出なくなるトレードオフは docs/dev-environment.md に明記する。
- **licenses**: `confidence-threshold = 0.8`（既定値以上）。allow リストは棚卸し表の実際に使われているライセンスのみ。ワークスペース内クレート（license 宣言なし）は `[licenses.private] ignore = true` で対象外にする。
- **bans**: `multiple-versions = "warn"`、`wildcards = "deny"`。
- **sources**: `unknown-registry = "deny"`、`unknown-git = "deny"`（既定の allow-registry = crates.io index のみ）。

## ライセンスの棚卸し（cargo deny list、評価対象 764 クレート、2026-11-13 時点）

| ライセンス | クレート数 | 備考 |
|---|---|---|
| MIT | 658 | |
| Apache-2.0 | 538 | OR 式の一部として含むクレートあり |
| Unicode-3.0 | 28 | |
| Zlib | 23 | |
| BSD-3-Clause | 15 | |
| Unlicense | 14 | |
| GPL-3.0-only | 13 | すべて Slint 系（OR 式の1分岐。下記） |
| LicenseRef-Slint-Royalty-free-2.0 | 13 | 同上（OR 式の1分岐） |
| LicenseRef-Slint-Software-3.0 | 13 | 同上（OR 式の1分岐） |
| MPL-2.0 | 8 | 単独ライセンス。回避不可（下記） |
| Apache-2.0 WITH LLVM-exception | 7 | |
| ISC | 6 | |
| BSL-1.0 | 4 | |
| BSD-2-Clause | 3 | |
| LGPL-2.1-or-later | 2 | r-efi の OR 式の1分岐（MIT OR Apache-2.0 OR LGPL…）で allow 不要 |
| BSD-1-Clause | 2 | |
| GPL-2.0-only | 1 | self_cell@1.3.0 の OR 式の1分岐（Apache-2.0 OR GPL…）で allow 不要 |
| CDLA-Permissive-2.0 | 1 | webpki-roots（パーミッシブなデータライセンス。コピーレフトではない） |
| CC0-1.0 | 1 | imgref の OR 式の1分岐（CC0-1.0 OR Apache-2.0）で allow 不要 |
| 0BSD | 1 | |
| （license 宣言なし） | 13 | すべてワークスペース内クレート。`[licenses.private] ignore = true` で対象外 |

### コピーレフト系の内訳（停止理由）

**MPL-2.0（単独。OR の代替分岐なし。allow しない限り検査は通らない）**

| クレート | 依存の経路 |
|---|---|
| webauthn-rs@0.6.2 | pelway-auth（直依存 `=0.6.2`） |
| webauthn-rs-core@0.6.2 | webauthn-rs → webauthn-rs-core |
| webauthn-rs-proto@0.6.2 | webauthn-rs / webauthn-rs-core → webauthn-rs-proto |
| webauthn-attestation-ca@0.6.2 | webauthn-rs-core → webauthn-attestation-ca |
| crypto-glue@0.2.2 | webauthn-attestation-ca / webauthn-rs-core → crypto-glue |
| riverqueue@0.6.0 | pelway-dev-queue-compare（直依存。#99 の River 比較、ADR 0006） |
| riverqueue-macros@0.6.0 | riverqueue → riverqueue-macros |
| riverqueue-migrate@0.6.0 | riverqueue → riverqueue-migrate |
| （webauthn-authenticator-rs@0.6.2） | pelway-auth の dev-dependency。現時点では cargo deny の評価グラフ外だが Cargo.lock に存在。グラフに入れば 9 件目の MPL-2.0 |

**GPL-3.0-only（OR 式の1分岐。Slint 系 13 クレート）**

- ライセンス式: `GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0`
- 経路: pelway-client-ui / pelway-dev-client-probe → slint@1.18.1（+ ビルド依存 slint-build、i-slint-* 一式）
- GPL-3.0-only を allow しなくても、Slint 独自ライセンス（LicenseRef-Slint-Royalty-free-2.0 = 無償・ロイヤリティフリー、LicenseRef-Slint-Software-3.0 = 有償商用）のどちらかを allow すれば検査は通る。独自ライセンスの内容確認・承認は人間の判断。

**GPL-2.0-only / LGPL-2.1-or-later（OR 式の1分岐。allow 不要）**

- self_cell@1.3.0: `Apache-2.0 OR GPL-2.0-only` → Apache-2.0 の allow で満たる
- r-efi@5.3.0 / 6.0.0: `MIT OR Apache-2.0 OR LGPL-2.1-or-later` → MIT の allow で満たる

AGPL・EPL・CDDL は依存ツリーに無い。

## 初回検出と対応方針（round 2 で実施）

| 検出 | 内容 | 対応 |
|---|---|---|
| RUSTSEC-2023-0071（vulnerability、Marvin Attack） | rsa 0.9.10（openidconnect 4.1.0 経由 + pelway-auth の dev 依存）と rsa 0.10.0-rc.19（crypto-glue 0.2.2 経由）。修正は rsa >= 0.10.0（正式版）だが、crypto-glue 0.2.2 は rsa 0.10.0 正式版に削除された feature `sha2` を要求していて解決不能、openidconnect 4.1.0 は rsa ^0.9、pelway-auth は webauthn-rs/openidconnect をピン留め。semver 範囲内の更新では直らない | ignore エントリ（理由 + フォロー Issue） |
| unmaintained 5 件 | RUSTSEC-2026-0441 pin-utils 0.1.1（slint/spin_on 経由）、RUSTSEC-2026-0192 ttf-parser 0.25.1（slint 経由）、RUSTSEC-2026-0370 zvariant 5.15.1、RUSTSEC-2026-0371 zbus_names 4.3.5、RUSTSEC-2026-0372 zvariant_derive 5.15.1（zbus/atspi 経由。slint クライアントスタック） | `unmaintained = "none"` で失敗させず、ignore エントリに台帳として理由 + フォロー Issue を添える |
| wildcard 5 件（bans、失敗） | 自ワークスペースの path 依存に version 無し: pelway-client-core（2 件）、pelway-client-ui、pelway-db、pelway-http（`pelway-contract = { path = "../pelway-contract" }` など） | path 依存に `version = "0.0.0"` を追加して解消 |
| 重複バージョン 60 件（bans、警告） | rsa 2 版、rand、base64、digest など。multiple-versions は warn なので検査は通る | この PR では対応しない（slint/webauthn 側の更新待ち。必要なら個別にフォロー Issue） |

## 実装者メモ（round 1）

1. 詰まった箇所: 手順4（ライセンス棚卸し）: `cargo deny list` の評価グラフ（764 クレート）と `Cargo.lock`（835 件）が一致せず、対象範囲の確認に手間取った（差はターゲット固有・feature-gated・dev-only の 72 件。棚卸しは検査対象のグラフを基準にした）。同じく手順4: `cargo update -p rsa@0.10.0-rc.19 --precise 0.10.0` が `package crypto-glue depends on rsa with feature sha2 but rsa does not have that feature` で失敗し、rsa の advisory を semver 内の更新で直せないことが確定した。
2. 曖昧だった文: 「`deny.toml` で、unmaintained と unsound は警告にとどまる」（Issue #168 受け入れ条件）— cargo-deny 0.20.2 の `[advisories]` に lint level の warn はなく、`unmaintained` / `unsound` は `all` / `workspace` / `transitive` / `none`（v2 で `vulnerability` 等の lint level フィールドは廃止）。
3. 仮定と解釈: 「警告にとどまる」は「検査が失敗しない」として `unmaintained = "none"` / `unsound = "none"` で実装する（新しい unmaintained で CI を落とさないことを優先）。選ばなかった解釈: 既定（all=エラー）+ 既知分を ignore 登録（新規の unmaintained で CI が落ちる）。
4. 足りなかった情報・検証手段: unmaintained を `none` にした際の出力喪失を補う恒常的な可視化手段（0.20.2 には無い。deny.toml の ignore 台帳とフォロー Issue で代用）。yanked=deny の実機再現は、cargo が新規の解決で yanked 版を拒否するため「lock 後に yank される」という実際の事故形態でしか起きない。
5. 改善案: `write-issue` skill の「受け入れ条件の書き方」に「検査ツールの設定値を条件に書くときは、その版の設定スキーマに存在する値で書く（例: cargo-deny 0.20.2 の advisories に warn は無い、は事前に確かめられる）」という一文を足す。

## round 2: 実装（hatai の決定を受けて）

### 経緯

1. Issue #168 の本文が書き換えられ、未決事項が解決した: MPL-2.0 は allow リストに入れる（webauthn-rs 系・riverqueue 系。改変せずに使う）、Slint は `LicenseRef-Slint-Royalty-free-2.0` を allow し GPL-3.0 は入れない、`dev-` の開発用クレートもライセンス検査の対象に含める。
2. フォロー Issue を起票した: #228（RUSTSEC-2023-0071 rsa の解消）、#229（slint 系 unmaintained 5 件の解消）。deny.toml の ignore 台帳に番号を添えた。
3. wildcards 違反（version なしの path 依存 5 件: pelway-client-core ×2、pelway-client-ui、pelway-db、pelway-http）は path 依存に `version = "0.0.0"` を追加して解消した（repo 内の path 依存はこの 5 件のみで全件対応。cargo-deny の `allow-wildcard-paths` による除外は使わない: Issue の規模見積もりが「Cargo.toml の版の変更」を想定しており、`*` 版の混入だけを失敗させ続けられる方を取った）。
4. `deny.toml` を作成し、`cargo deny check`（advisories / licenses / bans / sources 全部）が通ることを確認した。
5. `mise.toml` に `deny-check` タスク（`cargo deny check`）を追加し、`verify-static` の依存に加えた。CI の static ジョブ（`mise run verify-static` を実行）から自動的に走る。workflow の表示名 `static (hk / fnox / pitchfork)` は対象外のため変えていない（docs/dev-environment.md に明記）。
6. `docs/dev-environment.md` に cargo-deny の節（検査内容・例外の足し方）と、ツール構成・タスク一覧・CI ジョブ構成の各行を追加・更新した。

### 確かめ方（受け入れ条件の検証）

- **vulnerability が 1 件でもあれば失敗**: RUSTSEC-2023-0071 の ignore エントリだけを除いた deny.toml で `cargo deny check advisories` を実行 → `error[vulnerability]` 2 件（rsa 0.9.10 と 0.10.0-rc.19）で exit 1・`advisories FAILED`。ignore を戻すと `advisories ok`（該当 advisory は `note[advisory-ignored]` として出力）。ほかに実在の脆弱性が無いため、ignore の有無での比較を確かめ方とした。
- **yanked**: `yanked = "deny"` を deny.toml に明示（0.20.2 の既定は `warn`）。実機再現は「lock 後に yank される」という事故形態でしか起きない（cargo は新規の解決で yanked 版を拒否するため、意図的な指定が作れない）。
- **verify-static が cargo deny check を実行**: タスク追加前は hk-check / fnox-check / pitchfork-check のみ。追加後は `[deny-check] $ cargo deny check` と `advisories ok, bans ok, licenses ok, sources ok` を含めて exit 0。
- **allow リストと棚卸しの一致**: 上の棚卸し表のうち、OR 式の代替分岐（GPL-3.0-only、GPL-2.0-only、LGPL-2.1-or-later、CC0-1.0、LicenseRef-Slint-Software-3.0）を除いた 14 ライセンス + license 宣言なし 13 件（`[licenses.private]` で対象外）で `licenses ok`。

### 例外の一覧（deny.toml に登録済みのもの）

| エントリ | 種類 | 理由 | フォロー Issue |
|---|---|---|---|
| RUSTSEC-2023-0071（rsa 0.9.10 / 0.10.0-rc.19） | ignore（vulnerability） | rsa >= 0.10.0 正式版への更新が crypto-glue / webauthn-rs / openidconnect の対応待ち（crypto-glue 0.2.2 は 0.10.0 正式版で削除された feature `sha2` を要求） | #228 |
| RUSTSEC-2026-0441（pin-utils 0.1.1） | ignore（unmaintained、台帳） | slint の spin_on 経由。slint の更新で解消 | #229 |
| RUSTSEC-2026-0192（ttf-parser 0.25.1） | ignore（unmaintained、台帳） | slint のテキスト描画。slint の更新で解消 | #229 |
| RUSTSEC-2026-0370（zvariant 5.15.1） | ignore（unmaintained、台帳） | slint の zbus/atspi 経由。slint の更新で解消 | #229 |
| RUSTSEC-2026-0371（zbus_names 4.3.5） | ignore（unmaintained、台帳） | 同上 | #229 |
| RUSTSEC-2026-0372（zvariant_derive 5.15.1） | ignore（unmaintained、台帳） | 同上 | #229 |

ライセンスの例外（`[licenses.exceptions]`）は使っていない。重複バージョン 60 件（bans multiple-versions、警告）は意図的に例外登録せず警告のまま（上流の更新待ち。減ったら自動的に警告も減る）。

## 実装者メモ（round 2）

1. 詰まった箇所: 手順4（deny.toml 作成）: `unmaintained = "none"` では ignore 台帳の unmaintained 系エントリが検査に現れず「未使用」扱いになるため、`unused-ignored-advisory` の挙動（既定 warn だと毎回警告が出る）の確認が必要だった → `allow` にして台帳運用とした。手順4（wildcards 対応）: `allow-wildcard-paths`（path 依存のワイルドカードを除外）と path 依存への version 追加のどちらにするか判断が要った → version 追加を採った（Issue の規模見積もりが「Cargo.toml の版の変更」を想定、`*` 指定の検出はそのまま残る）。
2. 曖昧だった文: 「MPL-2.0 と LicenseRef-Slint-Royalty-free-2.0 以外のコピーレフト系・独自ライセンス（GPL・LGPL・AGPL・EPL・CDDL など）を**単独で要求する**クレートがあれば、allow リストに入れず…止まる」（Issue #168 受け入れ条件、round 2 版）— 「単独で要求する」を「OR 式の代替分岐なしで要求する」と解釈した。
3. 仮定と解釈: 現在の依存ツリーに「単独で要求する」該当は無い（self_cell は Apache-2.0 分岐、r-efi は MIT/Apache-2.0 分岐、imgref は Apache-2.0 分岐で満たせる）ため止まらず、CDLA-Permissive-2.0（webpki-roots、単独）はコピーレフト・独自ライセンスの列挙に入らないパーミッシブなデータライセンスとして allow に入れた。選ばなかった解釈: CDLA も「独自ライセンス」に含めて止まる（ライセンス本文上パーミッシブであり、Issue の列挙の意図はコピーレフト系と読んだ）。
4. 足りなかった情報・検証手段: `LicenseRef-Slint-Royalty-free-2.0` の allow リストでの受理は 0.20.2 での動作確認が必要だった（結果: 受理され `licenses ok`）。フォロー Issue の起票様式（write-issue の見出し・ラベル）を implement-issue の手順に明記の上で作った（needs-triage を付けて人間の確認待ちにした）。
5. 改善案: `implement-issue` skill の「よくある失敗」に「フォロー Issue を作るときは write-issue の見出し形式（目的・受け入れ条件…）で作り、needs-triage を付ける」を足す。
