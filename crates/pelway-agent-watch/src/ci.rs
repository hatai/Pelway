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
