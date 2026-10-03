//! `TestOutcome`: a device test's verdict per case, from its report and how the process
//! ended (spec §7). `symbian-test` marks each case `running` before it runs it, so a panic
//! — which ends the process — is the running case's; the cases still pending never ran.
use symdev_emulator::{CaseState, TestReport};

use super::AppExit;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Ok,
    Failed,
    NotRun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CaseLine {
    pub name: String,
    pub verdict: Verdict,
    pub detail: String,
}

pub(crate) struct TestOutcome;

impl TestOutcome {
    pub fn settle(report: &TestReport, exit: &AppExit) -> Vec<CaseLine> {
        report
            .cases
            .iter()
            .map(|c| {
                let (verdict, detail) = match (c.state, c.ok) {
                    (None, true) => (Verdict::Ok, c.detail.clone()),
                    (None, false) => (Verdict::Failed, c.detail.clone()),
                    (Some(CaseState::Running), _) => (
                        Verdict::Failed,
                        exit.message
                            .clone()
                            .unwrap_or_else(|| "the process ended during this test".into()),
                    ),
                    (Some(CaseState::Pending), _) => (Verdict::NotRun, String::new()),
                };
                CaseLine {
                    name: c.name.clone(),
                    verdict,
                    detail,
                }
            })
            .collect()
    }
}
