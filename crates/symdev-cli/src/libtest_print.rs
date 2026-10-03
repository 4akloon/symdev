//! `LibtestPrint`: a device test run printed the way `libtest` prints one (spec §7).
use crate::run::{CaseLine, Verdict};

pub(crate) struct LibtestPrint;

impl LibtestPrint {
    /// `running N tests`, a `test <name> ... ok|FAILED|not run` line per case, the
    /// failures with their detail, and `test result: …`; and whether the run passed: every
    /// case ran and passed, and there was at least one.
    pub fn lines(cases: &[CaseLine]) -> (Vec<String>, bool) {
        let count = |v: Verdict| cases.iter().filter(|c| c.verdict == v).count();
        let (passed, failed, not_run) = (
            count(Verdict::Ok),
            count(Verdict::Failed),
            count(Verdict::NotRun),
        );
        let mut out = vec![format!(
            "running {} test{}",
            cases.len(),
            plural(cases.len())
        )];
        for c in cases {
            let verdict = match c.verdict {
                Verdict::Ok => "ok",
                Verdict::Failed => "FAILED",
                Verdict::NotRun => "not run",
            };
            out.push(format!("test {} ... {verdict}", c.name));
        }
        out.push(String::new());
        if failed > 0 {
            out.push("failures:".into());
            for c in cases.iter().filter(|c| c.verdict == Verdict::Failed) {
                out.push(format!("    {}: {}", c.name, c.detail));
            }
            out.push(String::new());
        }
        let ok = failed == 0 && not_run == 0 && !cases.is_empty();
        let mut result = format!(
            "test result: {}. {passed} passed; {failed} failed",
            if ok { "ok" } else { "FAILED" }
        );
        if not_run > 0 {
            result.push_str(&format!("; {not_run} not run"));
        }
        out.push(result);
        (out, ok)
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

#[cfg(test)]
mod tests {
    use super::LibtestPrint;
    use crate::run::{CaseLine, Verdict};

    fn line(name: &str, verdict: Verdict, detail: &str) -> CaseLine {
        CaseLine {
            name: name.into(),
            verdict,
            detail: detail.into(),
        }
    }

    #[test]
    fn a_passing_run_prints_like_libtest() {
        let (out, ok) =
            LibtestPrint::lines(&[line("a", Verdict::Ok, ""), line("b", Verdict::Ok, "")]);
        assert!(ok);
        assert_eq!(
            out,
            [
                "running 2 tests",
                "test a ... ok",
                "test b ... ok",
                "",
                "test result: ok. 2 passed; 0 failed"
            ]
        );
    }

    #[test]
    fn a_panic_fails_its_test_and_leaves_the_rest_not_run() {
        let (out, ok) = LibtestPrint::lines(&[
            line("a", Verdict::Ok, ""),
            line("b", Verdict::Failed, "panicked: RUST 3"),
            line("c", Verdict::NotRun, ""),
        ]);
        assert!(!ok);
        assert_eq!(
            out,
            [
                "running 3 tests",
                "test a ... ok",
                "test b ... FAILED",
                "test c ... not run",
                "",
                "failures:",
                "    b: panicked: RUST 3",
                "",
                "test result: FAILED. 1 passed; 1 failed; 1 not run"
            ]
        );
    }

    #[test]
    fn no_case_at_all_is_a_failure() {
        let (out, ok) = LibtestPrint::lines(&[]);
        assert!(
            !ok && out
                .last()
                .unwrap()
                .starts_with("test result: FAILED. 0 passed")
        );
    }
}
