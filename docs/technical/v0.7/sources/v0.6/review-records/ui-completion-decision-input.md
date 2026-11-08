# 取消完了後の表示案（未採用・J3判断材料）

出典は今回ルームでのUI担当案とプロダクト担当の補足。文書完成の条件ではない。J1で案Aを採用する正式判断、S1の送出制御の検証、03の対象外部runの停止確認がそろうまでは使用しない。案Bで使えるとの決定ではない。

- reason-cancel-completed：日「取り消しました（これ以降、外部への送信はありません）」、英「Cancelled. No further external actions will be sent.」
- reason-cancel-completed-pending：日「取り消しました · 取消前に開始した送信{count}件は結果確認中」、英「Cancelled · {count} send(s) started before cancellation still being confirmed」

過去の送信中/結果不明を隠さず、同じ行に実際の件数を出す案。countは変数であり実件数ではない。「開始」の基準はJ1の判断と整合させる必要がある。開始受理だけでは実送出の証拠でないという08の条件は変えない。文言の保証そのものが未確認なので、実装採択・保証成立を先取りしない。
