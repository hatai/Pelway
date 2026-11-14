# Issue #296 の作業ログ

## 実装者メモ（round 1）

対象 head: ae515ee06896b5a32fd19b5e86b01a89f39736c4（この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: `docs/dev-environment.md`「レビュー担当の Claude」: 確認手順の結果の表が未実施のまま残っていたため、受け入れ条件の「dev-environment 側の記録」の置き場所に迷った。Issue #296 の検証記録は独立した節として追加し、#256 の確認手順の表には触れないことにした（#256 の対象はレビュー用の権限・ネットワーク制限の確認で、本 Issue の圧縮機能の検証とは別物のため）
2. 曖昧だった文: 「使用率が取得できる環境では使用率 70% 以上（代理指標は使わない）」（Issue #296 本文「採用する方式」トリガー）: 「取得できる」の判定主体（エージェント自身か、セッショントランスクリプトの解析か）が書かれていなかった。検証の結果、T3 のスレッド読み取り（`t3_thread_read`）にコンテキスト使用状況は現れず、トランスクリプトからトークン数は読めるが上限値が記録されないため、使用率は算出できないと解釈した（選ばなかった解釈: モデル上限の既知値を外から与えてトークン数から算出すれば「取得できる」とする。上限の変動に追従できず、1 ターン遅れにもなるため避けた）
3. 仮定と解釈: 「dev-environment 側の記録」は `docs/dev-environment.md` と仮定した。「command-team §5 が恒久交代に適用できるかを確認し、不足する場合はこの手順を正本に置く」に対しては、§5（代行）は一時代行（正本の指揮役へ戻す前提）で、操作権限の移譲点・確認失敗時の権限不移転・同時稼働の禁止の明示が不足すると判断し、恒久交代の手順を「7. コンテキスト管理」節の正本に置いた。検証は `delegate_task` の子（claude_review、claude-opus-5-5）への報告依頼と、私の側からの `t3_thread_send` での `/compact` 送信で行った
4. 足りなかった情報・検証手段: 自動圧縮（`trigger: auto`）の実例を確認する手段が無かった（意図的に大量のコンテキストを消費する必要があり、実施しなかった）。`docs/dev-environment.md`「レビュー担当の Claude」に書かれた `~/.t3/userdata/settings.json` の `providerInstances.claude_review` はこの環境には存在せず（`~/.t3/caches/claude_review.json` に状態があった）、設定の実体がどこで管理されているかは確認できなかった
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「6. 検査してコミットし、行数を確かめてから push する」に「実装者メモの対象 head はメモコミットの親（実装の実体を含むコミット）を指すため、メモを独立したコミットにしてよい」を足す（Issue #290 round 1 の改善案と同旨で未反映のため、今回も 2 コミット構成にした）

使ったスキル: implement-issue（割り当て行に試行の明示が無いため輸入スキル `tdd` は使わず、手順 4 のテストは `scripts/tests/board-patrol-docs.test.sh` と同じ grep パターンで新規作成した）、write-pull-request

## 実装者メモ（round 2）

対象 head: 473c455f603e17c9b11711be847565eada086cc7（round 2 の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: `scripts/agent-gh`: 検証中にこのセッションで DNS が壊れた（`/etc/resolv.conf` の symlink 先 `/mnt/wsl/resolv.conf` がこの sandbox のマウント名前空間から消え、`gh token generate` が名前解決に失敗）し agent-gh 経由の GitHub 操作が全滅した。`/` が tmpfs の書き込み可能領域だったため `/mnt/wsl/resolv.conf` を再作成して復旧した（環境固有の一時障害で、repo 側の変更は無し）
2. 曖昧だった文: 「プロバイダごとの差分を調べ、正本化する手順にも差分を明記すること」（指揮役からの修正指示）: 手順の本体（2 経路・外部化チェック・復元検証・照合）をプロバイダごとに分岐して書くのか、手順は共通で差分が現れる箇所だけ明記するのかが決まっていなかった。手順は共通とし、差分が現れる 3 箇所（導入文・トリガーの使用率判定・予期しない圧縮・導入条件）に差分と検証記録への参照を明記する解釈を採った（選ばなかった解釈: 経路ごとにプロバイダ別の手順節を書く。重複が増えて保守が難しくなるため）
3. 仮定と解釈: 「検証子スレッドは必要に応じてプロバイダごとに立ててよい」から、opencode・Devin CLI・Codex の 3 子を `delegate_task` で並列に立てた。Devin CLI の `/compact` は子が自分のスレッドへの `t3_thread_send` で注入・実行まで確認した報告を採用した。opencode の T3 からの `/compact` 送信は未検証として記録した（HTTP API での実行方法が検証済みで、導入条件の阻害にならないと判断）
4. 足りなかった情報・検証手段: どのプロバイダも auto（自動圧縮）の実際の発火は未観測（意図的な大量コンテキスト消費は実施しなかった）。Codex の自動圧縮の有無はセッション内のツール一覧・CLI の確認からは判定不能だった
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「手順 4」に「受け入れ条件が指揮役・人間の指示で途中拡大されたときは、拡大後の受け入れ条件を Issue コメントとして明示してから実装する」を足す（本件は Claude 限定の条件から全プロバイダへ拡大されたが、Issue 本文のチェックボックスは更新されないため）

使ったスキル: implement-issue

## 実装者メモ（round 3）

対象 head: 848261e342d905362d55fcedd74a6d3461fb30b3（main の取り込みと衝突解消のマージコミット。このメモを含むコミットの親）

1. 詰まった箇所: main の取り込み: main 側の #297（agent-sandbox）が worktree の `opencode.json`・`opencode.jsonc`・`.opencode/`・`.devin/` を正規化したのに対し、このセッションの sandbox が同名ファイルを空の読み取り専用バインド（ro-bind）として掴んでいたため、`git merge` も `git checkout` もこれらの書き換えに失敗した。一時 index（`GIT_INDEX_FILE`）で `git read-tree -m` → `git merge-file` → `git commit-tree` → `git update-ref` でマージコミットを作り、差分パスを worktree に反映した（ro-bind の 5 ファイルは index と HEAD が正しい状態で、worktree の実体だけ置けない）
2. 曖昧だった文: 「`mise run verify` が失敗したらコミットしない。直してからやり直す。」（`.agents/skills/implement-issue/SKILL.md` の手順 6）: 失敗がこの PR の差分ではなく環境の制約による場合の扱いが書かれていなかった。verify-lint・verify-static・cargo test（test タスク）・script-test の個別テストのうち `agent-sandbox.test.sh` 以外がすべて通り、失敗が main 由来のテストのファイルシステム要件（`~/.t3/worktrees` の兄弟ディレクトリと `/home` 直下への書き込み）でこのセッションの sandbox では満たせないことを確認した上で、CI の通過をもって検証とする解釈で push した（選ばなかった解釈: テストをスクリプトから外す。この PR の差分と無関係な main 由来のテストを変えない）
3. 仮定と解釈: `agent-sandbox.test.sh` は CI（sandbox 無しのランナー）で動く性質のもので、#297 の CI が main で通過している以上、この PR の差分（docs と script-test への 1 行）はこの失敗に影響しないと仮定した
4. 足りなかった情報・検証手段: このセッション内で CI 相当の検証（sandbox 制約の外で `mise run verify` を回すこと）を実行する手段が無かった
5. 改善案: `docs/dev-environment.md` の「実装担当の sandbox」の「確認手順」に「`agent-sandbox.test.sh` は sandbox 内の実装スレッドでは `~/.t3/worktrees` 兄弟の作成に失敗するため、CI でのみ確認する」を足す

使ったスキル: implement-issue
