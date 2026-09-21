//! [`join`] and [`race`]: the two ways to have more than one request outstanding from
//! a single task, and the reason an executor exists at all.
//!
//! Both box their halves. A combinator that projects `Pin` into its fields without
//! moving them is the allocation-free form, and it needs either `unsafe` or a pin
//! projection macro this SDK does not carry; two heap blocks on a phone with 128 MB of
//! user RAM is the honest trade, and [`crate::sleep`]'s own request is a third.
use alloc::boxed::Box;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

/// Which side of a [`race`] finished first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Either<L, R> {
    Left(L),
    Right(R),
}

/// Runs both futures at once and finishes when both have.
///
/// This is what makes two `sleep`s take as long as the longer one rather than as long
/// as the two of them: each is polled, each issues its own request, and the scheduler
/// completes them independently.
pub fn join<A: Future, B: Future>(a: A, b: B) -> Join<A, B> {
    Join {
        a: Some(Box::pin(a)),
        b: Some(Box::pin(b)),
        left: None,
        right: None,
    }
}

pub struct Join<A: Future, B: Future> {
    a: Option<Pin<Box<A>>>,
    b: Option<Pin<Box<B>>>,
    left: Option<A::Output>,
    right: Option<B::Output>,
}

/// Both halves are `Pin<Box<_>>` and therefore already `Unpin`; what stops the derived
/// bound is the two finished **outputs** the value holds until its partner arrives. A
/// value that has been produced by a future and moved into this one is no longer
/// pinned, and nothing here ever hands out a pinned reference to it, so moving a
/// half-finished `Join` is safe.
impl<A: Future, B: Future> Unpin for Join<A, B> {}

impl<A: Future, B: Future> Future for Join<A, B> {
    type Output = (A::Output, B::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // `Pin<Box<_>>` is `Unpin`, so this value may be moved and polled through an
        // ordinary `&mut`.
        let this = self.get_mut();
        if let Some(future) = this.a.as_mut()
            && let Poll::Ready(value) = future.as_mut().poll(cx)
        {
            this.left = Some(value);
            this.a = None;
        }
        if let Some(future) = this.b.as_mut()
            && let Poll::Ready(value) = future.as_mut().poll(cx)
        {
            this.right = Some(value);
            this.b = None;
        }
        match (this.left.take(), this.right.take()) {
            (Some(left), Some(right)) => Poll::Ready((left, right)),
            (left, right) => {
                this.left = left;
                this.right = right;
                Poll::Pending
            }
        }
    }
}

/// Runs both futures at once and finishes with the first to complete. **The loser is
/// dropped**, which cancels its outstanding request: that is `CActive::Cancel`, and it
/// is the path that must be right or the kernel writes into a freed request status.
pub fn race<A: Future, B: Future>(a: A, b: B) -> Race<A, B> {
    Race {
        a: Box::pin(a),
        b: Box::pin(b),
    }
}

pub struct Race<A: Future, B: Future> {
    a: Pin<Box<A>>,
    b: Pin<Box<B>>,
}

impl<A: Future, B: Future> Future for Race<A, B> {
    type Output = Either<A::Output, B::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if let Poll::Ready(value) = this.a.as_mut().poll(cx) {
            return Poll::Ready(Either::Left(value));
        }
        if let Poll::Ready(value) = this.b.as_mut().poll(cx) {
            return Poll::Ready(Either::Right(value));
        }
        Poll::Pending
    }
}
