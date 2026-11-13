# Issue #97 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: openidconnect 4.1 の `AsyncHttpClient` は高階トレイト（`for<'a>`）で、axum ハンドラ内の「1 回の呼び出しにだけ生きるクロージャ」を渡すと `authorize_url`・`exchange_code` が axum の handler 境界を越えられず HRTB エラーになる。oauth2 crate 内部の reqwest 実装と同じ「名前付きクライアント型 + 手動 `AsyncHttpClient` impl」に変えて解消した。ほか: webauthn-rs の RP 設定は `UserVerificationPolicy::Required` を要求するため、テスト用ソフト認証器は `SoftPasskey::new(true)`（true = UV を満たす側）が必要で、`new(false)` では `NotSupported` で失敗する。`IdTokenClaims` の nonce 設定は `StandardClaims` ではなく構築後の `IdTokenClaims` に対して `.set_nonce` を呼ぶ。`CoreRsaPrivateSigningKey::as_verification_key` は `PrivateSigningKey` trait の import が必要。token endpoint の型は `Option<TokenUrl>` なので `Some(..)` で包む。
2. 曖昧だった文: 「passkeyと顧客のIdPでのログインをaxumに組み込めるか確かめる」（Issue #97 題名・やること）。「確かめる」の成果物が調査文書なのか実行可能コードなのか、組み込み先の axum サービスは新規か既存 `pelway-http` への追加かが書かれていない。session・device・revocation の「12章との照合」も、どこまで動くコードにするかが未定義。
3. 仮定と解釈: #92 の調査と 12章「自動的に新サービスを追加しない」方針から、成果物は新規 crate `pelway-auth` としての実行可能な spike + テストと解釈した（issue #94・#96 と同じく crate が成果物になる型）。組み込み先は `auth_router(state) -> Router` を返す薄い axum 部品とし、`pelway-http` 本体への接続はこの Issue では行わない。OIDC は Google `hd` / Entra `tid` 相当の追加 claim を `AdditionalClaims` で表現し、tenant 側の許可ドメインと照合してから session を発行する。WebAuthn の登録・認証 state は webauthn-rs が要求する通りサーバー側に保持する。選ばなかった解釈: 調査文書のみを置く、`pelway-http` へ直接ルートを追加する。
4. 足りなかった情報・検証手段: 実際の Google Workspace / Entra ID テナントは無く、本物の IdP レスポンス形式（追加 claim の正確なキー名・有無）は公式ドキュメントと自前の偽 IdP でしか確認できない。JWKS のローテーション・キャッシュ失効動作も偽 IdP では再現していない。`SoftPasskey` は UV・resident key の振る舞いをソフト的に「装う」だけで、実ブラウザ・実認証器での動作保証は別途必要。
5. 改善案: docs/agents/issue-tracker.md の「検証系 Issue には成果物の置き場所と検証深度を書く」に、「axum 組み込み可否の検証ではルート提供の薄い crate を作るか既存サービスへ組み込むかを明記する」と足す。

## 実装者メモ（round 2）

1. 詰まった箇所: 手順6（git reset --soft で分割）: `origin/main` が分割判断の間に進んでおり、旧 main 上の統合コミットを `reset --soft` で PR-A に積むと、新しい main が入れた変更を巻き戻す差分が混ざった。`reset --hard` で PR-A に揃えてから `git checkout <統合コミット> -- crates/pelway-auth docs/worklogs/` で認証分だけ取り直して解消。ほか: hk の pre-commit は mise 環境外だと rustup 既定の cargo 1.94.1 を拾い riverqueue 0.6.0 の rust-version 検査で落ちるため、コミットは `mise exec -- scripts/agent-gh dev git commit` が必要（指揮役指示どおり mbx の成果物は権限を変えず worktree の target 内で stale 分だけ消す）。
2. 曖昧だった文: 「PR B は PR A のブランチに積む」（指揮役の t3 thread send）。GitHub の base を PR-A ブランチにするのか、main 向けのまま commit だけ積むのかが書かれていない。
3. 仮定と解釈: PR-B の GitHub base を PR-A のブランチ（agent/97-passkey-login）にした積み上げ PR と解釈した（issue-96 の #129/#130 と同じ形。base 行には base ブランチ名と PR 番号を併記）。分割線は指揮役承認のコメント案どおり session+passkey / OIDC+worklog とし、`session::AuthMethod::Oidc` 変種は pub のため警告にならず B 側の受け口として A に残した。選ばなかった解釈: base=main の独立 2 PR（同じ差分が二重に載る）。
4. 足りなかった情報・検証手段: PR-A（+798/-0）は上限 800 に 2 行の余裕しかなく、レビュー指摘で数行足すだけで超過しうる。超過時の扱い（例外ラベル or 再度分割）はコメント欄で指揮役判断を仰ぐしかない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「止まる条件」の表に「分割が承認された積み上げ PR では base を親 PR のブランチにし、pr-size は --base <親ブランチ> で測る」を足す（issue-96 で scripts/pr-size.sh --base の使い方だけはログに残ったが skill 本文には無い）。

## 実装者メモ（round 3）

1. 詰まった箇所: PR #145 のレビュー must-fix（passkey.rs `revoke_credential` が未認証 JSON の `member_id`/`credential_id` で credential 削除と全セッション失効ができた）。直すと行数が増えるが PR-A は +798/800 で余力が無かったため、passkey.rs 内で 10 箇所に散らばっていた `.lock().unwrap_or_else(|p| p.into_inner())` を `lock` ヘルパーに集約し（重複の除去、-38 行）、修正分を吸収して +780/-0 に収めた。ほか: 一時 `cargo` が /usr/bin の rustc 1.75.0 を拾い thiserror の rust-version 検査で落ちる現象があった（再実行で mise 側 1.96.0 に戻った。hook も同じ原因と思われ `mise exec` 経由が安定）。
2. 曖昧だった文: 「PR #146 は PR #145 のブランチに積んであるので、直したら PR #146 のブランチにも取り込む」（指揮役の t3 thread send）。取り込みが rebase か merge か書かれていない。
3. 仮定と解釈: rebase だと force-push になるため、`git merge agent/97-passkey-login` で取り込むと解釈した（main にも `Merge branch 'main' into agent/...` の履歴があり、このリポジトリは merge 取り込みの運用）。must-fix は「for a self-service endpoint derive the member ID from the session」の文言どおり、member をセッション Cookie から特定する本人向け経路にした。member 単位のセッション全失効は従来どおり（呼び出し元のセッションも消える）。選ばなかった解釈: `member_id` を body に残したまま管理者権限チェックを足す（管理者モデルが spike に無い）。
4. 足りなかった情報・検証手段: 失効した端末・管理者による他人の credential 取消の正規経路は Issue・12 章ともに未定義で、ハンドラ文書に「本番で別経路を検討」と残すに留めた。worklog は PR-B 側のコミットにのみ置く分割のため、PR-A の修正コミット自体にはメモ更新を含められない。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「手順 6」に「積み上げ PR の下流ブランチへの取り込みは merge で行う（rebase は force-push が要る）」を足す。

## 実装者メモ（round 4）

1. 詰まった箇所: PR #146 のレビュー must-fix（oidc.rs `allowed_hd`/`allowed_tid` が両方空だと任意の IdP アカウントでセッション発行）。`OidcStore::new` を `Result` 返しに変えるとテスト 2 箇所が変わる程度で波及は小さかった。ほか: 空リスト拒否の検証は偽 IdP 不用で `OidcStore::new` 単体で確かめられる普通の `#[test]` にした（`#[tokio::test]` 不要）。
2. 曖昧だった文: 「少なくとも許可リスト未設定を設定エラーとして拒否してください（対象の provider claim も必須にする）」（PR #146 レビュー must-fix）。「対象の provider」が Google/Entra のどちらかを tenant 設定側で識別する型を求めているのか、claim 照合の必須化だけでよいのかが書かれていない。
3. 仮定と解釈: provider 種別を tenant 設定に新設せず、「非空の許可リストに対応する claim の一致を必須、両方空は `OidcStore::new` で `TenantConfigError`」と解釈した（既存の `is_some_and` 照合は非空リストで claim 必須を既に満たす。Google 向けに `allowed_tid` だけ登録する誤設定は tid 不在で拒否側に倒れる）。選ばなかった解釈: `provider` 列挙型を `OidcTenant` に追加し対応 claim だけを見る（spike の段階では型を増やさない判断）。
4. 足りなかった情報・検証手段: 「設定エラー」の発生箇所が登録時かログイン時かの明記なし。fail fast のため `OidcStore::new`（登録時）で拒否したが、ログイン時 500 で返す設計もありえた。
5. 改善案: `docs/agents/triage-labels.md` またはレビュー指針に「安全でない既定値の指摘は、拒否すべき層（登録時/起動時/要求時）を書く」例を足す。

## 12 章との対照表（セッション・端末の認証・取り消し）

対象: `docs/technical/v0.7/body/12-communication-secrecy.md`。範囲は本スパイク（passkey + OIDC + セッション）に関係する要件に絞る。

| 12 章の該当要件 | 今回の実装での扱い | 未対応・未検証 |
|---|---|---|
| P07（§3）認証と能力確認: 接続に必要な項目と宛先だけに限定 | 応答は `pending_id`・challenge options・`member_id` のみ。セッション Cookie・JSON に認証情報の平文を出さない。OIDC の scope は `email` のみを追加要求 | member 属性（email 等）の保存・利用は本番の目的別分類（P表）で要確認 |
| P07（§3）: 資格情報取消で通常の利用を停止 | credential 取消 → `SessionStore::revoke_member` で member の全セッションを削除（テストで取消後 401 を確認）。OIDC 側は tenant 設定の削除で同等に可能 | credential 単位のセッション特定（その端末だけ止める）と、失効端末・管理者による取消の正規経路は未実装（ハンドラ文書に記録） |
| §3.3 P06/P07 行: 認証通信と操作データを分け | 認証は `/auth/*` に集約し、セッションは不透明 `Uuid` のみを運ぶ。member 情報はサーバー側 `Session` にだけ置く | `pelway-http` 本体との結合と、操作データ側の分離は未接続 |
| §4: 認可は server で強制（UI で隠すだけにしない） | `whoami`・`revoke_credential` はサーバー側の `SessionStore`/tenant 許可リストで判定。member はセッション Cookie から特定（#145 のレビュー指摘を修正済み） | 管理者・監査用の認可モデルは spike に無い |
| §5: 失効が判明した経路は新規実行しない | 消費済みの pending（passkey セレモニー・OIDC `state`）は one-shot で削除し再送を拒否。取消後のセッションは `lookup` に出ない | pending/セッションの期限切れ・永続化は未実装（メモリのみ） |
| §6 / §9 S20: 台帳・証拠不足は既定拒否 | 許可リスト両方空の tenant は `OidcStore::new` が `TenantConfigError` で拒否（#146 レビュー指摘を修正済み）。unknown tenant/state/credential/セッションは 400/401/403/404 で拒否 | 認可台帳の永続ストア化は本番課題 |
| §7: server が平文を扱う信頼モデルを明示 | crate 文書（lib.rs）に「secret・ID トークンはサーバーが平文で扱う」と明記 | host/DB 管理者の役割分離、鍵へのアクセスとデータアクセスの分離は本番検討 |
| §7: 連携の認証 token と保存時暗号鍵は別管理 | セッション トークンは不透明 ID で暗号鍵を兼ねない。IdP 側の署名は検証のみで Pelway 側の署名鍵は持たない | 保存時暗号化の方式は 12 章の未決（v0.6/08 の未決解消待ち）どおり未対応 |
| §7: 外部の権限管理サービスを必須にしない | Ory/Kanidm 等の IdP 製品を足さず、`webauthn-rs`・`openidconnect` の RP だけを axum に組み込んだ | — |
| 端末の認証（credential 単位の識別） | passkey は `credential_id` を `AuthMethod::Passkey` に保持し、どの端末でログインしたかの監査手がかりにする。credential 取消も credential 単位 | OIDC ログインは端末に紐付かない（tenant+subject のみ）。credential counter の更新・clone 検出は未対応 |
| §9 S17: log・記録に秘密値を出さない | エラー応答は `{"error":"<code>"}` のみ。ログ出力は実装していない | 構造化ログ・監査イベントとの連携は未実装 |

## 実装者メモ（round 5）

1. 詰まった箇所: PR #146 のレビュー must-fix（split-plan の受け入れ条件「セッション・端末の認証・取り消しを 12 章の要件と照らした表」が作業記録に無い）。crate 文書（lib.rs）には簡易の照合表があったが worklog には無かったため、要件ごとの扱い・未対応を対照表として追記した。
2. 曖昧だった文: 「セッション・端末の認証・取り消しを 12 章の要件と照らした表」（Issue #97 本文の「やること」）。表の置き場所（worklog・crate 文書・別文書のどれか）と対象範囲・粒度が書かれていない。
3. 仮定と解釈: レビューが `docs/worklogs/issue-97.md` を指名したため worklog に対照表を置くと解釈した。対象要件は P07（限定・取消）・§3.3（認証通信と操作データの分離）・§4（server 側認可）・§5（失効経路の未開始抑止）・§6/§9 S20（既定拒否）・§7（信頼モデル・鍵分離・外部権限管理不要）・§9 S17（記録への秘密値非含有）と、端末単位の識別（credential_id）に絞った。選ばなかった解釈: docs/technical 配下に別文書を作る。
4. 足りなかった情報・検証手段: 12 章は認証そのものの詳細設計を持たず、P07 の行と §5〜§7 の断片的記述から該当要件を拾うしかなかった。端末認証の粒度（credential 単位のセッション特定まで要るか）は章に明記が無く、未対応として表に記した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「手順 5」に「Issue が照合表・比較表を求める場合、実装の crate 文書に加えて `docs/worklogs/issue-<N>.md` にも置く」を足す。
