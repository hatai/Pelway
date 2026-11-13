# Issue #230 の作業ログ

## 実装者メモ（round 1）

1. 詰まった箇所: 手順 4（カバレッジ計測）: `cargo llvm-cov report --show-missing-lines` が `--no-cfg-coverage` を受け付けず、lcov の `DA:` に 0 カウント行が出ないため未カバー行が一覧できなかった。llvm-cov の LF/LH は関数（クロージャ）単位の行統計を instantiation group で merge した値で、lcov の DA とは別計算だった。lcov の `FN:/FNDA:0` から未実行クロージャを抽出したら未カバー 10 行と 1:1 で対応した。
2. 曖昧だった文: 「`pelway-auth` の未カバー行が 14 行以下になる → `mise exec -- cargo llvm-cov -p pelway-auth --locked --no-cfg-coverage --lcov --output-path lcov.info` の `pelway-auth` の LF と LH」（Issue #230 の受け入れ条件）: `pelway-auth` が src 4 ファイルだけか tests を含むか不明だった。PR #162 の 448/474 は src 4 ファイルの合計だったので、同じ src 4 ファイル（lib/oidc/passkey/session.rs）の LF/LH 合計で判定することにした（tests/auth_flow.rs のブロックは数に入れない）。
3. 仮定と解釈: 実装変更禁止（対象外）なのでテスト側からのみ到達する。`AuthState` は `Arc` のため直接書き換えられないが、router を drop してから `Arc::get_mut` で `oidc.tenants` を書き換え、router を作り直せば公開 API だけで `tenant_removed` / callback の `bad_tenant_urls` を再現できると仮定した（実際に成立した）。選ばなかった解釈: この 2 経路を「到達不能」として諦め PR 本文に列挙する案。
4. 足りなかった情報・検証手段: 未カバー行の一次情報。`DA:` では取れないため、`FN:`/`FNDA:`（未実行関数の開始行）との照合と、webauthn-rs-core 0.6.2 の `new_challenge_register_builder`（空名で `InvalidUsername`）/`new_challenge_authenticate_builder`（空リストのみ `CredentialNotFound`）の実装読みで到達可能性を確かめた。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「手順 4」に「カバレッジ系 Issue では、受け入れ条件のコマンドとその失敗出力（しきい値超過）を『失敗するテスト』として先に記録する」を足す。
