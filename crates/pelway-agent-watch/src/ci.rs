//! `agent/` ブランチで verify が続けて失敗したかの判定。

use serde::Deserialize;

use crate::watch::State;

/// `gh run list --json conclusion,headSha` の 1 要素。
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RunInfo {
    pub conclusion: String,
    #[serde(rename = "headSha")]
    pub head_sha: String,
}

/// 見張る PR: このリポジトリ内の `agent/` ブランチから出ている open な PR。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPr {
    pub branch: String,
    pub head_sha: String,
}

/// `gh pr list --json headRefName,headRefOid,isCrossRepository` の出力を読む。
/// fork からの PR は、ブランチ名が `agent/` でも外す（外部の人がブランチ名を選べるため）。
pub fn parse_agent_prs(json: &str) -> Result<Vec<AgentPr>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("gh pr list: {e}"))?;
    Ok(v.as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["isCrossRepository"] == false)
        .filter_map(|p| {
            let branch = p["headRefName"].as_str()?;
            let head_sha = p["headRefOid"].as_str()?;
            branch.starts_with("agent/").then(|| AgentPr {
                branch: branch.to_string(),
                head_sha: head_sha.to_string(),
            })
        })
        .collect())
}

/// 最新の実行が PR の head に対するものか。fork の実行は同じブランチ名でも head が違うので外れる。
pub fn newest_run_is_head(runs_newest_first: &[RunInfo], head_sha: &str) -> bool {
    runs_newest_first
        .first()
        .is_some_and(|r| r.head_sha == head_sha)
}

/// 直近 2 件（新しい順）がどちらも失敗で、別のコミットに対するものなら、新しい方の sha を返す。
pub fn ci_failed_twice(runs_newest_first: &[RunInfo]) -> Option<String> {
    match runs_newest_first {
        [newest, previous, ..]
            if newest.conclusion == "failure"
                && previous.conclusion == "failure"
                && newest.head_sha != previous.head_sha =>
        {
            Some(newest.head_sha.clone())
        }
        _ => None,
    }
}

/// まだ通知していない連続失敗なら記録し、`emit` が true なら true を返す。
/// 初回の周回では `emit = false` で呼び、既にある連続失敗を記録だけする。
pub fn ci_should_notify(state: &mut State, branch: &str, runs: &[RunInfo], emit: bool) -> bool {
    let Some(sha) = ci_failed_twice(runs) else {
        return false;
    };
    if state.ci_notified.get(branch) == Some(&sha) {
        return false;
    }
    state.ci_notified.insert(branch.to_string(), sha);
    emit
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(conclusion: &str, sha: &str) -> RunInfo {
        RunInfo {
            conclusion: conclusion.into(),
            head_sha: sha.into(),
        }
    }

    #[test]
    fn agent_prs_skip_forks_and_other_branches() {
        let prs = parse_agent_prs(
            r#"[{"headRefName":"agent/1-x","headRefOid":"s1","isCrossRepository":false},
                {"headRefName":"agent/2-y","headRefOid":"s2","isCrossRepository":true},
                {"headRefName":"feat/z","headRefOid":"s3","isCrossRepository":false}]"#,
        )
        .unwrap();
        assert_eq!(
            prs,
            vec![AgentPr {
                branch: "agent/1-x".into(),
                head_sha: "s1".into()
            }]
        );
    }

    #[test]
    fn agent_prs_missing_cross_repo_flag_is_skipped() {
        let prs = parse_agent_prs(r#"[{"headRefName":"agent/1-x","headRefOid":"s1"}]"#).unwrap();
        assert!(prs.is_empty());
    }

    #[test]
    fn newest_run_must_match_head() {
        let runs = [run("failure", "b"), run("failure", "a")];
        assert!(newest_run_is_head(&runs, "b"));
        assert!(!newest_run_is_head(&runs, "x"));
        assert!(!newest_run_is_head(&[], "b"));
    }

    #[test]
    fn ci_two_failures_distinct_sha() {
        assert_eq!(
            ci_failed_twice(&[run("failure", "b"), run("failure", "a")]),
            Some("b".to_string())
        );
    }

    #[test]
    fn ci_same_sha_rerun_is_not_twice() {
        assert_eq!(
            ci_failed_twice(&[run("failure", "a"), run("failure", "a")]),
            None
        );
    }

    #[test]
    fn ci_failure_then_success() {
        assert_eq!(
            ci_failed_twice(&[run("failure", "b"), run("success", "a")]),
            None
        );
        assert_eq!(ci_failed_twice(&[run("failure", "b")]), None);
    }

    #[test]
    fn ci_baseline_records_without_notifying() {
        let mut state = State::default();
        let runs = [run("failure", "b"), run("failure", "a")];
        assert!(!ci_should_notify(&mut state, "agent/1-x", &runs, false));
        assert!(!ci_should_notify(&mut state, "agent/1-x", &runs, true));
    }

    #[test]
    fn ci_notify_once_per_sha() {
        let mut state = State::default();
        let runs = [run("failure", "b"), run("failure", "a")];
        assert!(ci_should_notify(&mut state, "agent/1-x", &runs, true));
        assert!(!ci_should_notify(&mut state, "agent/1-x", &runs, true));
        let newer = [run("failure", "c"), run("failure", "b")];
        assert!(ci_should_notify(&mut state, "agent/1-x", &newer, true));
    }
}
