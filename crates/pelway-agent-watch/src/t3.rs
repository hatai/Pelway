//! `t3 thread list --json` / `t3 thread show --json` の出力を読む。

use serde_json::Value;

use crate::watch::{ListedThread, ThreadDetail};

/// `t3 thread list --json` の出力（配列）を読む。
pub fn parse_list(json: &str) -> Result<Vec<ListedThread>, String> {
    serde_json::from_str(json).map_err(|e| format!("t3 thread list: {e}"))
}

/// `t3 thread show --json` の出力を読む。最後のメッセージが assistant の返答なら `last_assistant_text` に入れる。
pub fn parse_show(json: &str) -> Result<ThreadDetail, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("t3 thread show: {e}"))?;
    let t = &v["thread"];
    let text = |key: &str| t[key].as_str().map(str::to_string);
    let count = |key: &str| {
        t[key]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0)
    };
    let id = text("threadId").ok_or("t3 thread show: threadId がない")?;
    let last_assistant_text = v["messages"]
        .as_array()
        // 最後のメッセージが assistant のときだけ。後ろに user の指示があれば、その返答はまだ無い
        .and_then(|ms| ms.last())
        .filter(|m| m["role"] == "assistant")
        .and_then(|m| m["text"].as_str())
        .map(str::to_string);
    Ok(ThreadDetail {
        id,
        status: text("status").unwrap_or_default(),
        updated_at: text("updatedAt").unwrap_or_default(),
        branch: text("branch"),
        provider: text("providerInstanceId").unwrap_or_default(),
        model: text("model").unwrap_or_default(),
        link: text("link").unwrap_or_default(),
        run_count: count("runCount"),
        pending_requests: count("pendingRequestCount"),
        last_assistant_text,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHOW: &str = r#"{"thread":{"threadId":"mcp:t1","status":"completed","updatedAt":"2026-11-10T17:43:20.093Z",
      "branch":"agent/80-agent-gh-help","providerInstanceId":"opencode","model":"zai-coding-plan/glm-5.3",
      "runCount":2,"pendingRequestCount":0,"link":"https://app.t3.codes/threads/x"},
      "messages":[{"role":"assistant","text":"first"},{"role":"user","text":"next"},{"role":"assistant","text":"issue: #80"}]}"#;

    #[test]
    fn list_reads_array() {
        let l = parse_list(r#"[{"threadId":"a","status":"running","updatedAt":"2026-11-10T16:00:00Z","title":"x"}]"#).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].id, "a");
        assert_eq!(l[0].status, "running");
    }

    #[test]
    fn list_rejects_garbage() {
        assert!(parse_list("not json").is_err());
    }

    #[test]
    fn show_reads_fields_and_last_assistant() {
        let d = parse_show(SHOW).unwrap();
        assert_eq!(d.id, "mcp:t1");
        assert_eq!(d.branch.as_deref(), Some("agent/80-agent-gh-help"));
        assert_eq!(d.provider, "opencode");
        assert_eq!(d.run_count, 2);
        assert_eq!(d.pending_requests, 0);
        assert_eq!(d.last_assistant_text.as_deref(), Some("issue: #80"));
    }

    #[test]
    fn show_ignores_assistant_text_followed_by_user() {
        let d = parse_show(r#"{"thread":{"threadId":"c","status":"failed","updatedAt":"t","branch":null,
          "providerInstanceId":"p","model":"m","runCount":2,"pendingRequestCount":0,"link":"l"},
          "messages":[{"role":"assistant","text":"verify: pass"},{"role":"user","text":"fix it"}]}"#)
        .unwrap();
        assert_eq!(d.last_assistant_text, None);
    }

    #[test]
    fn show_null_branch_and_no_messages() {
        let d = parse_show(r#"{"thread":{"threadId":"b","status":"running","updatedAt":"t","branch":null,
          "providerInstanceId":"codex","model":"m","runCount":1,"pendingRequestCount":1,"link":"l"},"messages":[]}"#)
        .unwrap();
        assert_eq!(d.branch, None);
        assert_eq!(d.pending_requests, 1);
        assert_eq!(d.last_assistant_text, None);
    }
}
