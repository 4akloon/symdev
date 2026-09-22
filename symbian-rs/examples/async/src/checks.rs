//! The cases, in the order they build on each other.
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};
use core::fmt::Write as _;
use core::time::Duration;

use symbian_async::{Either, block_on, join, race, sleep, spawn};
use symbian_core::{ErrorKind, Result, SymbianError};
use symbian_std::test_report::Report;
use symbian_std::writeln;

use crate::{SLACK_MS, millis, start_of};

/// What a concurrent pair of these must not take twice of.
const SPAN_MS: u64 = 300;

pub(crate) fn run(report: &mut Report, notes: &mut String) {
    a_value_comes_back(report);
    one_timer(report, notes);
    two_at_once(report, notes);
    one_after_the_other(report, notes);
    the_shorter_one_finishes_first(report);
    the_loser_of_a_race_is_cancelled(report, notes);
    a_spawned_task_runs_beside_the_root(report);
    spawn_needs_a_scheduler(report);
    block_on_does_not_nest(report);
    a_blocking_wait_is_refused(report);
    too_long_a_sleep(report);
    the_scheduler_can_be_owned_again(report, notes);
}

/// The executor at its smallest: a future that is ready on the first poll never
/// touches the scheduler at all.
fn a_value_comes_back(report: &mut Report) {
    let got = block_on(async { 42u32 });
    report.check_detail(
        "block_on returns what the future produced",
        got == Ok(42),
        format_args!("{got:?}"),
    );
}

/// One request, end to end: `RTimer::After` issued from a poll, completed by the
/// scheduler, delivered to the waker, awaited.
fn one_timer(report: &mut Report, notes: &mut String) {
    let case = "one 300 ms sleep completes";
    let Some(start) = start_of(report, case) else {
        return;
    };
    let outcome = block_on(async { sleep(Duration::from_millis(SPAN_MS)).await });
    let took = millis(start.elapsed());
    let _ = writeln!(notes, "one_timer_ms={took}");
    report.check_detail(case, outcome == Ok(Ok(())), format_args!("{outcome:?}"));
    report.check_detail(
        "one 300 ms sleep takes about 300 ms",
        near(took, SPAN_MS),
        format_args!("{took} ms"),
    );
}

/// **The criterion of step 73.** Two timers of the same length, awaited together,
/// finish in the time of one — which is only true if both requests were outstanding at
/// once, and that is exactly what a blocking `User::WaitForRequest` cannot do.
fn two_at_once(report: &mut Report, notes: &mut String) {
    let case = "two 300 ms sleeps awaited together both complete";
    let Some(start) = start_of(report, case) else {
        return;
    };
    let outcome = block_on(async {
        let (first, second) = join(
            sleep(Duration::from_millis(SPAN_MS)),
            sleep(Duration::from_millis(SPAN_MS)),
        )
        .await;
        first.and(second)
    });
    let took = millis(start.elapsed());
    let _ = writeln!(notes, "two_at_once_ms={took}");
    report.check_detail(case, outcome == Ok(Ok(())), format_args!("{outcome:?}"));
    report.check_detail(
        "two 300 ms sleeps awaited together take about 300 ms, not 600",
        near(took, SPAN_MS),
        format_args!("{took} ms"),
    );
}

/// The control for [`two_at_once`]: the same two sleeps one after the other really do
/// cost twice as much, so the figure above is concurrency and not a fast timer.
fn one_after_the_other(report: &mut Report, notes: &mut String) {
    let case = "the same two sleeps in sequence take about 600 ms";
    let Some(start) = start_of(report, case) else {
        return;
    };
    let outcome = block_on(async {
        sleep(Duration::from_millis(SPAN_MS)).await?;
        sleep(Duration::from_millis(SPAN_MS)).await
    });
    let took = millis(start.elapsed());
    let _ = writeln!(notes, "one_after_the_other_ms={took}");
    report.check_detail(
        case,
        outcome == Ok(Ok(())) && near(took, 2 * SPAN_MS),
        format_args!("{took} ms, {outcome:?}"),
    );
}

/// "In the right order": the 100 ms timer of a concurrent pair completes before the
/// 400 ms one, which is the scheduler choosing between two outstanding requests.
fn the_shorter_one_finishes_first(report: &mut Report) {
    let order = Rc::new(RefCell::new(Vec::new()));
    let (slow, quick) = (order.clone(), order.clone());
    let outcome = block_on(async move {
        let (first, second) = join(
            async move {
                let done = sleep(Duration::from_millis(400)).await;
                slow.borrow_mut().push(400u32);
                done
            },
            async move {
                let done = sleep(Duration::from_millis(100)).await;
                quick.borrow_mut().push(100u32);
                done
            },
        )
        .await;
        first.and(second)
    });
    let finished = order.borrow();
    report.check_detail(
        "the shorter of two concurrent timers finishes first",
        outcome == Ok(Ok(())) && finished.as_slice() == [100u32, 400],
        format_args!("{:?}, {outcome:?}", finished.as_slice()),
    );
}

/// Dropping a future with a request outstanding is `CActive::Cancel`, and it is the
/// path that must be right or the kernel writes into a freed request status. A race
/// whose loser is a 20-second timer returns in a tenth of a second and the program
/// carries on — the cases after this one are the proof that it did.
fn the_loser_of_a_race_is_cancelled(report: &mut Report, notes: &mut String) {
    let case = "a race finishes with the first timer and cancels the other";
    let Some(start) = start_of(report, case) else {
        return;
    };
    let outcome = block_on(async {
        race(
            sleep(Duration::from_millis(100)),
            sleep(Duration::from_secs(20)),
        )
        .await
    });
    let took = millis(start.elapsed());
    let _ = writeln!(notes, "race_ms={took}");
    report.check_detail(
        case,
        matches!(outcome, Ok(Either::Left(Ok(())))) && took < 1_000,
        format_args!("{took} ms, {outcome:?}"),
    );
}

/// The **joined** shape: `spawn` adds a task to the scheduler that is already running
/// and never starts or stops one. Here that scheduler is `block_on`'s; in an Avkon
/// application it is CONE's, and the code is the same.
fn a_spawned_task_runs_beside_the_root(report: &mut Report) {
    let done = Rc::new(Cell::new(false));
    let flag = done.clone();
    let outcome: Result<Result<()>> = block_on(async move {
        spawn(async move {
            if sleep(Duration::from_millis(100)).await.is_ok() {
                flag.set(true);
            }
        })?;
        sleep(Duration::from_millis(SPAN_MS)).await
    });
    report.check_detail(
        "a spawned task runs on the same scheduler as the root future",
        outcome == Ok(Ok(())) && done.get(),
        format_args!("done={}, {outcome:?}", done.get()),
    );
}

/// With no scheduler on the thread there is nothing to add a `CActive` to, and that is
/// an error rather than the panic `CActiveScheduler::Add` would raise.
fn spawn_needs_a_scheduler(report: &mut Report) {
    let outcome = spawn(async {});
    report.check_detail(
        "spawn outside any scheduler is refused",
        outcome.map_err(SymbianError::kind) == Err(ErrorKind::NotReady),
        format_args!("{outcome:?}"),
    );
}

/// A second `CActiveScheduler::Install` on one thread panics, so `block_on` refuses
/// first — which is also the error a GUI application gets, where CONE owns the one
/// that is already there.
fn block_on_does_not_nest(report: &mut Report) {
    let outcome = block_on(async { block_on(async { 1u32 }) });
    report.check_detail(
        "a nested block_on is refused rather than installing a second scheduler",
        matches!(&outcome, Ok(Err(error)) if error.kind() == ErrorKind::InUse),
        format_args!("{outcome:?}"),
    );
}

/// The two models must not share a thread: `User::WaitForRequest` would consume a
/// completion belonging to an active object, and neither side would say so. Under a
/// scheduler the blocking helper refuses before it waits.
fn a_blocking_wait_is_refused(report: &mut Report) {
    let outcome = block_on(async { symbian_core::net::blocking(|_status| {}) });
    report.check_detail(
        "a blocking User::WaitForRequest under a scheduler is refused",
        matches!(&outcome, Ok(Err(error)) if error.kind() == ErrorKind::InUse),
        format_args!("{outcome:?}"),
    );
}

/// `RTimer::After` takes a `TInt` of microseconds, so anything past 35 minutes 47
/// seconds is an error and not a silently shorter sleep.
fn too_long_a_sleep(report: &mut Report) {
    let outcome = block_on(async { sleep(Duration::from_secs(3600)).await });
    report.check_detail(
        "a sleep longer than RTimer::After can express is an error",
        matches!(&outcome, Ok(Err(error)) if error.kind() == ErrorKind::Overflow),
        format_args!("{outcome:?}"),
    );
}

/// Twenty scheduler lifetimes in a row: each `block_on` installs one, opens a timer,
/// closes it and uninstalls. A handle or an active object left behind would show here
/// and nowhere else.
fn the_scheduler_can_be_owned_again(report: &mut Report, notes: &mut String) {
    const ROUNDS: u32 = 20;
    let case = "a scheduler can be installed, run and uninstalled twenty times over";
    let Some(start) = start_of(report, case) else {
        return;
    };
    let mut failed = None;
    for round in 0..ROUNDS {
        let outcome = block_on(async { sleep(Duration::from_millis(1)).await });
        if outcome != Ok(Ok(())) {
            failed = Some((round, outcome));
            break;
        }
    }
    let took = millis(start.elapsed());
    let _ = writeln!(notes, "twenty_rounds_ms={took}");
    report.check_detail(
        case,
        failed.is_none(),
        format_args!("{failed:?} after {took} ms"),
    );
}

/// Whether a measurement is the duration it should be, allowing for the 15 625 µs
/// system tick at both ends of `RTimer::After`.
fn near(measured: u64, expected: u64) -> bool {
    measured + SLACK_MS >= expected && measured <= expected + SLACK_MS
}
