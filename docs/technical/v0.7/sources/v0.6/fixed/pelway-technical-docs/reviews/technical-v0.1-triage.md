# 技術レビューv0.1の採否（改訂作業中）

本文へ反映中。v0.1固定版は保持する。最終検査/外部レビューと担当再確認前であり、完成判定ではない。

| 指摘 | 暫定扱い | 理由/対応 |
|---|---|---|
| TR-1 | 採用・本文反映 | 未送信状態からstoppedへの取消/権限取消/期限切れの行を追加提案として明記。PRDを変更しない |
| TR-2 | 改訂TR-2を採用 | 承認時/呼出直前の再検査を維持し、dispatch予約とattempt_startedの境界を明確化した。PRD既存の送信中→stopped（外部呼出前だけ）を、文書上勝手に削除しない。キュー前検査だけで実行直前の再検査を省かない |
| TR-3 | 事実部分を採用・本文反映 | X一次投稿を独立に取得して確認。本人はElectron/React NativeからRustへの移行とGPUI採用を説明し、ユーザー試験を今後と記す。移行完了/一般配布とはしない。1.3GB→300MB比較は本文に根拠なし |
| TR-4 | 採用・本文反映 | 管理ホストAdapterと任意の利用者端末bridgeを図で分離。後者の所在/接続は未決、今回未実装 |
| TR-5 | 将来の代替として追記 | commit後のdispatcher採番には永続化と順序検証が別途必要。今のsmall local lock案を置き換えない |
| TR-6 | 採用・本文反映 | dedup/order/cursor/照会契約をcapabilityへ追加 |
| TR-7 | 維持 | Apple内部network未確認を機密実行の阻害として維持 |

## TR-3の一次証拠

[CTO本人の投稿](https://x.com/svishnevskiy/status/2118081920194203799)を2026-11-08（日本時間）に公開X取得で確認。投稿は2026-11-05 05:44:16（日本時間）。author ID22238139、profileはDiscord CTO & Co-Founder。note_tweetの全文にElectron廃止、React Native廃止、mobile native UI/Rust共通コード、GPUIと記載し、ユーザー試験を今後とする。取得原文はdiscord-primary-post-evidence.txt。数字の同条件比較の証拠として画像/第三者報道は採用しない。
