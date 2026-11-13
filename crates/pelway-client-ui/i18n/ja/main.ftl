# 画面の文言（日本語）。キーは安定した名前にする（docs/technical/v0.7/body/06-i18n.md）。
# 段階Aでは Fluent の crate を使わないため、このファイルはテスト（行の「キー =」を読む
# 照合）だけが読む。段階Bで Rust から Fluent として引いて Slint のプロパティへ渡す。
# 変数は Fluent の書式（{ $name }）。部品の並びは docs/ui-mocks/csv.html（暫定承認 11/8）による。

## 左の列（rail）
rail-viewing-as = { $name }として表示
rail-today = 今日
rail-channels = チャンネル
rail-dm-people = DM・人
rail-dm-ai = DM・AI社員
## AI 社員の在席状態（rail の表示）
ai-presence-standby = 待機中
ai-presence-working = 作業中
ai-presence-waiting = 判断待ち
## 上の帯
search-placeholder = ⌕ 会話と作業記録を検索
search-shortcut = ⌘K
## 「今日」の画面
today-title = 今日
today-section-consultations = あなたへの相談
today-section-pending = 判断待ち
today-section-decided = 離れている間に決まったこと
today-section-empty = なし
today-quiet = 急ぎはありません
## チャンネルの画面
unread-divider = ここから未読
ai-badge = AI
decision-mark = ◆ 決定 · { $name } { $time }
message-private-note = あなただけに表示
thread-replies = 返信 { $n } 件
thread-working = { $name } 作業中
thread-done = 完了 { $time }
thread-view-work = 作業を見る
mention-read-range = { $from }〜{ $to }の{ $n }件を読みました
## 引き継ぎカード
handoff-context = 経緯を見る
handoff-acknowledged = 受領 · { $name } { $time }
handoff-not-acknowledged = 未受領
handoff-acknowledge = 受けました
handoff-decision-owner = 納期の決定：{ $name }（代表）
## 社外への送信の確認カード
ext-card-label = 社外への送信 · { $name }さんの確認待ち
ext-card-question = { $to }にメールを送りますか
ext-address-label = 宛先：
ext-subject-label = 件名：
ext-basis-label = 根拠：
ext-send = 送る
ext-fix = 直す
ext-lock-note = 🔒 社外への送信は毎回確認。「送る」を押すと、{ $name }さんの送信承認として記録されます
ext-sent = ✓ { $name } { $time } 送信 · 顧客回答待ち
## 右の枠（スレッド・AIの作業）
pane-thread-title = スレッド
pane-close = 閉じる
pane-back-to-thread = スレッドに戻る
pane-ai-work-title = { $name }の作業
pane-ai-steps = 手順 { $done }/{ $total }
pane-ai-from = 依頼元：
pane-ai-read-range = 読んだ範囲：
pane-ai-note = 作業の途中経過は会話には出しません。
thread-input-placeholder = スレッドに返信
## 下のメッセージ入力欄
composer-placeholder = { $channel } にメッセージ
## トップレベルメニューと About ダイアログ（Slint の attribution、Issue 237）
menu-help = ヘルプ
menu-about = このアプリについて
about-title = このアプリについて
about-close = 閉じる
