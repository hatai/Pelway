# UI担当 v0.7置換案（表示・翻訳）

- 作成：業務アプリUIデザインのプロ / 2026-11-09
- 対象：v0.6 `delivery-v0.6/fixed/pelway-technical-docs/` の表示・翻訳キー。原本は変更していない。
- 根拠：best-effort-cancel/proposal-v0.3 §9.2、secrecy-addendum-v0.2、operational-defaults v0.1、reviews/ui-secrecy-input-2026-11-09.md
- 外観（色・書体・部品）は変更しない。文言と表示条件のみ。

## 1. 置換（v0.6本文を書き換える）

| v0.6の節 | 現行 | v0.7での扱い | 理由 |
|---|---|---|---|
| 03 L49 判断待ち行 | 「取り消し依頼 · 停止確認中」 | 「取消受付済み · 停止確認中」（`reason-cancel-accepted` 系に統一） | P案では受付時点で未開始送信の抑止が効く。「依頼」は効力の時点が読めない |
| 03 L83 取消の説明 | 開始記録後は `reason-cancel-after-start`、「取り消しました」と表示しない | 維持。加えて「取消受付後は `reason-cancel-accepted` を表示。開始済み送信が残る間は集約行に `reason-cancel-accepted-pending`、各送信行に `reason-cancel-after-start`」を追記 | 集約と個別で役割を分ける |
| 03 L119 `reason-cancel-requested` | 取り消し依頼 / Cancellation requested | **削除し `reason-cancel-accepted` に置換**（日英は§2） | 同上。旧キーと併記しない |
| 03 L120 `reason-cancel-after-start` | 維持 | 維持（送信行専用と注記） | P案と矛盾しない |
| 08 L131 S1-J3 | 「取消完了前は『取り消しました』と表示しない…案Bで止める連携」 | ベストエフォート表示契約へ書き換え：受付表示／開始済み件数を同じ行に出す／結果と停止要求を別軸で表示／「取り消しました」「以後送信なし」等のゼロ送信文言は出さない。案B記述はv0.7本文に残さない（経緯はreviews/inputに収録） | 厳密保証前提の記述は置換対象 |
| 10 L74 D11 | 開始後取消で after-start、「取り消しました」不表示 | 維持。期待値に「受付直後に accepted、開始済みありで accepted-pending の件数が実件数と一致」を追加（未実施の期待値） | 表示の試験条件 |
| 11 §表示契約 | 有効化時の説明 | 追記：`integration-connected-no-business-data` は送信記録が0件で裏付けられる場合のみ表示。有効化時と承認カードの2か所でベストエフォート取消の同意文を表示 | 接続しただけでは業務データを送らない方針の可視化 |

`reason-cancel-completed`（ゼロ送信文言）は撤回済みのため、v0.7のどこにも入れない。

## 2. 03の翻訳キー表に追加（06のキー一覧にも同期）

| 安定ID | 日本語 | English |
|---|---|---|
| reason-cancel-accepted | 取り消しを受け付けました。開始前の送信は止めます。開始済みの送信は止められない場合があります。 | Cancellation accepted. Sends not yet started will be stopped; sends already started may not be stoppable. |
| reason-cancel-accepted-pending | 取消受付済み · 開始済みの送信{count}件は結果確認中（撤回されない可能性があります） | Cancellation accepted · {count} started send(s) still being confirmed (may not be reversed) |
| integration-connected-no-business-data | 接続済み · 業務データは未送信 | Connected · No business data sent |
| notice-ai-input-excluded | 一部の発言を除外して処理しました（利用の同意を確認できない範囲） | Some messages were excluded (consent for this use could not be confirmed) |
| state-use-stopped | 利用停止中（削除は未完了） | Use stopped (deletion not yet complete) |
| state-deletion-in-progress | 削除処理中（受付 {datetime} JST） | Deletion in progress (requested {datetime} JST) |
| state-deletion-completed | 削除完了（オンライン・索引・メモリ・端末で確認済み） | Deletion completed (verified for online data, index, memory, and devices) |
| state-deletion-backup-note | 復旧対象は作成から最大{backup_days}日バックアップに残り、期間経過で消えます。 | Backups may retain the data for up to {backup_days} days from creation, then expire. |

- `{backup_days}` は13（operational-defaults）の設定値を差し込む。数値を直書きしない。24時間の内部削除目標は画面に出さない。
- `state-deletion-completed` は全証拠が揃った場合のみ。
- 期限・容量・有人対応の文言は13の記述をそのまま参照（期限は絶対日付JST＋残日数、7日未満で強調／容量80%警告・100%超の保存拒否／有人は「受付済み。有人対応の時間帯は未確定」）。UIから新しい文言は作らない。
- 9状態は増やさない。上記はすべて理由・注記キー。

## 3. 確認観点（v0.7案ができたら照合）

03の翻訳キー表、06、08のS1-J3、10のD11、11の表示契約、12・13の表示記述。旧キー `reason-cancel-requested` と「取り消しました」「以後送信なし」が本文に残っていないことを grep で確認する。

## 4. 追記（プロダクトPR-03/PR-04との統一、2026-11-09 01:40）

送信ごとの行の4区分（PR-03）に、表示キーを1対1で対応させる。`reason-stop-confirming` と並べて残さない。

| 区分 | 安定ID | 日本語 | English |
|---|---|---|---|
| 未送信 | send-row-not-started | 未送信（取消により開始していません） | Not sent (not started due to cancellation) |
| 送信済み | send-row-sent | 送信済み（撤回されていません） | Sent (not reversed) |
| 結果不明 | send-row-unknown | 結果不明 · 確認中 | Result unknown · checking |
| 停止要求済み（結果確認中） | reason-cancel-after-start（文言を置換） | 停止要求済み · 結果確認中（撤回されない可能性があります） | Stop requested · checking result (may not be reversed) |

- `reason-cancel-after-start` はIDを維持し、文言だけを上表に置き換える。「取り消し依頼済み」は「受付」と用語がずれるため使わない。
- `reason-stop-confirming`（停止確認中）は削除し、上の4区分へ統合する。外部runの行も同じ4区分で表示する。
- 要約行は `reason-cancel-accepted-pending` の {count}＝「結果不明」と「停止要求済み」の合計。「送信済み」は件数に含めず、別に「送信済み{n}件」と出す（送ったものを、まだ止められる可能性があるように見せないため）。
- 「送信済み」の行に、取消ボタンや取り消し線を出さない。

## 5. v0.7本文 最終確認（2026-11-09 01:41、body/を照合）

旧キー `reason-cancel-requested`・`reason-cancel-completed`・「以後送信なし」は本文に残っていない。「取り消しました」は「表示しない」という条件文の中だけ。以下2件は修正が必要。

1. **[必須] 06:64 `notice-ai-input-excluded` の文言が誤り。** 現在は「許可された範囲だけを参照しました / Only the permitted scope was used.」で、06:52 `notice-consented-scope-only` と同じ意味になり、12:50 の「一部の発言を除外して処理しました（利用の同意を確認できない範囲）」と食い違う。二つは用途が違う（52は除外元の存在を知る権限がない出力先向け、64は知る権限がある出力先向け）。64を「一部の発言を除外して処理しました（利用の同意を確認できない範囲） / Some messages were excluded (consent for this use could not be confirmed)」に直し、12:50からキーで参照する。
2. **[必須] 03:133 `reason-cancel-after-start` が旧文言のまま。** 「取り消し依頼済み / Cancellation requested」は、同じ画面の集約行（「取消受付済み」）と用語がずれ、受付前のように読める。03:128 の外部run行（「停止要求済み（結果確認中）」）とそろえ、「停止要求済み · 結果確認中（撤回されない可能性があります） / Stop requested · checking result (may not be reversed)」にする。IDは維持。`reason-stop-confirming` を外部run行専用として残す03:161の整理は、それで問題ない。

## 6. 最終判断：reason-stop-confirming（2026-11-09 01:42）

§4の「削除」を取り下げ、本文03:128・03:161・06:57の現行（外部run行専用として残す）を採る。外部runは送信ではなく「撤回されない」の注記が当たらないため、送信行と別キーの方が正確。§4の4区分は送信行に適用する。残る修正は§5の2件（06:64、03:133）のみ。
