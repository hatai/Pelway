# Screen messages (English). Draft translations; final wording is undecided.
# Keys mirror ja/main.ftl (stable ids per docs/technical/v0.7/body/06-i18n.md).
# Stage A has no Fluent crate yet; only the tests read this file. Stage B will
# look these up from Rust via Fluent and pass the strings into Slint properties.
# Variables use Fluent syntax ({ $name }).

## Left rail
rail-viewing-as = Viewing as { $name }
rail-today = Today
rail-channels = Channels
rail-dm-people = DMs · People
rail-dm-ai = DMs · AI staff
## AI staff presence (rail)
ai-presence-standby = Standby
ai-presence-working = Working
ai-presence-waiting = Awaiting decision
## Top bar
search-placeholder = ⌕ Search conversations and work logs
search-shortcut = ⌘K
## Today view
today-title = Today
today-section-consultations = For you
today-section-pending = Pending decisions
today-section-decided = Decided while you were away
today-section-empty = None
today-quiet = Nothing urgent
## Channel view
unread-divider = New below here
ai-badge = AI
decision-mark = ◆ Decision · { $name } { $time }
message-private-note = Visible only to you
thread-replies = { $n } replies
thread-working = { $name } working
thread-done = Done { $time }
thread-view-work = View work
mention-read-range = Read { $n } messages from { $from } to { $to }
## Handoff card
handoff-context = View background
handoff-acknowledged = Received · { $name } { $time }
handoff-not-acknowledged = Not yet received
handoff-acknowledge = Got it
handoff-decision-owner = Deadline decided by: { $name } (lead)
## External send confirmation card
ext-card-label = External send · awaiting { $name }'s confirmation
ext-card-question = Send an email to { $to }?
ext-address-label = To:
ext-subject-label = Subject:
ext-basis-label = Basis:
ext-send = Send
ext-fix = Edit
ext-lock-note = 🔒 Every external send is confirmed each time. Pressing Send records it as { $name }'s send approval
ext-sent = ✓ Sent by { $name } at { $time } · awaiting customer reply
## Right pane (thread / AI work)
pane-thread-title = Thread
pane-close = Close
pane-back-to-thread = Back to thread
pane-ai-work-title = { $name }'s work
pane-ai-steps = Step { $done } of { $total }
pane-ai-from = Requested from:
pane-ai-read-range = Read range:
pane-ai-note = Work in progress is not shown in the conversation.
thread-input-placeholder = Reply in thread
## Message composer
composer-placeholder = Message { $channel }
