// The C++ counterpart of `symbian-rs/examples/files`: write a file, close it, reopen
// it, read it back, compare, seek, rename, delete — and write the same JSON result
// file the Rust example writes, so `symdev test` reads either run.
//
// The Rust side reaches these through `symbian_std::fs` / `symbian_std::io`, which is
// `std`'s vocabulary; this side is the F32 client API the Rust one sits on.
#include <e32base.h>
#include <f32file.h>

#include "symdevreport.h"

const TUint KUid3 = 0xe00006a1;

_LIT(KDir,     "E:\\symdev\\cppfiles\\");
_LIT(KPath,    "E:\\symdev\\cppfiles\\roundtrip.bin");
_LIT(KRenamed, "E:\\symdev\\cppfiles\\renamed.bin");
_LIT(KKept,    "E:\\symdev\\cppfiles\\kept.bin");
_LIT(KMissing, "E:\\symdev\\cppfiles\\not-here.bin");

// The same bytes as the Rust example, including the UTF-8 em dash, so the round trip
// is not just ASCII.
_LIT8(KBytes, "symdev step 71 \xe2\x80\x94 files through symbian-std.\n");

/// Write the file and close it, so the re-open below really is a second look at the
/// disk. `RFile::Flush` is what `File::sync_all` calls.
static TInt WriteIt(RFs& aFs)
    {
    RFile file;
    TInt err = file.Replace(aFs, KPath, EFileWrite);
    if (err != KErrNone)
        {
        return err;
        }
    err = file.Write(KBytes);
    if (err == KErrNone)
        {
        err = file.Flush();
        }
    file.Close();
    return err;
    }

/// Re-open and read the whole file, as `read_to_end` does.
static TInt ReadIt(RFs& aFs, TDes8& aOut)
    {
    RFile file;
    TInt err = file.Open(aFs, KPath, EFileRead);
    if (err != KErrNone)
        {
        return err;
        }
    err = file.Read(aOut);
    file.Close();
    return err;
    }

/// Seek to byte 7 and read three bytes, which `read_to_end` alone would not prove.
static TInt ReadFromMiddle(RFs& aFs, TDes8& aOut)
    {
    RFile file;
    TInt err = file.Open(aFs, KPath, EFileRead);
    if (err != KErrNone)
        {
        return err;
        }
    TInt pos = 7;
    err = file.Seek(ESeekStart, pos);
    if (err == KErrNone)
        {
        err = file.Read(aOut, 3);
        }
    file.Close();
    return err;
    }

static void RunL(CSymdevReport& aReport, RFs& aFs)
    {
    aReport.Checked(_L8("create_dir_all"), aFs.MkDirAll(KDir));
    // An existing directory is success in `std`; F32 says KErrAlreadyExists, so the
    // idempotence the Rust side gets for free is one branch here.
    TInt again = aFs.MkDirAll(KDir);
    aReport.Check(_L8("create_dir_all is idempotent"),
                  again == KErrNone || again == KErrAlreadyExists);

    aReport.Checked(_L8("write and close"), WriteIt(aFs));

    TEntry entry;
    TInt err = aFs.Entry(KPath, entry);
    aReport.Checked(_L8("metadata"), err);
    if (err == KErrNone)
        {
        aReport.Check(_L8("metadata.len is what was written"),
                      entry.iSize == KBytes().Length());
        aReport.Check(_L8("metadata says file, not dir"), !entry.IsDir());
        }

    TBuf8<128> back;
    err = ReadIt(aFs, back);
    aReport.Checked(_L8("read back"), err);
    if (err == KErrNone)
        {
        aReport.Check(_L8("the bytes are the bytes"), back == KBytes());
        }

    TBuf8<3> three;
    err = ReadFromMiddle(aFs, three);
    aReport.Checked(_L8("seek and read_exact"), err);
    if (err == KErrNone)
        {
        aReport.Check(_L8("seek landed where it said"),
                      three == KBytes().Mid(7, 3));
        }

    RFile missing;
    err = missing.Open(aFs, KMissing, EFileRead);
    if (err == KErrNone)
        {
        missing.Close();
        aReport.CheckDetail(_L8("opening a missing file fails"), EFalse, _L8("it opened"));
        }
    else
        {
        // There is no `ErrorKind` here: the classification the Rust side gets from
        // `io::Error::kind()` is this comparison, written out by hand.
        aReport.Check(_L8("a missing file is NotFound"), err == KErrNotFound);
        aReport.Check(_L8("and keeps its TInt"), err == -1);
        }

    RFile kept;
    err = kept.Replace(aFs, KKept, EFileWrite);
    if (err == KErrNone)
        {
        err = kept.Write(KBytes);
        kept.Close();
        }
    aReport.Checked(_L8("fs::write leaves a file behind"), err);

    aReport.Checked(_L8("rename"), aFs.Rename(KPath, KRenamed));
    aReport.Checked(_L8("remove_file"), aFs.Delete(KRenamed));
    TEntry gone;
    aReport.Check(_L8("the file is gone"), aFs.Entry(KRenamed, gone) == KErrNotFound);
    }

static void MainL()
    {
    RFs fs;
    User::LeaveIfError(fs.Connect());
    CleanupClosePushL(fs);
    CSymdevReport* report = CSymdevReport::NewL(_L8("files"), KUid3);
    CleanupStack::PushL(report);
    RunL(*report, fs);
    TInt written = report->Finish();
    TBool pass = report->IsPass() && written == KErrNone;
    CleanupStack::PopAndDestroy(2);  // report, fs
    User::Exit(pass ? 0 : 1);
    }

GLDEF_C TInt E32Main()
    {
    CTrapCleanup* cleanup = CTrapCleanup::New();
    if (cleanup == NULL)
        {
        return KErrNoMemory;
        }
    TRAPD(err, MainL());
    delete cleanup;
    return err;
    }
