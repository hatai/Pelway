# Issue #725 の作業ログ

## 実装者メモ（round 1）

対象 head: 56ef448e69a656ce76a60217985474be950252b3

1. 詰まった箇所: 手順 4（受け入れ条件ごとのテスト）: 今回の受け入れ条件は実行時の振る舞いではなく文書どうしの整合なので、先に書く失敗するテストをコードのテストとしては作れなかった。既存の wording pin（文言が文書に存在することだけを確かめる検査）を新しい決まりに合わせて先に失敗させてから直し、あわせて `git grep` で関連ファイルの回数上限の記述を洗い直した。
2. 曖昧だった文: 「確認だけ（別の仕組みで、t629u の対象かを判断する）」（Issue #725「修正ラウンドの上限で止める記述」）
3. 仮定と解釈: `.agents/skills/pr-watch/SKILL.md` の「20 ラウンド」は見張りそのものの実行上限ではなく修正と push の回数上限なので、t629u の対象と解釈した。回数の区切りだけを外し、見張りの実行を切り上げる 2 時間の区切りは残した。round 4 以降の PR は hatai の admin マージにする今の運用を残す解釈を採り、Kodiak のマージに戻す解釈は採らなかった。長時間化の見張りは追加しない: 修正 round ごとのサマリーが PR に残り、終わりの報告にもラウンド数が入るため、Issue や停止報告へ同じ回数を転記する決まりは重複と判断した。
4. 足りなかった情報・検証手段: 文書の意味を判定する LLM judge はまだ無い。今回は更新した wording pin と、関連ファイル間で同じ条件を指す文言の照合で確かめた。`agent-team.toml` のモデル更新に `scripts/tests/agent-team-config.test.sh` の pin が追随しておらず verify が落ちたため、現在の登録値に合わせる小さな修正を足した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「4. 受け入れ条件ごとにテストを先に書く」に「文書の確認条件では、変更後の決まりが関連する手順書どうしで矛盾しないことを確認方法に書く」を足す。

使ったスキル: implement-issue、write-pull-request、writing-for-agents、diagnosing-bugs

## 実装者メモ（round 2）

対象 head: 32ee0359b61c69286b2d9d976136862ab69f6a0a

1. 詰まった箇所: 手順 9（Request changes）: round 1 では、回数上限の一部だった「4 回目にセキュリティの must-fix が出たら止める」行を上限そのものと一緒に消し、Issue の「セキュリティの must-fix の扱いは変えない」を見落とした。
2. 曖昧だった文: 「いままでどおり残す条件（委託の連続 failed、セキュリティの must-fix の扱い、`needs-human` など）は変えない」（Issue #725「直し方」）
3. 仮定と解釈: 「セキュリティの must-fix の扱い」は、round 4 以降でセキュリティ領域の must-fix が出たときに hatai の判断へ戻す停止条件を指すと解釈し、回数上限から切り離して残す。round 4 に進むこと自体は例外にしない解釈は変えない。`automerge` を付ける条件に round の確認を足す指摘は印の無い nit なので、この round では直さない。
4. 足りなかった情報・検証手段: main は `d1c177fa`・`ac5c9fce` で `agent-team-config.test.sh` を同じ内容に直していた。`origin/main` を取り込んでこの PR の差分から外した。`human-actions.md` の行番号は基準コミットの行なので、H11 の参照を基準コミットの値に戻した。
5. 改善案: `.agents/skills/implement-issue/SKILL.md` の「Request changes」の 1 に「既存の停止条件を直す変更では、回数だけの条件と中身による条件を分けて確認する」を足す。

使ったスキル: implement-issue、write-pull-request
