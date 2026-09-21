// The C++ counterpart of `symbian_std::test_report::Report`.
//
// Same file, same path, same JSON shape (`docs/research/cpp-parity.md`), so
// `symdev test --emulator` reads a C++ run exactly as it reads a Rust one.
//
// One deliberate difference from the Rust type, recorded in the parity note: this
// builds the `cases` array incrementally into one growable descriptor, where
// `Report` keeps a `Vec<Case>` of owned `String`s and formats at the end. The C++
// shape is what a Symbian author would write; the heap figures name the gap.
#ifndef SYMDEVREPORT_H
#define SYMDEVREPORT_H

#include <e32base.h>

class CSymdevReport : public CBase
    {
public:
    static CSymdevReport* NewL(const TDesC8& aApp, TUint aUid3);
    ~CSymdevReport();

    void Check(const TDesC8& aName, TBool aOk);
    void CheckDetail(const TDesC8& aName, TBool aOk, const TDesC8& aDetail);
    /// A case from a Symbian error code: `KErrNone` passes, anything else fails and
    /// the code goes in the detail.
    void Checked(const TDesC8& aName, TInt aError);

    TInt Passed() const { return iPassed; }
    TInt Failed() const { return iFailed; }
    TBool IsPass() const { return iFailed == 0 && (iPassed + iFailed) > 0; }

    /// Writes `E:\symdev\results\<uid3>.json`. Returns a Symbian error code.
    TInt Finish();

private:
    CSymdevReport(TUint aUid3);
    void ConstructL(const TDesC8& aApp);
    /// Appends, growing the buffer when it is full; a failed grow drops the text
    /// rather than leaving, exactly as the Rust side discards a formatter error.
    void Add(const TDesC8& aText);
    void AddEscaped(const TDesC8& aText);
    void AddInt(TInt aValue);
    void Record(const TDesC8& aName, TBool aOk, const TDesC8& aDetail);

    TUint iUid3;
    RBuf8 iApp;
    RBuf8 iCases;
    TInt iPassed;
    TInt iFailed;
    };

#endif // SYMDEVREPORT_H
