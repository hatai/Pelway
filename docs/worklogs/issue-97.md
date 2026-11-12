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
