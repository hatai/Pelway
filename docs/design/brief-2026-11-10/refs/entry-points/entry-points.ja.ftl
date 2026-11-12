# Pelway UI strings — entry points / S00 banners (Japanese · proposal)
# Message IDs must match entry-points.en.ftl exactly.
# Merge into main ja.ftl later; do not edit export-i18n/*.ftl from this folder.

## S00 always-on labels (never hide in English reduction)
# REQUIRED
# SOURCE: ja wording from RUI §2.0 L147 / GDPR-BE A11 L104. EN is UI proposal (no EN in sources).
# ID: proposed by UI (demo-screen-list §5 item 4: no stable ID in sources).
# LEGAL (2026-11-09 via lead): 「（模擬）」/ "simulated responses" must NEVER be truncated,
# ellipsized, or dropped on narrow screens or in notifications. Allow wrap (up to 2 lines);
# do not use text-overflow: ellipsis on this string.
banner-demo-mock = デモ・模擬応答
# REQUIRED
# SOURCE: requirement name is 「AIであることの表示」(RUI L147, GDPR-BE L104).
# Variants seen: 「AIの表示」(RUI L173), 「AIであることを示す表示」(delivery-v0.7/input/ui.md L42), 「AIであることを明示」(public decisions.md).
# Pick: shortest unambiguous badge 「AI」(proposal; lead confirmed for demo 2026-11-09).
# Visible text only. Screen readers use label-ai-member-a11y (do not announce this ID twice).
# LEGAL (2026-11-09 via lead): show next to the name on EVERY message, profiles, and mention
# suggestions (not only the member list). Cannot be hidden/removed by user or admin settings
# (no toggle exists). Distinguishable by text, not only color (keep the letters "AI" + border).
label-ai-member = AI
# REQUIRED
# LEGAL (2026-11-09 via lead): accessible name for the AI badge. Screen readers read this
# instead of (or in addition to, without duplicating) the visible "AI".
# Implementation: visible span uses aria-hidden="true"; a visually-hidden span holds this
# string (preferred over aria-label alone so the name+badge reads e.g. "浜田Bot、AIメンバー"
# and not "AI AI"). Do not put both aria-label="AI" and this string.
label-ai-member-a11y = AIメンバー
# REQUIRED
# SOURCE: ja 「送信済み（模擬）」 RUI L146 / L183 / demo-screen-list L75.
# Distinct from product state sent_awaiting_reply (BODY03 L23 = 「送信済み・顧客回答待ち」).
# Lead (2026-11-09): demo display string for sent_awaiting_reply; state enum stays; wording is demo-only.
# Needs backend/core contract (demo-screen-list §5 item 5). EN is UI proposal.
# ID: proposed by UI.
# LEGAL (2026-11-09 via lead): 「（模擬）」 must NEVER be truncated, ellipsized, or dropped
# on narrow screens or in notifications. Allow wrap (up to 2 lines); no text-overflow: ellipsis.
state-sent-simulated = 送信済み（模擬）

## Native data-protection panel (§3.4 · hide delete/export/consent)
native-dp-panel-title = データ保護
native-dp-locations = データの置き場所一覧
native-dp-legal-basis = 目的ごとの根拠記録
native-dp-no-body-logs = 本文なしログの表示
# REQUIRED
# CONDITIONAL: only when native build (Web build must not show this line).
# Static non-interactive note. Not a link, not a button, no focus stop. Normal text color.
# LEGAL condition (2026-11-09 via lead): native may omit delete/export/consent ONLY if the
# same account can actually perform them in the web version, and the demo script shows them there.
native-dp-web-only-note = 書き出し・削除・同意の操作はWeb版で行います。

## Sidebar / nav labels used in entry-point mocks (parity only; may already exist elsewhere)
nav-channel-cases = 案件B
nav-group-channels = チャンネル
nav-group-tools = ツール
nav-wiki = Wiki・ナレッジ
nav-notifications = 通知
nav-data-protection = データ保護
nav-settings = 設定
nav-language-row = 表示言語 · Language
