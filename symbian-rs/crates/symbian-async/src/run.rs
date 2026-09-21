//! The two ways in: [`block_on`], which owns a scheduler, and [`spawn`], which joins
//! one.
use alloc::sync::Arc;
use core::future::Future;
use core::task::{Context, Poll, Waker};

use symbian_core::Result;

use crate::executor::Executor;
use crate::scheduler::Scheduler;
use crate::task::Task;
use crate::waker::Readiness;

/// Runs one future to completion on a scheduler of its own, and returns what it
/// produced. This is the **console** shape, and it reads like
/// `futures::executor::block_on`:
///
/// ```ignore
/// #[symbian_std::main]
/// fn main() -> Result<i32> {
///     block_on(async {
///         sleep(Duration::from_millis(300)).await?;
///         Ok::<(), SymbianError>(())
///     })??;
///     Ok(0)
/// }
/// ```
///
/// The future is polled on this stack, so it need not be `'static` and nothing is
/// allocated for it. Between polls the thread is inside `CActiveScheduler::Start()`,
/// which is where every other active object in the program gets to run too.
///
/// # A GUI application must not call this
///
/// It installs a `CActiveScheduler`, and an Avkon application already has one: CONE's.
/// There it returns `KErrInUse` — and that is the whole diagnosis, so use [`spawn`].
///
/// # Errors
///
/// `KErrInUse` when a scheduler is already installed on this thread, `KErrNoMemory`
/// when one cannot be created, and any leave `CActiveScheduler::Start()` produced.
/// Whatever the future itself returns comes back inside `Ok`.
///
/// # What it does not do
///
/// A future that returns `Pending` without having registered its waker with anything
/// leaves the thread inside `Start()` for ever, because no request will ever complete
/// — this executor has no timeout of its own and no way to invent one. Tasks
/// [`spawn`]ed beside the future are dropped, and so cancelled, when it finishes.
pub fn block_on<F: Future>(future: F) -> Result<F::Output> {
    let scheduler = Scheduler::install()?;
    let executor = Executor::get();
    let ready = Readiness::new();
    let waker = ready.waker();
    let previous = executor.set_root(Some(ready.clone()));
    let result = drive(&scheduler, future, &ready, &waker);
    executor.set_root(previous);
    result
}

/// The loop: poll while the root has been woken, otherwise hand the thread to the
/// scheduler until a completion wakes it.
fn drive<F: Future>(
    scheduler: &Scheduler,
    future: F,
    ready: &Arc<Readiness>,
    waker: &Waker,
) -> Result<F::Output> {
    let mut future = core::pin::pin!(future);
    let mut cx = Context::from_waker(waker);
    loop {
        if ready.take()
            && let Poll::Ready(value) = future.as_mut().poll(&mut cx)
        {
            return Ok(value);
        }
        // A future that woke itself during the poll — `yield`-shaped work — must not
        // send the thread into a wait nothing will end.
        if !ready.is_set() {
            scheduler.run()?;
        }
    }
}

/// Adds a task to the executor on **this thread's existing scheduler** and polls it
/// once, which is what gets its first request issued.
///
/// This is the shape for an application that does not own its event loop — an Avkon
/// one, where CONE installed `CCoeScheduler` and is running it (step 75). Nothing here
/// starts or stops a scheduler; the task simply runs whenever the one that is already
/// there gets to it.
///
/// Inside [`block_on`] it works the same way, and that is the only form this SDK can
/// test until the Avkon framework lands: the task runs concurrently with the future
/// `block_on` is polling, on the scheduler `block_on` installed.
///
/// # Errors
///
/// `KErrNotReady` when this thread has no `CActiveScheduler` at all; `KErrInUse` when
/// the executor is already bound to another thread's.
pub fn spawn(future: impl Future<Output = ()> + 'static) -> Result<()> {
    let executor = Executor::get();
    executor.bind()?;
    executor.spawn(Task::new(future));
    Ok(())
}
