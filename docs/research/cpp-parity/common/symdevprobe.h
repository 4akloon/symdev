// The C++ half of the heap/startup probe (`docs/research/cpp-parity.md`).
//
// Header-only and deliberately dumb: two euser calls and a subtraction, the same two
// the Rust half (`docs/research/cpp-parity/probe.rs`) makes, in the same order.
//
// `User::AllocSize(TInt&)` (e32std.h line 4484) returns the number of allocated cells
// and writes the total bytes across them. That is the "bytes allocated" figure.
// `RHeap::Size()` is not: e32cmn.inl line 78 defines it as "the total number of bytes
// committed by the host chunk", which moves in page-sized steps.
//
// `User::NTickCount()` is a free-running TUint32 whose period no header in this SDK
// states. `UserHal::TickPeriod` states the *system* tick period instead, and
// `SystemTickPeriodMicros()` below reads it so the note can quote a measured rate
// rather than assume milliseconds.
#ifndef SYMDEVPROBE_H
#define SYMDEVPROBE_H

#include <e32std.h>
#include <e32hal.h>

class TSymdevProbe
    {
public:
    static TSymdevProbe Now()
        {
        TSymdevProbe p;
        TInt bytes = 0;
        p.iCells = User::AllocSize(bytes);
        p.iBytes = bytes;
        p.iTicks = User::NTickCount();
        return p;
        }

    TUint32 TicksSince(const TSymdevProbe& aEarlier) const
        {
        return iTicks - aEarlier.iTicks;
        }

    /// `UserHal::TickPeriod` in microseconds, or the negative error code.
    static TInt SystemTickPeriodMicros()
        {
        TTimeIntervalMicroSeconds32 period(0);
        TInt err = UserHal::TickPeriod(period);
        return err == KErrNone ? period.Int() : err;
        }

    TInt iCells;
    TInt iBytes;
    TUint32 iTicks;
    };

#endif // SYMDEVPROBE_H
