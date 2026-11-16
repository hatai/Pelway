# Issue #198 の作業ログ

## 実装者メモ（round 1）

対象 head: f366a39008d27dd2342f0808c724781529c90ca2（この round の実装の実体を含むコミット。`scripts/agent-gh-resign` による署名付け替え後の SHA。このメモを含むコミットの親）

1. 詰まった箇所: 環境: このスレッドの sandbox は `/etc/resolv.conf` が `/mnt/wsl/resolv.conf` への dangling symlink で GitHub に届かず、`scripts/agent-gh` が `gh token generate` で失敗する。`unshare -rm` の user namespace で `/mnt/wsl` を作って resolv.conf を bind して DNS を復旧した。userns 内は uid=0 のため権限系テスト（`pelway-agent-watch` の `unwritable_state_does_not_repeat_events`）が壊れるので、`deny-check`（RustSec DB の fetch が必要）だけ userns で実行し、それ以外の verify タスクは通常環境に分けた。`$HOME` 直下が読み取り専用の制約（issue-342・352 のメモと同種）には、`MISE_DISABLE_TOOLS=node,rust`＋node を含まない PATH で `agent-sandbox.test.sh` の plugin 節を仕様どおり skip にし、`RUSTUP_TOOLCHAIN`＋toolchain bin で rustup のチャネル同期を避け、`MBX_CACHE_DIR=/tmp/mbx-cache` で mbx を通した
2. 曖昧だった文: 「世代（単調な版数）で失効を表す方式の採否と、その場合の記録形式が推奨として書かれている」（Issue #198 本文「受け入れ条件」）
3. 仮定と解釈: 「世代（単調な版数）」は Buzz の全店舗共有 `protectionClock` と同じ粒度（workspace 全体の共有時計）とは限らず、対象（subject）ごとの単調な版数でよい、と解釈した。共有時計は参照の照合が `(subject, generation)` 単位で閉じるため必須ではなく、対象ごとの時計の方が既存の `epoch`・版・`fence`（いずれも対象単位の単調な値）と一致する。記録形式は「対象行の `generation` 列」＋「参照側が刻む `(subject_id, generation)`」を推奨とした
4. 足りなかった情報・検証手段: 受け入れ条件が「目視で確認」のため、文書の存在と必要節の有無を確かめるスクリプトを作業環境内に作って全 9 項目の PASS を確かめた（リポジトリには残していない）。意味論の正しさはレビュアーの目視に委ねる
5. 改善案: なし

使ったスキル: `.agents/skills/implement-issue`、`.agents/skills/write-pull-request`

## 実装者メモ（round 2）

対象 head: 85e5972d8c197cb6a1ae96e20775fab9d068dfe0（round 2 の実装の実体を含むコミット。push 後の署名付け替え後の SHA。このメモを含むコミットの親）

1. 詰まった箇所: 手順 9（レビュー結果の処理）: round 1 で codex approve と claude request-changes（must-fix 2）が割れ、旧スキル版の「複数担当の判定が割れた」で停止・Issue へエスカレーションした。指揮役（hatai の恒久指示 2026-11-16「要修正と承認でレビュー結果が割れたときは修正を優先して」）が request-changes 側を正として再開を指示し、must-fix 2 件を修正した
2. 曖昧だった文: 「失効台帳（または tombstone 行の集合）にも残す」（自分が書いた `docs/research/revocation-generation-model.md` 記録形式の節）
3. 仮定と解釈: tombstone を同じ DB に置く案を「または」で併記したが、claude の指摘どおり同じ DB 内 tombstone は backup 復旧で台帳ごと巻き戻り保証にならないため、台帳は復旧範囲の外に独立して保管し再開前に最終世代を再適用する形に限定した。再付与の形は「同じ行で generation+1」と「新行」が併存して読み違えられていたため、subject_kind ごとの表（approval・credential・grant・membership は新行、policy は同一行 generation+1）で明示し、照合の根拠を「世代の値ではなく `(subject_id, generation)` の組（新行は subject_id が異なる）」に直した
4. 足りなかった情報・検証手段: なし。修正は文書の記述の一貫性に関わるもので、目視確認の受け入れ条件の範囲内
5. 改善案: なし

使ったスキル: `.agents/skills/implement-issue`
