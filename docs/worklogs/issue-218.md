# Issue #218 の作業ログ

## 実装者メモ（round 1）

対象 head: 09d832ed28902d8d5569dc32405102a34da1d874

1. 詰まった箇所: mise run verify: セッション環境の /etc/resolv.conf が /mnt/wsl への切れた symlink で DNS が解けず、mise の rust 1.99.1 インストールと mbx のロック取得が失敗した。DNS 修正済み /etc コピーに差し替えた bwrap 内で XDG_CACHE_HOME/CARGO_TARGET_DIR/RUSTUP_TOOLCHAIN を指定して実行し、fmt・clippy・test・deny・hk・toolchain は全部緑。script-test のみ agent-sandbox.test.sh が失敗（$HOME/.t3/worktrees/sbx-plug-test を worktree 外に作れず、セッション sandbox の書き込み制限による環境起因。docs 2 ファイルの差分には影響しない）
2. 曖昧だった文: 「llm-eval の文書がある場合は矛盾しないよう参照を合わせる」（Issue #218 本文・背景・前提）— 「参照を合わせる」が文書側を AGENTS.md 向けに書き換えることか、AGENTS.md 側を文書に合わせることか不明だった
3. 仮定と解釈: 仮定: 正本は AGENTS.md。解釈: evals/agent-team/README.md 30 行目の「LLM ジャッジの規則（別 Issue）に従う」を「AGENTS.md の『LLM judges』節に従う」に書き換える方向（本 Issue がその規則を正本に定める Issue であるため）。選ばなかった: AGENTS.md 側に README の手順を取り込む形
4. 足りなかった情報・検証手段: Rakazo の AGENTS.md 本文（関連情報に repo 名のみ）。Issue 本文の要約（ルーブリック版管理・較正・採点禁止・コスト別建て・エラー時判定不能）をそのまま規則の要素として使い、要約に無い細部は追加していない
5. 改善案: .agents/skills/write-issue/SKILL.md の「関連情報」に「外部文書は要約の要素を本文に列挙する（正本に到達できない実装者でも要素の漏れなく扱える）」を足す

使ったスキル: implement-issue（受け入れ条件の確認は grep ベースで RED→GREEN。seam なし）
