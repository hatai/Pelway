//! Grok Bot に送る payload（`pelway.agent-watch/v1`）。

use serde_json::{Value, json};

use crate::parse::{extract_report, issue_number};
use crate::watch::{Event, StallReason, ThreadDetail};

pub const SCHEMA: &str = "pelway.agent-watch/v1";

/// イベントとスレッドの詳細から payload を作る。キーは 14 個で固定。
pub fn build_payload(event: &Event, d: &ThreadDetail, host: &str, occurred_at: &str) -> Value {
    let (name, stall_reason) = match event {
        Event::Finished => ("finished", None),
        Event::Failed => ("failed", None),
        Event::Cancelled => ("cancelled", None),
        Event::Waiting => ("waiting", None),
        Event::Stalled(StallReason::NoActivity) => ("stalled", Some("no-activity")),
        Event::Stalled(StallReason::CiFailedTwice) => ("stalled", Some("ci-failed-twice")),
    };
    json!({
        "schema": SCHEMA,
        "event": name,
        // エージェントのイベントは、どれも Grok Bot の次の行動が要る（仕様「見張り役と通知」）
        "urgency": "immediate",
        "thread_id": d.id,
        "provider": d.provider,
        "model": d.model,
        "branch": d.branch,
        "issue_number": d.branch.as_deref().and_then(issue_number),
        "round": d.run_count,
        "report": d.last_assistant_text.as_deref().and_then(extract_report),
        "stall_reason": stall_reason,
        "link": d.link,
        "host": host,
        "occurred_at": occurred_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::watch::StallReason;

    fn detail(branch: Option<&str>, run_count: u32, last: Option<&str>) -> ThreadDetail {
        ThreadDetail {
            id: "mcp:t1".into(),
            status: "completed".into(),
            updated_at: "2026-11-10T16:00:00Z".into(),
            branch: branch.map(Into::into),
            provider: "opencode".into(),
            model: "zai-coding-plan/glm-5.3".into(),
            link: "https://app.t3.codes/threads/x".into(),
            run_count,
            pending_requests: 0,
            last_assistant_text: last.map(Into::into),
        }
    }

    #[test]
    fn payload_has_fixed_keys() {
        let v = build_payload(
            &Event::Finished,
            &detail(None, 1, None),
            "host",
            "2026-11-10T16:00:05Z",
        );
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        let mut want = vec![
            "schema",
            "event",
            "urgency",
            "thread_id",
            "provider",
            "model",
            "branch",
            "issue_number",
            "round",
            "report",
            "stall_reason",
            "link",
            "host",
            "occurred_at",
        ];
        want.sort_unstable();
        assert_eq!(keys, want);
        assert_eq!(v["schema"], SCHEMA);
        assert_eq!(v["event"], "finished");
        assert_eq!(v["urgency"], "immediate");
        assert_eq!(v["stall_reason"], Value::Null);
    }

    #[test]
    fn payload_report_null_for_prose() {
        let v = build_payload(
            &Event::Finished,
            &detail(None, 1, Some("確認しました。")),
            "h",
            "t",
        );
        assert_eq!(v["report"], Value::Null);
    }

    #[test]
    fn payload_report_for_report_lines() {
        let v = build_payload(
            &Event::Finished,
            &detail(None, 1, Some("pr: #81\nverdict: approve\nmust-fix: 0")),
            "h",
            "t",
        );
        assert_eq!(v["report"], "pr: #81\nverdict: approve\nmust-fix: 0");
    }

    #[test]
    fn payload_issue_and_round() {
        let v = build_payload(
            &Event::Finished,
            &detail(Some("agent/80-x"), 2, None),
            "h",
            "t",
        );
        assert_eq!(v["issue_number"], 80);
        assert_eq!(v["round"], 2);
        assert_eq!(v["branch"], "agent/80-x");
    }

    #[test]
    fn payload_event_names_and_stall_reason() {
        let d = detail(None, 1, None);
        let name = |e: Event| build_payload(&e, &d, "h", "t")["event"].clone();
        assert_eq!(name(Event::Failed), "failed");
        assert_eq!(name(Event::Cancelled), "cancelled");
        assert_eq!(name(Event::Waiting), "waiting");
        let v = build_payload(&Event::Stalled(StallReason::CiFailedTwice), &d, "h", "t");
        assert_eq!(v["event"], "stalled");
        assert_eq!(v["stall_reason"], "ci-failed-twice");
        let v = build_payload(&Event::Stalled(StallReason::NoActivity), &d, "h", "t");
        assert_eq!(v["stall_reason"], "no-activity");
    }
}
