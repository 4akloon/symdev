#include "symdevreport.h"

#include <f32file.h>

_LIT(KResultsDir, "E:\\symdev\\results\\");
_LIT8(KSchemaHead, "{\"schema\":1,\"app\":\"");

CSymdevReport* CSymdevReport::NewL(const TDesC8& aApp, TUint aUid3)
    {
    CSymdevReport* self = new (ELeave) CSymdevReport(aUid3);
    CleanupStack::PushL(self);
    self->ConstructL(aApp);
    CleanupStack::Pop(self);
    return self;
    }

CSymdevReport::CSymdevReport(TUint aUid3)
    : iUid3(aUid3), iPassed(0), iFailed(0)
    {
    }

void CSymdevReport::ConstructL(const TDesC8& aApp)
    {
    iApp.CreateL(aApp);
    iCases.CreateL(256);
    }

CSymdevReport::~CSymdevReport()
    {
    iApp.Close();
    iCases.Close();
    }

void CSymdevReport::Add(const TDesC8& aText)
    {
    if (iCases.Length() + aText.Length() > iCases.MaxLength())
        {
        TInt want = (iCases.Length() + aText.Length()) * 2;
        if (iCases.ReAlloc(want) != KErrNone)
            {
            return;
            }
        }
    iCases.Append(aText);
    }

void CSymdevReport::AddEscaped(const TDesC8& aText)
    {
    for (TInt i = 0; i < aText.Length(); i++)
        {
        TUint8 c = aText[i];
        switch (c)
            {
            case '"':  Add(_L8("\\\"")); break;
            case '\\': Add(_L8("\\\\")); break;
            case '\n': Add(_L8("\\n")); break;
            case '\r': Add(_L8("\\r")); break;
            case '\t': Add(_L8("\\t")); break;
            default:
                if (c < 0x20)
                    {
                    TBuf8<8> hex;
                    hex.Format(_L8("\\u%04x"), c);
                    Add(hex);
                    }
                else
                    {
                    Add(TPtrC8(&aText[i], 1));
                    }
            }
        }
    }

void CSymdevReport::AddInt(TInt aValue)
    {
    TBuf8<16> digits;
    digits.Num(aValue);
    Add(digits);
    }

void CSymdevReport::Record(const TDesC8& aName, TBool aOk, const TDesC8& aDetail)
    {
    if (iPassed + iFailed > 0)
        {
        Add(_L8(","));
        }
    Add(_L8("{\"name\":\""));
    AddEscaped(aName);
    Add(aOk ? _L8("\",\"ok\":true") : _L8("\",\"ok\":false"));
    if (aDetail.Length() > 0)
        {
        Add(_L8(",\"detail\":\""));
        AddEscaped(aDetail);
        Add(_L8("\""));
        }
    Add(_L8("}"));
    if (aOk)
        {
        iPassed++;
        }
    else
        {
        iFailed++;
        }
    }

void CSymdevReport::Check(const TDesC8& aName, TBool aOk)
    {
    Record(aName, aOk, KNullDesC8);
    }

void CSymdevReport::CheckDetail(const TDesC8& aName, TBool aOk, const TDesC8& aDetail)
    {
    Record(aName, aOk, aDetail);
    }

void CSymdevReport::Checked(const TDesC8& aName, TInt aError)
    {
    if (aError == KErrNone)
        {
        Record(aName, ETrue, KNullDesC8);
        }
    else
        {
        TBuf8<32> detail;
        detail.Format(_L8("TInt %d"), aError);
        Record(aName, EFalse, detail);
        }
    }

TInt CSymdevReport::Finish()
    {
    RBuf8 json;
    TInt err = json.Create(iCases.Length() + iApp.Length() + 128);
    if (err != KErrNone)
        {
        return err;
        }
    json.Append(KSchemaHead);
    json.Append(iApp);
    json.AppendFormat(_L8("\",\"uid3\":\"0x%08x\",\"passed\":%d,\"failed\":%d,\"cases\":["),
                      iUid3, iPassed, iFailed);
    json.Append(iCases);
    json.Append(_L8("]}"));

    RFs fs;
    err = fs.Connect();
    if (err != KErrNone)
        {
        json.Close();
        return err;
        }
    TFileName path;
    path.Append(KResultsDir);
    err = fs.MkDirAll(path);
    if (err != KErrNone && err != KErrAlreadyExists)
        {
        fs.Close();
        json.Close();
        return err;
        }
    path.AppendFormat(_L("%08x.json"), iUid3);
    RFile file;
    err = file.Replace(fs, path, EFileWrite);
    if (err == KErrNone)
        {
        err = file.Write(json);
        if (err == KErrNone)
            {
            err = file.Flush();
            }
        file.Close();
        }
    fs.Close();
    json.Close();
    return err;
    }
