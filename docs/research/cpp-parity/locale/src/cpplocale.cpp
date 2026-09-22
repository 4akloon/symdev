// The C++ counterpart of `symbian-rs/examples/locale`: the same three strings in
// three languages, picked by the device language, plus the same two measurements —
// what `User::Language()` returns here, and what one call costs.
//
// The design is the SDK's, not the Rust one's, and that is the point of the pair:
// the strings live in `\resource\apps\cpplocale.r01|r02|r93`, one compiled file per
// language, and the *loader* picks one by asking the file system for the nearest
// language variant. Rust keeps the table in `.rodata` and picks a column itself.
#include <e32base.h>
#include <f32file.h>
#include <barsc.h>
#include <barsread.h>
#include <bautils.h>

#include "symdevreport.h"
#ifdef SYMDEV_CPP_PARITY_PROBE
#include "symdevprobe.h"
static TSymdevProbe gEntry;
#endif
#include "cpplocale.rsg"

const TUint KUid3 = 0xe00006a3;
const TInt KCalls = 100000;

_LIT(KDir,      "E:\\symdev\\cpplocale\\");
_LIT(KNotes,    "E:\\symdev\\cpplocale\\measured.txt");
_LIT(KResource, "\\resource\\apps\\cpplocale.rsc");

/// The handful of `TLanguage` names this example bothers to print, as on the Rust
/// side. A full table would be 108 lines of no interest.
static const TText8* NameOf(TInt aLanguage)
    {
    switch (aLanguage)
        {
        case ELangTest:          return (const TText8*)"ELangTest";
        case ELangEnglish:       return (const TText8*)"ELangEnglish";
        case ELangFrench:        return (const TText8*)"ELangFrench";
        case ELangGerman:        return (const TText8*)"ELangGerman";
        case ELangUkrainian:     return (const TText8*)"ELangUkrainian";
        case ELangEnglish_Apac:  return (const TText8*)"ELangEnglish_Apac";
        case ELangOther:         return (const TText8*)"ELangOther";
        case ELangNone:          return (const TText8*)"ELangNone";
        default:                 return NULL;
        }
    }

/// One string out of the opened resource file. The caller owns the returned buffer.
static HBufC* ReadStringL(RResourceFile& aFile, TInt aId)
    {
    HBufC8* raw = aFile.AllocReadLC(aId);
    TResourceReader reader;
    reader.SetBuffer(raw);
    HBufC* text = reader.ReadHBufCL();
    CleanupStack::PopAndDestroy(raw);
    return text;
    }

static void AppendLine(TDes8& aNotes, const TDesC8& aKey, const TDesC& aValue)
    {
    aNotes.Append(aKey);
    aNotes.Append('=');
    // The notes file is UTF-8 on the Rust side; here the resource strings are 16-bit,
    // so each unit is narrowed. Non-Latin-1 text would be mangled — the Rust side
    // writes real UTF-8 and this does not, which the parity note records.
    for (TInt i = 0; i < aValue.Length(); i++)
        {
        TUint c = aValue[i];
        if (c < 0x80)
            {
            aNotes.Append((TUint8)c);
            }
        else if (c < 0x800)
            {
            aNotes.Append((TUint8)(0xC0 | (c >> 6)));
            aNotes.Append((TUint8)(0x80 | (c & 0x3F)));
            }
        else
            {
            aNotes.Append((TUint8)(0xE0 | (c >> 12)));
            aNotes.Append((TUint8)(0x80 | ((c >> 6) & 0x3F)));
            aNotes.Append((TUint8)(0x80 | (c & 0x3F)));
            }
        }
    aNotes.Append((TUint8)'\n');
    }

/// Opens the language variant the loader picked and writes its three strings out.
static void ReadTableL(RResourceFile& file, TDes8& aNotes)
    {
    // UNRESOLVED (docs/research/cpp-parity.md, "locale: C++ resource read"):
    // `ConfirmSignatureL` is what teaches `RResourceFile` the NAME offset the `.rsg`
    // ids carry, but on this file, inside EKA2L1, it panics BAFL 4 — with no
    // argument, with 4 (`EEikResourceSignatureValue`), and with a small NAME alike.
    // Without it `Offset()` stays 0 and every read leaves KErrNotFound, which the
    // caller records as a failed case instead of killing the run.

    HBufC* greeting = ReadStringL(file, R_GREETING);
    CleanupStack::PushL(greeting);
    HBufC* languageIs = ReadStringL(file, R_LANGUAGE_IS);
    CleanupStack::PushL(languageIs);
    HBufC* ok = ReadStringL(file, R_OK);
    CleanupStack::PushL(ok);

    AppendLine(aNotes, _L8("GREETING"), *greeting);
    AppendLine(aNotes, _L8("LANGUAGE_IS"), *languageIs);
    AppendLine(aNotes, _L8("OK"), *ok);

    CleanupStack::PopAndDestroy(3);  // ok, languageIs, greeting
    }

static void RunL(CSymdevReport& aReport, RFs& aFs, TDes8& aNotes)
    {
    // 1. What the device is set to, as the device answers it.
    TInt current = User::Language();
    aNotes.AppendFormat(_L8("user_language_raw=%d\n"), current);
    const TText8* name = NameOf(current);
    aNotes.Append(_L8("user_language_name="));
    aNotes.Append(name ? TPtrC8(name) : _L8("<not named by this SDK>"));
    aNotes.Append((TUint8)'\n');
    // There is no dialect/base relation in the C++ API: `TLanguage` is a flat enum and
    // `BaflUtils::NearestLanguageFile` is the only thing that knows the fallback. The
    // Rust `Language::base()` has no counterpart, so the line is written as absent.
    aNotes.Append(_L8("user_language_base=65535\n"));
    aReport.Check(_L8("the language reads the same twice"), User::Language() == current);

    // 2. What one `User::Language()` costs. `User::NTickCount()` is the finest counter
    // this SDK exposes; its period is not stated by any header (see the parity note).
    TUint32 start = User::NTickCount();
    TUint32 sum = 0;
    for (TInt i = 0; i < KCalls; i++)
        {
        sum += (TUint32)User::Language();
        }
    TUint32 asked = User::NTickCount() - start;
    TInt held = current;
    start = User::NTickCount();
    TUint32 sum2 = 0;
    for (TInt i = 0; i < KCalls; i++)
        {
        sum2 += (TUint32)held;
        }
    TUint32 local = User::NTickCount() - start;
    aNotes.AppendFormat(_L8("calls=%d ticks_asking_euser=%u ticks_local=%u\n"),
                        KCalls, asked, local);
    aNotes.AppendFormat(_L8("checksum=%u checksum_local=%u\n"), sum, sum2);

    // 3. The strings themselves, out of the language variant the loader picked.
    // The drive has to be supplied by hand. A path with no drive letter resolves
    // against the session drive, which is not where the package installed the
    // resources; the SDK idiom is to take the drive off the running binary.
    TFileName resource;
    resource.Copy(RProcess().FileName().Left(2));
    resource.Append(KResource);
    BaflUtils::NearestLanguageFile(aFs, resource);
    aNotes.Append(_L8("resource_file="));
    AppendLine(aNotes, _L8(""), resource);

    // Opening and reading the resource file leaves on every error, so it runs inside
    // its own TRAP: a failure has to become a recorded case, not a dead process with
    // no report. The Rust side's `fs`/`Result` chain needs no such bracket.
    RResourceFile file;
    TRAPD(err, file.OpenL(aFs, resource));
    aReport.Checked(_L8("the resource file opens"), err);
    if (err == KErrNone)
        {
        TRAP(err, ReadTableL(file, aNotes));
        aReport.Checked(_L8("the table came through"), err);
        }
    file.Close();

    // 4. The fallback chain. There is no way to ask for a language the device is not
    // set to: `NearestLanguageFile` reads `User::Language()` itself, and rcomp has
    // already thrown away every variant but the ones on the `LANG` line. So the six
    // cases the Rust example runs against `Text::get_in(language)` cannot be written
    // here at all, and the chain is exercised only for whatever the device is set to.
    aReport.CheckDetail(_L8("the fallback chain can only be exercised for the device language"),
                        ETrue, _L8("no per-language lookup exists in the C++ API"));

#ifdef SYMDEV_CPP_PARITY_PROBE
    TSymdevProbe end = TSymdevProbe::Now();
    TBuf8<64> detail;
    detail.Format(_L8("cells %d bytes %d"), gEntry.iCells, gEntry.iBytes);
    aReport.CheckDetail(_L8("probe:heap at entry"), ETrue, detail);
    detail.Format(_L8("cells %d bytes %d"), end.iCells, end.iBytes);
    aReport.CheckDetail(_L8("probe:heap at end"), ETrue, detail);
    detail.Format(_L8("%u"), end.TicksSince(gEntry));
    aReport.CheckDetail(_L8("probe:nanoticks entry to end"), ETrue, detail);
    detail.Format(_L8("%d us"), TSymdevProbe::SystemTickPeriodMicros());
    aReport.CheckDetail(_L8("probe:UserHal::TickPeriod"), ETrue, detail);
#endif
    }

static void MainL()
    {
    RFs fs;
    User::LeaveIfError(fs.Connect());
    CleanupClosePushL(fs);

    RBuf8 notes;
    notes.CreateL(1024);
    CleanupClosePushL(notes);

    CSymdevReport* report = CSymdevReport::NewL(_L8("locale"), KUid3);
    CleanupStack::PushL(report);

    RunL(*report, fs, notes);

    TInt err = fs.MkDirAll(KDir);
    if (err == KErrAlreadyExists)
        {
        err = KErrNone;
        }
    if (err == KErrNone)
        {
        RFile out;
        err = out.Replace(fs, KNotes, EFileWrite);
        if (err == KErrNone)
            {
            err = out.Write(notes);
            out.Close();
            }
        }
    report->Checked(_L8("the measurements are written out"), err);

    TInt written = report->Finish();
    TBool pass = report->IsPass() && written == KErrNone;
    CleanupStack::PopAndDestroy(3);  // report, notes, fs
    User::Exit(pass ? 0 : 1);
    }

GLDEF_C TInt E32Main()
    {
#ifdef SYMDEV_CPP_PARITY_PROBE
    gEntry = TSymdevProbe::Now();
#endif
    CTrapCleanup* cleanup = CTrapCleanup::New();
    if (cleanup == NULL)
        {
        return KErrNoMemory;
        }
    TRAPD(err, MainL());
    delete cleanup;
    return err;
    }
