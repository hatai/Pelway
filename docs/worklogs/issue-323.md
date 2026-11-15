# Issue #323 の作業ログ

## 実装者メモ（round 1）

対象 head: af65ab78cafe256b55782a99cae63f9dc3ba6ccd

1. 詰まった箇所: scripts/agent-gh: この Sandbox 環境は /etc/resolv.conf が無く DNS が引けず、また gh に認証が無いため、トークン発行前に必ず失敗した。`unshare -rm` で /etc/resolv.conf を bind mount し GH_TOKEN=dummy を渡すラッパー経由で実行して回避した。あわせて ~/.cache（mbx・hk のキャッシュ）が読み取り専用のため、cargo は rustup 直のバイナリ＋CARGO_TARGET_DIR=target-ci で実行した（hk の pre-commit フックにも同じ env を渡した）
2. 曖昧だった文: 「過去の記録の書き換え: docs/worklogs/、docs/technical/v0.7/sources/（v0.6 の固定ソースや追補のログ等の保存資料）、docs/planning/input/ 等の入力素材」（Issue #323 対象外）
3. 仮定と解釈: 「等の入力素材」に docs/planning/short-term-input/technical.md と docs/design/brief-2026-11-10/（指示パック。source-of-truth.md 自身が「指示パックであり変更していない」と明記）も含むと解釈し、両方は対象外とした。選ばなかった解釈: 背景 の現行文書列挙に docs/design/ があるため brief 配下も書き換える。tokens JSON は Issue の出現箇所調査に明記されているため対象とし、theme_tokens.rs の sha256 定数と「97fb70d の写し」注釈を書き換え後の正本に合わせた
4. 足りなかった情報・検証手段: スクリーンショットの出し方。Slint は i-slint-backend-testing（窓を開かない）で、画面描画のピクセル取得手段がこの repo に無い。テスト用 backend での文言プロパティ検査（i18n_fluent.rs に追加した ja_labels_call_the_ai_ai_bot）と、GroupLabel が幅固定の Text でないこと（ui/main.slint:184）で「崩れていない」ことを代用した。また scripts/tests/agent-sandbox.test.sh は worktree 外への書き込みを要するためこの Sandbox では失敗する（差分と無関係。CI で通る）
5. 改善案: docs/dev-environment.md の「ネットワーク制限」に「DNS が引けない環境では /etc/resolv.conf の欠落を疑う」を足す。加える文: 「gh のトークン発行に失敗するときは resolv.conf の有無と gh extension Link-/gh-token のインストールを先に確認する」

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: （push 後に記入）

1. 詰まった箇所: なし（レビュー指摘への修正のみ）
2. 曖昧だった文: 「機械的な一括置換はせず、出現箇所ごとに対象範囲かどうかと自社・他社の別を確認してから置換する」（Issue #323 採用する方式）
3. 仮定と解釈: 「AI社員」の部分文字列で grep するだけでは「専門の社員」「社員の作業場」のような単独の「社員」は拾えない。指摘の 2 箇所を修正し、対象範囲の全ファイルを `git grep 社員` で確認して残りが検査コード自身のみであることを確かめた
4. 足りなかった情報・検証手段: なし
5. 改善案: docs/prd.md「AIらしさの方針」の表に「AI の呼称は AI Bot を使う（単独の『社員』も旧称の一部とみなす）」を足す

使ったスキル: implement-issue
