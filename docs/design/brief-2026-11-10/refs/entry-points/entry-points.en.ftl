# Pelway UI strings — entry points / S00 banners (English · proposal)
# Message IDs must match entry-points.ja.ftl exactly.
# Merge into main en.ftl later; do not edit export-i18n/*.ftl from this folder.
# Product UI must contain no Japanese characters (options 「日本語」/English stay native per language mock).

## S00 always-on labels (never hide in English reduction)
# REQUIRED
# SOURCE: ja wording from RUI §2.0 L147 / GDPR-BE A11 L104. EN is UI proposal (no EN in sources).
# ID: proposed by UI (demo-screen-list §5 item 4: no stable ID in sources).
# LEGAL (2026-11-09 via lead): "(simulated)" / "simulated responses" must NEVER be truncated,
# ellipsized, or dropped on narrow screens or in notifications. Allow wrap (up to 2 lines);
# do not use text-overflow: ellipsis on this string. EN is longer — verify wrap on mobile.
banner-demo-mock = Demo · simulated responses
# REQUIRED
# SOURCE: requirement name is 「AIであることの表示」(RUI L147, GDPR-BE L104).
# Variants seen: 「AIの表示」(RUI L173), 「AIであることを示す表示」(delivery-v0.7/input/ui.md L42), 「AIであることを明示」(public decisions.md).
# Pick: shortest unambiguous badge "AI" (proposal; lead confirmed for demo 2026-11-09).
# Visible text only. Screen readers use label-ai-member-a11y (do not announce this ID twice).
# LEGAL (2026-11-09 via lead): show next to the name on EVERY message, profiles, and mention
# suggestions (not only the member list). Cannot be hidden/removed by user or admin settings
# (no toggle exists). Distinguishable by text, not only color (keep the letters "AI" + border).
label-ai-member = AI
# REQUIRED
# LEGAL (2026-11-09 via lead): accessible name for the AI badge. Screen readers read this
# instead of (or in addition to, without duplicating) the visible "AI".
# Implementation: visible span uses aria-hidden="true"; a visually-hidden span holds this
# string (preferred over aria-label alone so the name+badge reads e.g. "Hamada Bot, AI member"
# and not "AI AI"). Do not put both aria-label="AI" and this string.
label-ai-member-a11y = AI member
# REQUIRED
# SOURCE: ja 「送信済み（模擬）」 RUI L146 / L183 / demo-screen-list L75.
# Distinct from product state sent_awaiting_reply (BODY03 L23 = "Sent, awaiting customer reply").
# Lead (2026-11-09): demo display string for sent_awaiting_reply; state enum stays; wording is demo-only.
# Needs backend/core contract (demo-screen-list §5 item 5). EN is UI proposal.
# ID: proposed by UI.
# LEGAL (2026-11-09 via lead): "(simulated)" must NEVER be truncated, ellipsized, or dropped
# on narrow screens or in notifications. Allow wrap (up to 2 lines); no text-overflow: ellipsis.
# EN "Sent (simulated)" is longer — verify wrap on mobile.
state-sent-simulated = Sent (simulated)

## Native data-protection panel (§3.4 · hide delete/export/consent)
native-dp-panel-title = Data protection
native-dp-locations = Data location list
native-dp-legal-basis = Legal basis by purpose
native-dp-no-body-logs = Logs without message bodies
# REQUIRED
# CONDITIONAL: only when native build (Web build must not show this line).
# Static non-interactive note. Not a link, not a button, no focus stop. Normal text color.
# LEGAL condition (2026-11-09 via lead): native may omit delete/export/consent ONLY if the
# same account can actually perform them in the web version, and the demo script shows them there.
native-dp-web-only-note = Export, removal, and consent are handled in the web version.

## Sidebar / nav labels used in entry-point mocks (parity only; may already exist elsewhere)
nav-channel-cases = Case B
nav-group-channels = Channels
nav-group-tools = Tools
nav-wiki = Wiki & knowledge
nav-notifications = Notifications
nav-data-protection = Data protection
nav-settings = Settings
nav-language-row = 表示言語 · Language
