# Issue #149 の作業ログ

## 実装者メモ（round 2）

対象 head: 3a5934cfe3a40f6985baafb6f744c6fe3e415d9d（round 3 のエスカレーションで hatai が選択肢 1 を採用。この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: この sandbox の開発環境: rust 1.99.1 未導入（cargo が 1.94.1 に解決し `--workspace --locked` が MSRV エラー）、mbx キャッシュと podman が読み取り専用、PostgreSQL 無し。`mise install rust@1.99.1` + `RUSTUP_TOOLCHAIN=1.99.1` + XDG 系変数の /tmp/opencode へのリダイレクト + `mise install postgres@16` で解決。pre-commit フック（`mise x -- hk`）が mbx 経由で失敗するため、同一検査を `mise run verify` で通した上で `HK=0`（フック自身の無効化変数）でコミットした。`pr-size` +832 超過はテスト・説明を削らず構造圧縮（表駆動化・扶助関数化・重複統合）で収めた
2. 曖昧だった文: 「新しい NOLOGIN グループロール `pelway_audit`（仮名）に監査表への **INSERT のみ**を付与」（Issue #149 質問票ラウンド1 Q4 の推奨）
3. 仮定と解釈: 挿入のたびに鎖頭（直近の `hash`）を読み取る SELECT が `pelway_audit` に必要なため、「INSERT のみ」は「SELECT + INSERT を付与し、UPDATE・DELETE は誰にも付与しない」と解釈した（Q4 の趣旨である append-only と `pelway_admin` 無権限は保つ）。Q7 の「chain.rs をそのまま使う」は、`AuditEvent` に理由（reason）のフィールドが無く Q3 の「理由を記した監査行」を満たせないため、`reason: Option<String>` の追加とし、`None` のときは従来の符号化（タグ `0x00`）のバイト列を完全に保存し、`Some` のときだけ拡張符号化（タグ `0x01`）を使う拡張とした（`compat.rs` の ed25519-dalek 2.2.0 固定バイト列テストが変わらず通ることが保存の証拠）。選ばなかった解釈: reason を鎖の外の平文列に置く（理由の改竄が鎖の検出対象から外れるため）
4. 足りなかった情報・検証手段: 開発環境の PostgreSQL の調達手順。docs/dev-environment.md は podman 前提だが、この sandbox では podman が起動できず、mise の postgres パッケージが使えることに気づくまで手間取った
5. 改善案: docs/dev-environment.md の「開発用 PostgreSQL」に「podman が使えない環境では `mise install postgres@16` で `initdb`/`pg_ctl` を `/tmp` 側に起こす代替がある」を追記

使ったスキル: implement-issue、grilling（.agents/skills/grilling/SKILL.md）

## 実装者メモ（round 4）

対象 head: 3a5934cfe3a40f6985baafb6f744c6fe3e415d9d（round 3 のエスカレーションで hatai が選択肢 1 を採用。この round の実装の実体を含むコミット。このメモを含むコミットの親）

1. 詰まった箇所: `mise run pr-size`: must-fix の実装で追加行が膨らみ上限 800 を超えた。テスト・説明を削らず、構造の圧縮（AC5 の表のループ化・AC1 のタプル化・未使用 API の削除〔`AdminOpResult` の構造体化と `impl Error` の取り下げ〕・doc の緊密化）で上限内に収めた
2. 曖昧だった文: 「Include the relationship in the authenticated row content and verify its kind/uniqueness, or otherwise make this linkage tamper-evident.」（PR #302 の codex レビューの must-fix）
3. 仮定と解釈: 「verify its kind/uniqueness」は DDL の CHECK（kind と ref_id の組）と UNIQUE (ref_id) で満たすと解釈した。符号化のタグは reason/ref_id の有無の組 4 種（0x00〜0x03）とし、reason だけ・両方の区別はタグで一意にした
4. 足りなかった情報・検証手段: devin_review への effort 指定（mode のみを受け付け、2 回目の effort 引き上げが効かない）
5. 改善案: `.gitignore` へのハーネスファイル（opencode.json）の追加は本題と無関係なため取り下げた。別途の小 PR で足す

使ったスキル: implement-issue
