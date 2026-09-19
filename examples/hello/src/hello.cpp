/* ============================================================================
 Name        : hello.cpp
 Copyright   : Public Domain
 Description : Exe source file
 Copied from: https://fedor4ever.wordpress.com/2026/03/06/how-to-build-symbian-app-by-hand/
 ============================================================================ */
#include "hello.h"
#include <e32base.h>
#include <e32std.h>
#include <e32cons.h>

_LIT(KTextConsoleTitle, "Console");
_LIT(KTextFailed, " failed, leave code = %d");
_LIT(KTextPressAnyKey, " [press any key]\n");

LOCAL_D CConsoleBase* console;

LOCAL_C void MainL()
	{
	console->Write(_L("Hello, world!\n"));
	}

LOCAL_C void DoStartL()
	{
	CActiveScheduler* scheduler = new (ELeave) CActiveScheduler();
	CleanupStack::PushL(scheduler);
	CActiveScheduler::Install(scheduler);
	MainL();
	CleanupStack::PopAndDestroy(scheduler);
	}

GLDEF_C TInt E32Main()
	{
	__UHEAP_MARK;
	CTrapCleanup* cleanup = CTrapCleanup::New();
	TRAPD(
			createError,
			console = Console::NewL(KTextConsoleTitle,
					TSize(KConsFullScreen, KConsFullScreen)));
	if (createError)
		{
		delete cleanup;
		return createError;
		}
	TRAPD(mainError, DoStartL());
	if (mainError)
		console->Printf(KTextFailed, mainError);
	console->Printf(KTextPressAnyKey);
	console->Getch();
	delete console;
	delete cleanup;
	__UHEAP_MARKEND;
	return KErrNone;
	}
