# Issue #202 の作業ログ

## 実装者メモ（round 1）

対象 head: aa33fcbcb98f16d278d960a59702067dcb93b3e4

1. 詰まった箇所: 手順 6 の `mise run verify`: `scripts/tests/conductor-watch-ensure.test.sh` の "ensure restarts watch" がこの環境で失敗する。`git stash -u` で自分の変更を退避した main と同一のツリーでも同じ失敗になり、main の HEAD (b452cce) の CI は `mise run verify: success`。本 PR の変更（docs 2 ファイル＋テスト 1 追加）はこのテストの対象に触れないため環境依存の揺らぎと判断し、PR の CI で改めて確認する（PR 本文に証拠を記録）。
2. 曖昧だった文: 「本番トポロジの構成要素…と配置が文書化されている → 技術仕様または運用文書の該当箇所を目視で確認」（Issue #202 本文の受け入れ条件）。「技術仕様または運用文書」のどちらへ書くかが決まっていない。
3. 仮定と解釈: 決定の記録はこのリポジトリの流儀に従い ADR 0012 として書いた（ADR 0011 と同じ形: ADR＋構造テスト scripts/tests/production-topology-docs.test.sh）。技術仕様 v0.7 の body への新章追加（統合版で章構成が固定）と docs/ 直下への新規運用文書（ADR と二重管理になる）は選ばなかった。発見性のため docs/technical.md の「詳細と根拠」へ ADR 0012 への参照を 1 行足した。
4. 足りなかった情報・検証手段: 関連情報の Rakazo の URL が infra/compose・infra/systemd のディレクトリを指すのみで、雛形として採る個々のファイルが分からない。`scripts/agent-gh dev api repos/elie222/rakazo/contents/...` で docker-compose.prod.yml・Caddyfile.prod・harden-host.sh・timer ユニットの中身を確認してから翻案した。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「決まり（毎回守る）」に「verify の失敗が自分の変更と無関係な既存テストの揺らぎ（main と同一ツリーでも失敗し、main の CI は成功する）のときは、その証拠を PR 本文に記録して PR の CI で確認し、進んでよい」を足す。

使ったスキル: implement-issue

## 実装者メモ（round 2）

対象 head: 5688c000b66f2a443dd9b3c50fbb755ca0b7ff63

1. 詰まった箇所: `mise run verify`: round 1 と同じ `scripts/tests/conductor-watch-ensure.test.sh` の環境依存の揺らぎのみで失敗（他 27 系列は all passed。PR の CI では round 1 で通過済み）。
2. 曖昧だった文: 「sandbox controller | 信頼するホスト/コンテナランタイム側の限定API」（docs/technical/v0.7/body/01-architecture.md「ホストとコンテナ」）。この「信頼側」の記述から controller を `sandbox` ネットワークのみに置く配置を書いたが、同章のフロー図 `RUN --> CTRL` と噛み合わず、レビューの must-fix になった。
3. 仮定と解釈: must-fix の提示した修正（controller は `app` と `sandbox` の両方、task sandbox は `sandbox` のみ・`data` 非参加）を採った。選ばなかった解釈: dispatch 専用の別ネットワークを足す形（レビューが「別の形にするなら明記」と認めた代替）。v0.7 05 の `app`/`sandbox` 2網分離の枠内で収まり、図の辺をそのまま満たす前者が最小の変更のため。
4. 足りなかった情報・検証手段: 配置表を書いた時点で正本の図の辺（`RUN --> CTRL`）を表の各行の到達性として突き合わせていなかった。修正時に 01 のフロー図の本体内部の辺（API→DB、RUN→CTRL、CTRL→SB 等）を表中の配置で確認し直した。
5. 改善案: .agents/skills/implement-issue/SKILL.md の「よくある失敗」に「設計文書で構成要素の配置を書くとき、参照した正本の構成図の辺を 1 つずつ表中の配置で到達できるか突し合わせずに書く」を足す。

使ったスキル: implement-issue
