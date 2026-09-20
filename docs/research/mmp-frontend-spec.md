# `bld.inf` / `.mmp` front end — behavioural specification for a native reimplementation

Written 2026-09-20 by a separate agent (the "spec writer") from the S60 3rd Edition FP2 SDK's own
build system (the Perl programs and modules under `/home/genius/sdk/S60_3rd_FP2/epoc32/tools/`),
from the SDK's shipped configuration and header files, and from black-box runs of the SDK's
preprocessor under Wine. The engineer who implements the native front end did not read that Perl,
nor any scratch notes; this document is the only handover artefact. It contains prose, tables and
observed command lines only — no code from the sources, no identifiers from them.

Revised 2026-09-20 after experiment 63, which got the SDK's own generator running on this Linux
host and captured the makefile it produces for four projects. Sections marked **(exp 63)** were
re-measured against those captures; §14.8 and §15 say what that does and does not settle.

Facts below are marked:

* **(read)** — derived by reading the SDK's build programs. Deterministic, but not re-run.
* **(exp)** — confirmed by running a real SDK tool on crafted input; the run is recorded in §14.
* **(exp 63)** — confirmed against a makefile the SDK's own generator produced on this Linux host,
  in experiment 63 of [experiment-backlog.md](experiment-backlog.md). That experiment made the
  SDK's two generator programs run under Linux perl and captured the makefile for four projects;
  §14.8 says what it covers and where it cannot be trusted.
* **unknown** — the sources leave it open and no cheap experiment settled it. Never guess these.

---

## 0. Scope

This specification covers what an implementer needs to build **one S60 3rd Edition FP2
application** on a Linux host, for the **GCCE** platform, **ARMV5** ABI, **UREL** variant, from a
`bld.inf` plus one or more `.mmp` files. It describes:

* the whole front end (preprocessing, lexing, both grammars, path resolution, defaults);
* what the front end's output means for the compile, the link and the post-link.

Out of scope, and called out where they appear: the emulator platforms (`WINSCW`, `WINS`, `X86`),
the RVCT platforms (`ARMV5`/`ARMV5_ABIV2`/`ARMV4`, compiler key `ARMCC`), the tool platforms
(`TOOLS`, `TOOLS2`, `CWTOOLS`), the IDE pseudo-platforms (`VC6`, `VS6`, `VS2003`, `CW_IDE`),
`GCCXML`, `EDG`, kernel-side target types, feature variants, function-call logging, multifile
compilation, and the `abld` command layer above the two programs described here.

The SDK front end is two programs:

| Stage | Input | Output |
|---|---|---|
| Project-file stage | `bld.inf` | the list of platforms, the list of `.mmp` files, the export copies |
| Makefile stage | one `.mmp` + one platform | one makefile per `(component, platform)` covering both UREL and UDEB |

A native builder does not have to produce makefiles; it needs the *model* both stages compute.
Everything below describes that model.

---

## 1. Preprocessing

### 1.1 Which preprocessor

Both `bld.inf` and `.mmp` are passed through a **C preprocessor** before any parsing. The SDK uses
the `cpp.exe` that ships in `epoc32/gcc/bin/`. It identifies itself as **GNU CPP version
2.9-psion-98r2 (Symbian build 546) (ARM/EPOC/PE)** **(exp)**. This matters: it is a GCC 2.x
preprocessor, not a modern one.

An alternative preprocessor (a binary named `scpp`, same directory convention) is selected when the
environment variable `ALT_PRE` is exactly `1` **(read)**. Treat that as out of scope; if `ALT_PRE=1`
is set in the environment, refuse with a clear message rather than silently using a different
preprocessor.

### 1.2 The command line

The invocation is, in order **(read, confirmed exp, confirmed exp 63)**:

```
cpp -undef -nostdinc -+ \
    -I "<EPOCROOT>\epoc32\include" \
    -I . \
    -I "<directory containing the project file>" \
    [-D <MACRO>=_____<MACRO> ...] \
    [-I "<directory of the variant header>" -include "<variant header>"] \
    "<project file>"
```

Notes:

* `-undef` removes **all** built-in macros. Nothing like `__GNUC__`, `__unix__`, `__SYMBIAN32__`,
  `_UNICODE` or `NDEBUG` is defined **(exp)**.
* `-nostdinc` removes the compiler's own include directories. The only include directories are the
  three (or four) `-I` arguments above.
* `-+` puts the preprocessor in C++ mode, which is what makes `//` comments work.
* `.` is the current working directory — in practice the `group/` directory the build was started
  from, **not** the directory of the `.mmp` when that differs.
* The three fixed `-I` directories are in that order: SDK include, cwd, project-file directory.
* Every macro is passed as `NAME=_____NAME` — the value is the name with **five leading
  underscores**. This is a deliberate trick so the front end can tell an expansion apart from text
  the author wrote (§1.4).
* On Windows the absolute paths are prefixed with the drive letter of the current directory and
  quoted. On a Linux host there is no drive letter; a native implementation resolves paths itself
  and never needs this.
* Details the captured invocations add **(exp 63)**: the program is named `cpp.EXE` with the
  extension in capitals; the third `-I` keeps the trailing separator of the project-file directory
  while the first two do not; `-D` and its argument are two separate arguments, so the spelling is
  `-D NAME=_____NAME`; and the whole quoted-path style is `"…"` around each absolute path, which
  under `cmd.exe` leaves a trailing separator harmless and under a POSIX shell does not.

### 1.3 The macros defined for this configuration

The macro set is derived from the platform, and it is **not** the same set the compiler later sees.

For **GCCE** the six macros passed to the preprocessor of a `.mmp` (and of the second pass over
`bld.inf`) are, in this order **(read, confirmed exp 63)**:

| Macro | Where it comes from |
|---|---|
| `GCCE` | the compiler key for this platform |
| `EPOC32` | the target OS |
| `MARM` | the CPU family |
| `EABI` | this platform uses the EABI `.def`/ABI conventions |
| `GENERIC_MARM` | the platform is "generic" (no specific ASSP) and the CPU is `MARM` |
| `MARM_ARMV5` | the platform is generic, CPU `MARM`, and the ABI is `ARMV5` |

Not passed, and therefore **undefined inside a `.mmp`**: `UREL`, `UDEB`, `NDEBUG`, `_DEBUG`,
`_UNICODE`, `__SYMBIAN32__`, `WINSCW`, `ARMV5`, `ARMCC`, and every `__NAME__`-spelled macro
**(exp)**. In particular:

* **There is no way to write a UREL-only or UDEB-only line in a `.mmp`.** The `.mmp` is read once
  per platform and the resulting model covers both build variants.
* `#ifdef ARMV5` is false on a GCCE build even though the ABI is ARMV5; the spelling that is
  defined is `MARM_ARMV5`.

The macro name for a BSF-customised platform would additionally be defined **(read)**; no BSF
platform in this SDK survives validation, so that case does not arise here. **Correction
(exp 63):** this previously said there are no BSF files at all. There are three, and the generator
rejects each of them by name on every run with a warning that the specification is incomplete, so
the effect is the same but the noise is not — a native builder that mirrors this SDK will see those
three names and should not be surprised by them.

The other platforms' macro sets were visible in the same capture **(exp 63)**, and one of them is
worth recording because it is easy to guess wrong: the `ARMV5` pass defines `GCC32`, not `ARMCC`.
`ARMCC` is the `OPTION` key for the RVCT platforms (§8.5), not a preprocessor macro. The four
platforms this SDK iterates are `WINSCW` (`CW32`, `WINS`, `WINSCW`), `GCCXML`
(`GCC32`, `EPOC32`, `MARM`, `GCCXML`, `GENERIC_MARM`, `MARM_ARM4`), `ARMV5`
(`GCC32`, `EPOC32`, `MARM`, `GENERIC_MARM`, `MARM_ARMV5`) and `GCCE`.

### 1.4 Restoring the macro text

Because each macro expands to `_____NAME`, any *argument* that happened to be a macro name comes
back as `_____NAME`. The front end undoes this before tokenising, replacing `_____NAME` with `NAME`
and absorbing one following space if present (the 2.x preprocessor inserts a space after an
expansion) **(read)**. A leading `/` immediately before the expansion is preserved.

Practical consequence: writing `OPTION GCCE -O3` in a `.mmp` survives intact **(exp)** — the
expansion to `_____GCCE ` is reversed. An implementer that does not play the underscore trick gets
the same result by simply **not** substituting inside the directive's arguments; the trick exists
only because the SDK reuses a general-purpose preprocessor.

### 1.5 The variant header

The SDK reads `epoc32/tools/variant/variant.cfg`, takes the first line naming a `.hrh` file, and
resolves it relative to `EPOCROOT`. In this SDK that line names
`epoc32/include/variant/symbian_os_v9.3.hrh` **(read)**.

That header is **force-included into every `.mmp` and `bld.inf` preprocessing run**, and its
directory is added as a fourth `-I` **(read)**. It defines **79 macros** on this SDK **(exp)** —
names such as `__SECURE_SOFTWARE_INSTALL__`, `__WATCHER_API_V2__`, and a family of
`SYMBIAN_*` feature switches. All 79 are therefore visible to `#ifdef` inside a `.mmp`.

`variant.cfg` may also contain a line `ENABLE_ABIV2_MODE`. It does **not** in this SDK, which is why
the platform named `ARMV5` here means the RVCT ABIv1 platform and the ABIv2 RVCT platform is named
`ARMV5_ABIV2` **(read)**. This does not affect GCCE.

### 1.6 Difference between `bld.inf` and `.mmp`

`bld.inf` is preprocessed **twice** **(read, confirmed exp 63 — one pass with no `-D` at all,
then one pass per platform, five in total for this SDK's four platforms)**:

1. **Platform pass** — with the variant header but with **no `-D` macros at all**. This pass reads
   only `PRJ_PLATFORMS`, `PRJ_EXPORTS` and `PRJ_TESTEXPORTS`. So a `#ifdef GCCE` around a
   `PRJ_EXPORTS` line is **always false** in this pass; exports cannot be made platform-conditional.
2. **Per-platform pass** — once for each platform, with that platform's six macros. This pass reads
   `PRJ_MMPFILES`, `PRJ_TESTMMPFILES`, `PRJ_EXTENSIONS`, `PRJ_TESTEXTENSIONS`. Conditionals here do
   work.

`.mmp` is preprocessed once per platform, with that platform's six macros.

### 1.7 Consuming the output

The preprocessor output is read line by line **(read, confirmed exp)**:

* A line matching `# <number> "<file>"` optionally followed by a further number is a **line marker**.
  It sets the current file and the current line number; it is not data. Backslashes in the path come
  through doubled and are collapsed; forward slashes are turned into backslashes; a leading drive
  letter is stripped.
* A line that is empty or only whitespace is **skipped**, but still advances the line counter.
* Every other line is tokenised (§2) and becomes one record: `(file, line number, token…)`.

Exit status is checked: a non-zero preprocessor exit aborts the whole build with an error naming the
command **(read)**. A missing `#include` gives exit status 33 **(exp)**.

---

## 2. Lexical rules

### 2.1 Comments

Both `//`-to-end-of-line and `/* … */` comments are removed **by the preprocessor**, not by the
parser **(exp)**. A `/* … */` spanning *n* lines leaves *n* lines in the output (blank or
whitespace-only), so line numbers stay correct.

There is **no** `.mmp`-specific comment syntax. `#` at the start of a line is a preprocessor
directive, not a comment.

### 2.2 Line continuation

A backslash at end of line is consumed by the preprocessor in the ordinary C way: the two source
lines become one output line, and a blank line is emitted after it to preserve numbering **(exp)**.
So `SOURCE a.cpp \` / `b.cpp` is one `SOURCE` directive with two arguments.

### 2.3 Tokenising

A non-blank line is split into tokens by this rule, scanning left to right **(read)**:

* A token is either a **double-quoted run** — an opening `"`, one or more characters that are not
  `"`, tab, newline, carriage return or form feed, and a closing `"` — in which case the token is the
  content **without** the quotes; or
* a **bare run** of one or more characters that are not space, `"`, tab, newline, carriage return or
  form feed.

Everything else (spaces, tabs) separates tokens. Consequences worth knowing:

* A path containing spaces must be quoted: `SOURCEPATH "..\a dir with spaces"` gives one token
  `..\a dir with spaces` **(exp)**.
* A quote in the **middle** of a word splits the word. `MACRO -D"QUOTED=1"` yields three tokens:
  `MACRO`, `-D`, `QUOTED=1`. There is no way to write an argument that contains a literal quote.
* An empty quoted string `""` does not match the quoted alternative (it requires at least one
  character); it degenerates to nothing and is dropped.
* Tabs are separators, including a leading tab; indentation is free **(exp)**.

### 2.4 Case

* **`.mmp`**: the first token of a line (the directive name) is upper-cased before it is matched, so
  directive names are case-insensitive. **Arguments keep their source case** **(read)**. Exceptions
  where the front end then folds an argument's case itself:
  * bitmap colour-depth tokens are lower-cased;
  * bitmap source file names are lower-cased;
  * `TARGETTYPE`, `EPOCPROCESSPRIORITY` and capability names are matched case-insensitively and
    stored in a canonical spelling;
  * source file names are lower-cased when the compile rule is emitted, and then the first letter is
    upper-cased again for any extension other than `.cpp`/`.c`.
* **`bld.inf`**: in the per-platform pass **every token on every line is upper-cased** before
  matching, including `.mmp` file paths. The platform pass upper-cases only platform names; export
  source and destination paths keep their case **(read)**.

This is the SDK's deepest assumption about the host: it expects a **case-insensitive filesystem**.
A native Linux builder must compare and open paths case-insensitively (or normalise) if it wants to
accept `.mmp` files written for Windows. See §12.

---

## 3. Path semantics

All paths in both file kinds are written in DOS style. Forward slashes are accepted everywhere and
are converted to backslashes before anything else happens **(read)**; `../src` and `..\src` are the
same. A drive letter is never written in a `.mmp` and is stripped if present.

Let *EPOCROOT* be the SDK root (the directory containing `epoc32/`) and let *EPOCPATH* be
`EPOCROOT/epoc32/`. Resolution of a path argument against a base directory — the base being the
directory of the file the line came from, or the current `SOURCEPATH`, depending on the directive —
is **(read)**:

| Form | Resolves to |
|---|---|
| `\epoc32\<rest>` | *EPOCPATH*`/<rest>` — a special case, matched case-insensitively |
| `+\<rest>` (the `+` may appear anywhere, the match is on `+\`) | *EPOCPATH*`/<rest>` |
| `..\<rest>` or `..` at the start | base + the path, then `.`/`..` collapsed textually |
| starts with any character other than `.` or `\` | base + the path, **without** collapsing |
| starts with `\` (and is not `\epoc32\…`) | left as is — i.e. **relative to the root of the drive**, which on Windows is the drive the build runs on |
| `.\<rest>` | base + `<rest>`, collapsed |
| anything else | resolution fails |

`.`/`..` collapsing is purely textual: repeated `\.\` are removed, and `\<name>\..\` is removed
provided `<name>` is not itself `..`. Symlinks and the real filesystem are not consulted.

The bare leading `\` case is the one that has no meaning on Linux. `TARGETPATH \resource\apps`
and `SYSTEMINCLUDE \epoc32\include` look alike but behave differently: the first is a *device* path
that is only ever used as a string (§3.1), the second hits the `\epoc32\` special case and becomes
an SDK path. A native builder should:

* treat `\epoc32\…` and `+\…` as *EPOCPATH*-relative;
* treat any other leading-`\` path used as an **input** path as *EPOCROOT*-relative, and say so in
  the error message if it does not exist;
* treat leading-`\` paths used as **device target** paths as opaque strings (§3.1).

### 3.1 Target paths are device paths, not host paths

`TARGETPATH` (top level, inside `START RESOURCE`, inside `START BITMAP`) is normalised as
**(read)**: convert slashes, drop one leading `\`, ensure exactly one trailing `\`, then **prefix
`z\`**. So `TARGETPATH \resource\apps` becomes the string `z\resource\apps\`. That string is the
location on the emulated device's Z drive. Where the file actually lands on the host is
*EPOCPATH*`/data/` + that string, e.g. `epoc32/data/z/resource/apps/gui.rsc` **(read)**.

For built binaries the host location is *EPOCPATH*`/release/<platform>/<urel|udeb>/` — for GCCE,
`epoc32/release/gcce/urel/` **(read)**. `TARGETPATH` does not move the binary; it only records the
device path for later packaging.

### 3.2 Other fixed host locations (GCCE, ARMV5)

| Thing | Host location |
|---|---|
| Import libraries (`.dso`) | *EPOCPATH*`/release/armv5/lib/` |
| Static libraries (`.lib`), entry-point objects | *EPOCPATH*`/release/armv5/urel/` (and `/udeb/`) |
| Built binary | *EPOCPATH*`/release/gcce/urel/` |
| Generated resource headers (`.rsg`) and bitmap headers (`.mbg`) | *EPOCPATH*`/include/` |
| Compiled resources and bitmaps | *EPOCPATH*`/data/` + the `z\…\` target path |
| Intermediate objects | *EPOCPATH*`/build/<group dir>/<mmp base>/<platform>/<urel\|udeb>/` |

`<group dir>` there is the **whole absolute path** of the directory holding `bld.inf`, drive letter
removed, not just its last component **(exp 63)** — a project in `\work\gui\group` builds into
*EPOCPATH*`/build/work/gui/group/GUI/GCCE/urel/`. The `epoc32/build` part of the path is written in
capitals by the generator and the rest in the case the project used, which matters only on a
case-sensitive host.

Note the asymmetry: the **platform** name selects the binary directory, but the **ABI** name selects
the library directories. GCCE links against exactly the same `armv5` import libraries as an RVCT
build.

---

## 4. `bld.inf`

### 4.1 Shape

A `bld.inf` is a sequence of **sections**. A line whose first token matches `PRJ_` followed by word
characters starts a new section; nothing else may appear on that line. Everything until the next
section header belongs to the current section. Lines before the first section header belong to no
section and are silently dropped **(read)**.

The section header is matched case-insensitively. The recognised names are exactly:

| Section | Meaning |
|---|---|
| `PRJ_PLATFORMS` | which platforms this component supports |
| `PRJ_EXPORTS` | files to copy into the SDK tree before building |
| `PRJ_TESTEXPORTS` | same, for test material |
| `PRJ_MMPFILES` | the components to build |
| `PRJ_TESTMMPFILES` | the test components to build |
| `PRJ_EXTENSIONS` | makefile-template invocations |
| `PRJ_TESTEXTENSIONS` | same, for tests |

Any other `PRJ_…` name is a **fatal error** naming the file and line **(read)**.

### 4.2 `PRJ_PLATFORMS`

One or more platform names, on one or more lines, in any case. Processing **(read)**:

* `WINC` is accepted and ignored (an obsolete target).
* `DEFAULT` expands to the default platform list. On this SDK, with no RVCT compiler installed
  (detected by the absence of an `ARMROOT` environment variable and of an RVCT version), that list
  is `WINSCW GCCXML EDG`. If RVCT is present, `ARMV5` is appended.
* `BASEDEFAULT` expands to `ARM4 ARM4T WINSCW GCCXML EDG`, plus `ARMV4 ARMV5` if `ARMROOT` is set,
  plus `WINS X86` if `MSDevDir` is set.
* A name prefixed with `-` removes that platform, but only if `DEFAULT` or `BASEDEFAULT` appeared
  **earlier**; otherwise it is a fatal error.
* A name beginning with `TOOLS` additionally pulls in `CWTOOLS`.
* A name beginning with `VC` is a fatal error.
* An unsupported name is fatal (or, with the "keep going" option, a warning).
* If the section is absent or empty, the default list applies.

**GCCE is never in the default list.** It is an "optional" platform, discovered from the presence of
`epoc32/tools/compilation_config/gcce.mk`. Optional platforms are appended to the project's platform
list unconditionally, so `abld build gcce urel` works even when `PRJ_PLATFORMS` does not mention
GCCE — but the component is then not part of a default "build everything" **(read)**.

For a native Linux builder, the honest reading is: **`PRJ_PLATFORMS` does not gate a GCCE build.**
Parse it, use it for diagnostics, and do not refuse a GCCE build because the list omits GCCE.

### 4.3 `PRJ_EXPORTS` and `PRJ_TESTEXPORTS`

Each line is `<source> [<destination>] [overwrite]`, or `:zip <archive> [<destination>]`
**(read)**.

* `<source>` is resolved relative to the `bld.inf`'s directory.
* If `<destination>` is omitted, or omits a filename, the source's filename is appended.
* Default destination directory:
  * `PRJ_EXPORTS`: *EPOCPATH*`/include/`
  * `PRJ_TESTEXPORTS`: the `bld.inf`'s own directory
  * `:zip` archives: *EPOCROOT* (the archive is unpacked there)
* A destination beginning with `|` (optionally followed by a slash) forces "relative to the
  `bld.inf` directory" and the `|` is stripped.
* A destination of the form `<letter>:\<rest>` is rewritten to *EPOCPATH*`/data/<letter>\<rest>` —
  this is how a file is exported onto the emulated device's drives.
* The only third token accepted is `overwrite` (case-insensitive), and only for `:zip`; anything
  else is a fatal error.
* Two exports with the same destination are a fatal error. A missing source is fatal (or a warning
  in keep-going mode).
* The only archive type is `zip`; any other `:word` prefix is fatal.

A native Linux builder **can** honour exports: they are file copies plus one unzip. It must decide
whether it is willing to write into the user's SDK tree; if not, it must say so explicitly rather
than skipping silently.

### 4.4 `PRJ_MMPFILES` and `PRJ_TESTMMPFILES`

Each line is one of **(read)**:

| Form | Meaning |
|---|---|
| `<path>[.mmp] [qualifiers…]` | build this project |
| `MAKEFILE <path> [qualifiers…]` | hand off to an external makefile, built with `nmake` |
| `NMAKEFILE <path> [qualifiers…]` | same as `MAKEFILE` |
| `GNUMAKEFILE <path> [qualifiers…]` | hand off to an external makefile, built with GNU `make` |

For the plain form the `.mmp` extension is optional and is supplied if missing. For the makefile
forms, the extension is taken from the path as written (there is no default) and the path is
resolved relative to the `bld.inf`. The external makefile is then run **in its own directory** —
`nmake` after a `cd` for `MAKEFILE`/`NMAKEFILE`, GNU `make -C` for `GNUMAKEFILE` — once per build
stage, with the stage name as the goal. The stages are `MAKMAKE`, `BLD`, `SAVESPACE`, `LIB`,
`FREEZE`, `CLEAN`, `RESOURCE`, `FINAL`, `TIDY`, `CLEANLIB` and `ROMFILE`, and the makefile is
handed the variables `PLATFORM`, `CFG` (the build variant), `TO_ROOT`, `TO_BLDINF`, `EPOCBLD`, and
`BUILD_AS_ARM=1` when that qualifier was given **(read)**.

The file must exist, or the build fails (a warning in keep-going mode). Two lines naming the same
directory+basename are a fatal error even if the extensions differ.

Qualifiers, matched after upper-casing **(read)**:

| Qualifier | `PRJ_MMPFILES` | `PRJ_TESTMMPFILES` | Effect |
|---|---|---|---|
| `BLD` | — | — | **not a qualifier**; see below |
| `TIDY` | yes | yes | the target is removed from the release tree at the "tidy" stage |
| `IGNORE` | yes | yes | the whole line is discarded |
| `BUILD_AS_ARM` | yes | yes | the project is built with the ARM (rather than Thumb) instruction set |
| `MANUAL` | — | yes | excluded from automated test runs |
| `SUPPORT` | — | yes | marks supporting (non-test) material |

Any other qualifier is a fatal error. `BLD` is **not** recognised by this SDK's `bld.inf` parser; if
it appears it is rejected as an unknown qualifier. (It is a keyword from older `.mmp`-era syntax and
from other build systems; do not accept it.)

`BUILD_AS_ARM` on **GCCE** has no observable effect: the instruction-set settings it selects between
are both empty in the GCCE configuration (§8.4) **(read)**.

`PRJ_TESTMMPFILES` is skipped entirely when the "no test" option is in force **(read)**. A native
builder should default to building only `PRJ_MMPFILES`.

### 4.5 `PRJ_EXTENSIONS` and `PRJ_TESTEXTENSIONS`

The section contains one or more blocks:

```
START EXTENSION <name>
  TOOL         <…>
  OPTION       <key> <value…>
  TARGET       <…>
  SOURCES      <…>
  DEPENDENCIES <…>
END
```

Rules **(read)**:

* The first token of the opening line must be `START` (any case) and the second must be `EXTENSION`;
  a name must follow. Nested `START` is a fatal error, as is `END` without `START`.
* Inside the block only `TOOL`, `OPTION`, `TARGET`, `SOURCES` and `DEPENDENCIES` are allowed; any
  other keyword is a fatal error whose message suggests a missing `END`.
* `OPTION <key> <value…>` sets a makefile variable named `<key>`; the other keywords set a variable
  named after themselves. Values are restored to their original source case (the rest of the line
  having been upper-cased for matching).
* `<name>` selects a template pair in *EPOCPATH*`/tools/makefile_templates/`: `<name>.meta`
  (lower-cased) describing defaults and whether the template is GNU make or nmake, and `<name>.mk`
  which is included by the generated wrapper.
* The `.meta` file accepts only `MAKEFILE GNUMAKE|NMAKE`, `OPTION <key> <value…>`, `TECHSTREAM` and
  `PLATFORM` (the last two are ignored); anything else is a fatal error, as is a `.meta` that never
  says which makefile flavour it is.

**This SDK has no `epoc32/tools/makefile_templates/` directory at all** **(exp)**. Every
`START EXTENSION` block therefore fails here with "cannot open META file", whatever it names.

A native Linux builder must **reject** `PRJ_EXTENSIONS` / `PRJ_TESTEXTENSIONS` with a message
saying the extension-template mechanism is a makefile hand-off that this toolchain does not
implement, and naming the extension.

### 4.6 What a Linux native builder should do with each section

| Section | Verdict |
|---|---|
| `PRJ_PLATFORMS` | parse, use for diagnostics only; never let it veto a GCCE build |
| `PRJ_EXPORTS` | honour (file copies + unzip), or refuse explicitly if unwilling to write into the SDK |
| `PRJ_TESTEXPORTS` | honour, or skip with a message, when tests are requested |
| `PRJ_MMPFILES` (plain form) | honour |
| `PRJ_MMPFILES` (`MAKEFILE`/`NMAKEFILE`/`GNUMAKEFILE`) | **reject** — it is an escape hatch into `nmake`/`make` with SDK-specific targets |
| `PRJ_TESTMMPFILES` | parse; do not build by default |
| `PRJ_EXTENSIONS`, `PRJ_TESTEXTENSIONS` | **reject** |

---

## 5. `.mmp`

### 5.1 Shape

A flat sequence of directives, one per line, plus three kinds of `START … END` block. There is no
nesting beyond one block level. Unlike `bld.inf`, **an unrecognised directive is a warning, not an
error** **(read)**: the SDK prints it and carries on. This is why so many `.mmp` files in the wild
contain directives this SDK does not implement (§5.6).

A native builder should **not** copy that behaviour. Fail on unknown directives; a silently ignored
`CAPABILITY` or `EPOCSTACKSIZE` is worse than a stopped build.

### 5.2 `START` blocks

When a line's first token is `START`, the second token decides the block, tested in this order
**(read)**:

1. If the second token equals one of the six platform macro names of the current platform
   (§1.3 — for GCCE: `GCCE`, `EPOC32`, `MARM`, `EABI`, `GENERIC_MARM`, `MARM_ARMV5`), the block's
   lines are **captured** and handed to the platform back end. On GCCE nothing consumes them, so
   `START GCCE … END` is parsed and then **ignored**. Only the RVCT back end reads such a block
   (for RVCT-specific `ARMCC` settings).
2. `BITMAP` — §7.
3. `RESOURCE` — §6.
4. `STRINGTABLE` — a generated-constants mechanism; see §5.5.
5. Anything else — the block is **skipped entirely** until its `END`, with no message. So
   `START WINSCW … END` in a GCCE build vanishes.

An unterminated block is a fatal error at the end of the file.

### 5.3 Directives — honoured

Argument counts below are what the SDK enforces. "extra arguments" means the SDK warns and ignores
the surplus unless stated otherwise.

| Directive | Args | Default if absent | Effect |
|---|---|---|---|
| `TARGET` | 1 | **fatal error** unless `TARGETTYPE NONE` | Output file name, e.g. `gui.exe`. Slashes normalised. `{` or `}` in the name is fatal. Redefinition warns and keeps the first. |
| `TARGETTYPE` | 1 | **fatal error** | §9. Unknown type is fatal. Redefinition warns and keeps the first. |
| `TARGETPATH` | 1 | the type's default path (§9), usually empty | Device path for the target; `z\`-prefixed as in §3.1. Also becomes the default target path for resources and bitmaps when the type does not override it. |
| `UID` | 1–2 | both `0x00000000` | UID2 then UID3. A third is a warning and is dropped. §10. |
| `SECUREID` | 1 | UID3 | §10. |
| `VENDORID` | 1 | not passed to the post-linker at all | §10. |
| `CAPABILITY` | ≥1 | `none` | §10.3. Redefinition warns and keeps the first. |
| `MACRO` | ≥0 | — | Each argument is added verbatim to the compiler's `-D` list and to the resource compiler's `-D` list. Duplicates warn. **Zero arguments is silently accepted.** The argument is passed through untouched — `MACRO FOO=1` works. |
| `SOURCEPATH` | 1 | the `.mmp`'s own directory | Base directory for subsequent `SOURCE`, `DOCUMENT`, `RESOURCE`, `SYSTEMRESOURCE` and `START RESOURCE` arguments. Resolved per §3, trailing `\` added. A non-existent directory is a warning, not an error. |
| `SOURCE` | ≥1 | **at least one required** (except `TARGETTYPE IMPLIB`/`NONE`, where any is fatal) | Each argument may carry a directory part, resolved against the current `SOURCEPATH`. A leading `\` is stripped first. Order is preserved. |
| `DOCUMENT` | ≥1 | — | Files recorded for IDE project generation only; no build effect. Missing files warn. |
| `USERINCLUDE` | ≥1 | none | Added to the compiler include list, **before** the system includes. Resolved per §3. Non-existent directory warns. Duplicates warn and are dropped. |
| `SYSTEMINCLUDE` | ≥1 | none | Added after the user includes. Same resolution and checks. **Nothing is added implicitly** — an `.mmp` that does not say `SYSTEMINCLUDE \epoc32\include` does not get it (§8.3). |
| `LIBRARY` | ≥1 | none | Import libraries. Each name's extension is replaced with `.dso`; the file is looked for in *EPOCPATH*`/release/armv5/lib/`. Names may carry a `{version}` decoration (§11). Duplicates warn. Entries also join the UDEB list. |
| `DEBUGLIBRARY` | ≥1 | none | Extra import libraries linked into **UDEB only**. Same naming rules. |
| `STATICLIBRARY` | ≥1 | none | Static `.lib` files from *EPOCPATH*`/release/armv5/<urel\|udeb>/`. Duplicates warn. |
| `FIRSTLIB` | 1 | derived from the target type (§9) | Overrides the entry-point static library. Redefinition warns. |
| `EPOCSTACKSIZE` | 1 | not passed | Number (§11.1). Reaches the image as the stack size. |
| `EPOCHEAPSIZE` | 2 | not passed | Min then max (§11.1). Both required; one argument is fatal. |
| `EPOCALLOWDLLDATA` | 0 | off | Allows writable static data in a DLL; reaches the post-linker. |
| `EPOCPROCESSPRIORITY` | 1 | not passed | One of `LOW`, `BACKGROUND`, `FOREGROUND`, `HIGH`, `WINDOWSERVER`, `FILESERVER`, `REALTIMESERVER`, `SUPERVISOR`, case-insensitive. Anything else is fatal. |
| `EPOCFIXEDPROCESS` | 0 | off | Fixed-address process. |
| `EPOCDATALINKADDRESS` | 1 | not passed | Number (§11.1). |
| `EPOCCALLDLLENTRYPOINTS` | 0 | off | Parsed; the code that used it is disabled in this SDK, so it has **no effect**. |
| `VERSION` | 1, optionally `explicit` | major `10`, minor `0` for EABI platforms | `<major>.<minor>`, both decimal, both < 32768. §11.2. |
| `LINKAS` | 1 | the `TARGET` name | The name recorded inside the image for other binaries to link against. §11.2. `{`/`}` is fatal. |
| `DEFFILE` | 1 | derived (§5.4) | Path to the frozen export list. Redefinition warns. |
| `NOSTRICTDEF` | 0 | off | Suppresses the `u` suffix on the derived `.def` basename. Repetition warns. |
| `EXPORTUNFROZEN` | 0 | off | Build even though exports are not frozen; no import library is produced. |
| `EXPORTLIBRARY` | 1 | the `TARGET` basename | Basename of the produced import library. Mutually exclusive with `NOEXPORTLIBRARY` (fatal). |
| `NOEXPORTLIBRARY` | 0 | off | Produce no import library. Mutually exclusive with `EXPORTLIBRARY` (fatal). |
| `LANG` | ≥1 | `SC` | Language suffixes for resources that do not carry their own `LANG`. §6.3. Duplicates warn. |
| `ARMFPU` | 1 | `SOFTVFP` behaviour | `SOFTVFP` or `VFPV2`, case-insensitive; anything else warns and is dropped. On GCCE, `VFPV2` selects a compiler option that is **empty** in this SDK's GCCE configuration, so the only real effect is on the post-linker's FPU field. |
| `ALWAYS_BUILD_AS_ARM` | 0 | off | Same meaning as the `bld.inf` qualifier; no effect on GCCE (§4.4). |
| `PAGED` / `UNPAGED` | 0 | neither | Demand-paging hint; reaches the post-linker and the ROM description. Specifying both, or either twice, is fatal. |
| `COMPRESSTARGET` | 0 | this **is** the default | Compress the image, method chosen by the post-linker. |
| `NOCOMPRESSTARGET` | 0 | — | Do not compress. |
| `INFLATECOMPRESSTARGET` | 0 | — | Compress with the inflate method. Combined with `PAGED` it warns and is downgraded to byte-pair. |
| `BYTEPAIRCOMPRESSTARGET` | 0 | — | Compress with the byte-pair method. Implied by `PAGED`. |
| `OPTION <compiler> <text…>` | ≥2 | none | Extra compiler flags. §8.5. |
| `START RESOURCE … END` | — | — | §6. |
| `START BITMAP … END` | — | — | §7. |

### 5.4 The derived `.def` file name

Relevant only to target types that export (DLLs and `EXEXP`), so out of scope for a plain
application, but recorded for completeness **(read)**. When `DEFFILE` gives no directory, the
directory is the `.mmp`'s parent directory plus a sibling named after the platform's `.def` flavour
— for every EABI platform, including GCCE, that is `eabi`. A `~` component in a given `DEFFILE`
path is replaced by that same name. The basename defaults to the `LINKAS` basename; the extension
defaults to `.def`. Unless `NOSTRICTDEF` is set (or an explicit `VERSION` is in force, in which case
the `{version}` decoration is appended instead) a `u` is appended to the basename. A missing `.def`
is a warning: "project not frozen".

### 5.5 Directives accepted but of no use here

These parse cleanly and are recorded in the model, but are irrelevant to a GCCE application build,
or are actively dead on this platform **(read)**:

| Directive | Why |
|---|---|
| `AIF <target> <dir> <file> <depths> [<bitmaps>…]` | The pre-9.x application-information-file mechanism. Superseded by the registration resource. |
| `RESOURCE <file…>` | Old flat form of `START RESOURCE`; each file becomes a resource with no header, default target path. |
| `SYSTEMRESOURCE <file…>` | As `RESOURCE` but forces the target path to `z\system\data\`. Produces a migration note on this (secure) platform. |
| `RAMTARGET` / `ROMTARGET` | Extra release copies; `+` as the first argument means "in addition to the default". |
| `ASSPABI`, `ASSPEXPORTS`, `ASSPLIBRARY` | ASSP-specific linking. **Fatal on a generic platform**, and GCCE is generic — any of these three aborts the build. |
| `STRICTDEPEND` | Per-variant dependency generation. |
| `SRCDBG` | Marks the project as wanting source debug; on GCCE the debug flags are chosen by the build variant, so it changes nothing. |
| `WCHARENTRYPOINT` | Only meaningful for `TARGETTYPE STDEXE`; warns otherwise. |
| `FEATUREVARIANT` | Feature-variant builds; out of scope. Twice is fatal. |
| `START STRINGTABLE <file> … END` with `EXPORTPATH <dir>` / `HEADERONLY` | Generates a `.h`/`.cpp` pair from a string-table description. Note the block has no `END` handling bug-for-bug equivalent: the block is closed on `END` like the others. |
| `OPTION_REPLACE <compiler> <text…>` | Parsed, but **only the RVCT back end reads it**. On GCCE it is silently discarded. |
| `LINKEROPTION <linker> <text…>` | Parsed, and the SDK prints an unlabelled debug line to stdout when it is used, but **nothing on GCCE reads it**. It has no effect on the link. It also has a bug: the first text token is dropped if an `OPTION` with the same key appeared earlier. |
| `COMPRESSTARGET` family | See §5.3; correct but only affects the post-linker. |

### 5.6 Directives this SDK does **not** know

Searched for and absent from this SDK's `.mmp` parser **(read)**:

`DEBUGGABLE`, `DEBUGGABLE_UDEBONLY`, `SMPSAFE`, `STDCPP`, `NOSTDCPP`, `ARMFPU VFPV3` (only
`SOFTVFP`/`VFPV2` exist), `SOURCEPATH_ABS`, `LINKEROPTION` targets other than a bare key,
`EPOCPROCESSPRIORITY` values outside the eight listed.

Each of these produces the generic "unrecognised keyword" **warning** and is otherwise ignored. They
are directives of later Symbian releases (9.4 / Symbian^3) that people copy into FP2 projects.

For a native builder, the recommended split is:

| Group | Directives | Behaviour |
|---|---|---|
| Honoured | everything in §5.3 | implement |
| Accepted and ignored, with a one-line note | `DOCUMENT`, `SRCDBG`, `STRICTDEPEND`, `EPOCCALLDLLENTRYPOINTS`, `ALWAYS_BUILD_AS_ARM`, `OPTION_REPLACE`, `LINKEROPTION`, `DEBUGGABLE`, `DEBUGGABLE_UDEBONLY`, `SMPSAFE` | parse, record, warn once that they do nothing on this path |
| Must reject | `ASSPABI`, `ASSPEXPORTS`, `ASSPLIBRARY` (fatal in the SDK too), `AIF`, `START STRINGTABLE`, `FEATUREVARIANT`, `RAMTARGET`, `ROMTARGET`, `SYSTEMRESOURCE`, any unknown name | stop with a message naming the directive and the line |

---

## 6. `START RESOURCE … END`

```
START RESOURCE <source.rss>
  TARGET      <name>
  TARGETPATH  <device path>
  HEADER
  HEADERONLY
  LANG        <code…>
  UID         <uid2> [<uid3>]
END
```

### 6.1 The opening line

Exactly one argument, the `.rss` source, resolved against the current `SOURCEPATH`. Any other count
warns; with zero or more than one the source is left unset and the block is effectively broken
**(read)**.

### 6.2 Inner directives

| Inner directive | Args | Effect |
|---|---|---|
| `TARGET` | 1 | The **basename** of the compiled resource. Only the basename is kept — a directory or extension written here is discarded. A second `TARGET` warns and is ignored. An empty name is fatal. |
| `TARGETPATH` | 1 | Device path, normalised and `z\`-prefixed exactly as in §3.1. A second one warns. |
| `HEADER` | 0 | Generate the `.rsg` header **and** the compiled resource. Mutually exclusive with `HEADERONLY` (warns, first wins). |
| `HEADERONLY` | 0 | Generate **only** the `.rsg`; no `.rsc` is produced and no target path is used. |
| `LANG` | ≥1 | Language codes for this block only, overriding the file-level `LANG`. Duplicates within the block warn. |
| `UID` | 1–2 | Second and third UID of the resource file. More than two warns and the line is dropped. |
| anything else | — | warning, ignored |

`END` closes the block and records the current `SOURCEPATH` with it.

### 6.3 Output names

Let `B` be the basename: the `TARGET` value if given, otherwise the basename of the source `.rss`
**(read)**.

Let `P` be the target path: the block's `TARGETPATH` if given, otherwise the target type's resource
path (§9) if it has one, otherwise the file-level `TARGETPATH`, otherwise **empty**. Note that in
the first three cases `P` already carries its leading `z\` (§3.1); in the fourth it is the empty
string and the resource is not placed on the device's Z drive at all.

Let `L` be the list of languages: the block's `LANG` if given, otherwise the file-level `LANG`,
otherwise the single code `SC`.

Then **one compiled resource is produced per language**, and its device path is:

> `P` + `B` + **lower-case of** `.R` + *language code*

So the extension is formed by lower-casing the two characters `.R` followed by the code as written.
This is why the ordinary case comes out `.rsc`: the default code is `SC`, and `.RSC` lower-cased is
`.rsc`. `LANG 01` gives `.r01`; `LANG 02 03` gives `.r02` and `.r03` **(read)**.

Two resources resolving to the same device path are a fatal error.

The generated header, if `HEADER` or `HEADERONLY` was given, is always
*EPOCPATH*`/include/` + `B` + `.rsg` — **one** file, independent of language, overwritten by each
language's build **(read, confirmed exp 63)**.

The compiled resource lands on the host at *EPOCPATH*`/data/` + the device path.

### 6.4 The resource compiler call

For each (resource, language) pair the SDK runs its resource-compilation wrapper with, in order
**(read, confirmed exp 63)**:

* three suppressed message numbers: `-m045,046,047`;
* `-I <directory of the .rss>`;
* `-I <each USERINCLUDE>`;
* `-I-` (the separator that ends the "quoted include" list);
* `-I <each SYSTEMINCLUDE>`, followed by the variant header's directory;
* `-D<name>` for **each `MACRO` from the `.mmp`** — note: the *user's* macros only, not the platform
  macros;
* `-DLANGUAGE_<code>`;
* `-u` (Unicode);
* the source `.rss`;
* `-uid2 <uid2>` and `-uid3 <uid3>` if the block's `UID` gave them (only `-uid2` if one was given);
* `-o<output .rsc>`, omitted for `HEADERONLY`;
* `-h<temporary .rsg>`, present only when `HEADER` or `HEADERONLY` was given;
* `-t<temporary directory>`;
* `-l"<device directory>:<working directory>"`, omitted for `HEADERONLY`;
* `-preinclude"<variant header>"`.

The `.rsg` is written to a temporary directory first and copied to *EPOCPATH*`/include/` afterwards.
The temporary directory is the project's own build directory (§3.2), and the copy is a second
recipe line, not something the resource wrapper does **(exp 63)**.

The captured rule, with the paths shortened, is one line of the form

```
<resource wrapper> -m045,046,047 -I "<rss dir>" -I "<USERINCLUDE>"… -I- -I "<SYSTEMINCLUDE>"…
    -I "<variant dir>" -DLANGUAGE_SC -u "<source.rss>" -o<output .rsc>
    -h"<build dir>\<base>.rsg" -t"<build dir>"
    -l"<device directory>:<directory holding the .mmp>" -preinclude"<variant header>"
```

Note that the arguments the SDK quotes and the ones it does not are not consistent: `-I` takes a
quoted argument, `-o` takes `$@` unquoted, and `-h`, `-t`, `-l` and `-preinclude` are written with
no space and a quoted value **(exp 63)**.

`symdev` already has a native resource compiler; this section exists so the front end feeds it the
right macro set — in particular **`-DLANGUAGE_SC`** and the user's `MACRO` values, and **not** the
platform macros.

---

## 7. `START BITMAP … END`

```
START BITMAP <target.mbm>
  TARGETPATH  <device path>
  HEADER
  SOURCEPATH  <dir>
  SOURCE      <depths> <file.bmp> [<file.bmp> …]
END
```

### 7.1 The opening line

Exactly one argument, the `.mbm` file name. Zero arguments warns and substitutes a placeholder name;
extra arguments warn **(read)**. The name is used verbatim (slashes normalised) as the target file
name.

### 7.2 Inner directives

| Inner directive | Args | Effect |
|---|---|---|
| `SOURCEPATH` | 1 | Base directory for subsequent `SOURCE` lines **inside this block only**. Resolved against the `.mmp`'s directory, trailing `\` added. Reset to the `.mmp`'s directory at `END`. Extra arguments warn. |
| `SOURCE` | ≥2 | First argument is a comma-separated list of colour depths; the rest are `.bmp` files. |
| `HEADER` | 0 | Also generate the `.mbg` enumeration header. A second `HEADER` warns. Extra arguments warn. |
| `TARGETPATH` | 1 | Device path, normalised and `z\`-prefixed as in §3.1. A second one warns. |
| anything else | — | warning, ignored |

### 7.3 Colour depths

Each depth token is lower-cased and must match: an optional `c`, then one or two decimal digits.
Anything else is **fatal** **(read)**. Examples: `1`, `2`, `4`, `8`, `c8`, `c12`, `c16`, `c24`.
A leading `c` means colour; without it, greyscale.

The depth list is **cycled** over the file list: the list is repeated enough times to cover every
file, and each file takes the next depth. So `SOURCE c8,1 a.bmp b.bmp c.bmp d.bmp` gives
`a`→`c8`, `b`→`1`, `c`→`c8`, `d`→`1`. With a single depth, every file gets it.

Multiple `SOURCE` lines accumulate, each with its own depth cycle, and the **order of files across
all `SOURCE` lines is the order of bitmaps in the `.mbm`** — which is the order the generated
enumeration uses, so it is part of the interface.

### 7.4 File names

Every source file name is **lower-cased** before use **(read)** — the SDK comments that this is
because the bitmap compiler derives case-sensitive enumeration names from them. A missing source
file is a warning, not an error (the build then fails later in the bitmap compiler).

### 7.5 Output names

* Target device path: (block `TARGETPATH`, else the target type's resource path, else the
  file-level `TARGETPATH`, else empty) + the `START BITMAP` name. As in §6.3 the path already
  carries its leading `z\` in the first three cases.
* Host location: *EPOCPATH*`/data/` + that device path.
* Header, when `HEADER` was given: *EPOCPATH*`/include/` + **basename of the `START BITMAP` name** +
  `.mbg`. It is written to the build directory first and copied.
* Two bitmaps resolving to the same device path are a fatal error.

### 7.6 The bitmap compiler call

The SDK's wrapper turns the block into exactly **(read)**:

```
bmconv /q [/h"<build dir>\<base>.mbg"] <output .mbm> /<depth><source1> /<depth><source2> …
```

with one `/<depth><path>` argument per source, in declaration order, `<depth>` being the lower-cased
depth token (so `/c8/home/…/a.bmp`-style concatenation with no separator). `/q` is always passed.
`/h` is present only when `HEADER` was given. The wrapper does **not** quote the output path, so a
target path containing a space breaks — a real limitation of the SDK, not a rule to reproduce.

That `bmconv` line is still a **reading**: experiment 63 captured the makefile but did not run it,
so it saw the *wrapper's* arguments, not the ones the wrapper passes on. Those were **(exp 63)**:

```
<bitmap wrapper> -h"<build dir>\<base>.mbg" -o"<output .mbm>"
    -l"<device directory>:<directory holding the .mmp>"
    -b"\ /<depth><source1> /<depth><source2>…"
    -l"<device directory>:<directory holding the .mmp>"
```

with one `-l` before the `-b` and an identical one after it, the sources concatenated into a single
`-b` value, and each `/<depth><path>` written with no separator — which is where §7.6's `bmconv`
argument shape comes from. As with the resource wrapper, the `.mbg` is written to the build
directory and copied to *EPOCPATH*`/include/` by a separate recipe line.

`symdev` has a native bitmap compiler; the front end must give it: the ordered list of
`(source path, depth)` pairs, the output `.mbm` path, and optionally the `.mbg` path.

---

## 8. Macros, options and the compiler command line

### 8.1 Two disjoint macro namespaces

This is the single most confusing part of the format, so it is worth stating plainly:

| Namespace | Spelling | Where visible |
|---|---|---|
| Project-file macros | bare, e.g. `GCCE`, `MARM_ARMV5`, `GENERIC_MARM` | only inside `#if`/`#ifdef` in `bld.inf` and `.mmp` |
| Compilation macros | double-underscored, e.g. `__GCCE__`, `__MARM_ARMV5__` | only on the compiler command line, in C/C++ source |

They overlap but are **not** the same list, and neither is visible in the other place.

### 8.2 The compilation macro list, in order

For **GCCE / ARMV5 / UREL / `TARGETTYPE EXE`**, the `-D` arguments are emitted in exactly this order
**(read, confirmed exp 63 in full, item 6 included)**:

1. `-DNDEBUG` and `-D_UNICODE` — the UREL build-variant macros. (UDEB would be `-D_DEBUG -D_UNICODE`.)
2. `-D__GCCE__` — from the GCCE configuration file's compiler-identification setting.
3. The platform macro block, in order:
   `-D__SYMBIAN32__`, `-D__S60_32__`, `-D__S60_3X__`, `-D__SERIES60_3X__`,
   `-D__GCCE__`, `-D__EPOC32__`, `-D__MARM__`, `-D__EABI__`.
4. `-D__MARM_ARMV5__` — added once the ABI is known.
5. `-D__EXE__` — the basic target type, for basic types `EXE` and `DLL` only.
6. One `-D<text>` per **`MACRO`** line argument, verbatim, in source order.
7. `-D__SUPPORT_CPP_EXCEPTIONS__`.
8. `-D__MARM_ARMV5__` again — from the configuration file's platform-identification setting.
9. `-D__PRODUCT_INCLUDE__="<absolute path of the variant header>"`.

Two things surprise people here: `__GCCE__` and `__MARM_ARMV5__` really are passed **twice**, and
`__GENERIC_MARM__` is **never** defined for the compiler even though `GENERIC_MARM` is defined for
the `.mmp`. Both were visible in the captured makefile **(exp 63)**, as was item 6: a `.mmp`
carrying `MACRO MY_FIRST_MACRO` and `MACRO MY_SECOND_MACRO=7` produced
`-DMY_FIRST_MACRO -DMY_SECOND_MACRO=7` between `-D__EXE__` and `-D__SUPPORT_CPP_EXCEPTIONS__`,
in source order and with the `=value` form passed through untouched.

For basic type `DLL` item 5 is `-D__DLL__` **(exp 63)**.

### 8.3 The include list

The compiler's include arguments are, in order **(read, confirmed exp 63 for items 1–4 and 6)**:

1. `-I <directory of the source file being compiled>`
2. `-I <each USERINCLUDE>`, in `.mmp` order
3. `-I <each SYSTEMINCLUDE>`, in `.mmp` order
4. `-I <directory of the variant header>` — appended automatically, i.e.
   *EPOCPATH*`/include/variant`
5. `-I <SDK include directory for standard C APIs>` — *EPOCPATH*`/include/stdapis`, only for target
   types `STDEXE`, `STDDLL`, `STDLIB`
6. `-I <toolchain include directory>` — for GCCE this is the directory reported by asking the
   compiler for its `libgcc` path and appending `include`

`-nostdinc` is always passed, so this list is the whole search path. **The SDK include directory is
not added implicitly** — an `.mmp` that omits `SYSTEMINCLUDE \epoc32\include` genuinely cannot find
`e32base.h`. This is the sharpest confirmation the capture gives **(exp 63)**: `examples/gui` has
no `SYSTEMINCLUDE`, its generated compile line has no `-I` for the SDK include directory, and the
generator warns by name that it cannot find `aknapp.h`, `eikenv.h`, `eikstart.h` and the rest.
`symdev` builds that same project today, so `symdev` supplies an include path the SDK would not —
worth keeping in mind before treating a `symdev` build as evidence about the SDK.

Item 6, the tool-chain include directory, is obtained by asking `arm-none-symbianelf-g++` where its
`libgcc` is and appending `include` to the directory part. With no such compiler on `PATH` the
generator silently emits `-I "\include"` **(exp 63)**; it neither warns nor fails.

The whole include list, plus the source directory and the source file, is wrapped in a make
function that turns backslashes into forward slashes whenever the configuration says the compiler
wants POSIX separators for absolute paths, which the GCCE configuration does **(exp 63)**. The
`-o` argument and the source path go through the same wrapping.

`USERINCLUDE` and `SYSTEMINCLUDE` end up as the same kind of `-I`; the distinction survives only in
the resource-compiler call (§6.4), where a `-I-` separates them.

### 8.4 The flag set, GCCE / ARMV5 / UREL

These are the values this SDK's GCCE configuration file
(`epoc32/tools/compilation_config/gcce.mk`) supplies. That file is plain configuration, not program
text, and an implementer may read it directly; the effective flags are reproduced here so they do
not have to.

Compiler driver: `arm-none-symbianelf-g++` — for **every** source extension, including assembler.
There is no separate C driver and no separate assembler driver in the compile rules. Confirmed for
`.cpp` and `.c` **(exp 63)**: a project with two `.c` sources got the same driver and the same flag
set as a C++ one, differing only in the language option of §8.6.

Flags, in the order the SDK emits them, for a UREL compile. The whole table was reproduced from a
generated makefile and its configuration file in experiment 63 **(exp 63)**; the resulting UREL
line, with the empty positions collapsed, is

```
arm-none-symbianelf-g++ -O2 -fno-unit-at-a-time -fexceptions
  -Wall -Wno-ctor-dtor-privacy -Wno-unknown-pragmas
  -march=armv5t -mapcs -pipe -nostdinc -c -msoft-float
  -DNDEBUG -D_UNICODE -D__GCCE__ -D__SYMBIAN32__ -D__S60_32__ -D__S60_3X__ -D__SERIES60_3X__
  -D__GCCE__ -D__EPOC32__ -D__MARM__ -D__EABI__ -D__MARM_ARMV5__ -D__EXE__
  -D__SUPPORT_CPP_EXCEPTIONS__ -D__MARM_ARMV5__ -D__PRODUCT_INCLUDE__=\"<variant header>\"
  -x c++ -include <EPOCROOT>EPOC32/INCLUDE/GCCE/GCCE.h
  -I <source dir> -I <include list> -o <object> <source>
```

| Position | Flags | Origin |
|---|---|---|
| 1 | *(empty)* | UREL-specific compiler flags — none for GCCE |
| 2 | `-O2 -fno-unit-at-a-time` | release optimisation level |
| 3 | *(empty)* | runtime symbol visibility |
| 4 | `-fexceptions` | exceptions on (kernel-side targets would get `-fno-exceptions`) |
| 5 | `-Wall -Wno-ctor-dtor-privacy -Wno-unknown-pragmas` | warning control |
| 6 | `-march=armv5t` | target architecture |
| 7 | `-mapcs` | procedure call standard |
| 8 | `-pipe` | avoid intermediate files |
| 9 | `-nostdinc` | header search control |
| 10 | `-c` | compile only |
| 11 | *(empty)* | extra compiler options hook |
| 12 | the text of `OPTION GCCE` from the `.mmp`, if any | §8.5 |
| 13 | *(empty)* | instruction set (Thumb or ARM — **both empty on GCCE**) |
| 14 | `-msoft-float` | floating point (or the VFPv2 option, which is **empty** on GCCE) |
| 15 | *(empty)* | Thumb defines |
| 16 | *(empty)* | interworking defines |
| 17 | the `-D` list of §8.2 | |
| 18 | the language option and preinclude, §8.6 | |
| 19 | the include list of §8.3 | |
| 20 | `-o <object file>` | |
| 21 | the source file | |

For UDEB the only differences are: position 1 becomes `-g`, position 2 becomes empty (the debug
optimisation level is unset), and the build-variant macros become `-D_DEBUG -D_UNICODE`.

**The SDK does not pass `-mthumb` or `-mthumb-interwork` on GCCE.** Both the Thumb instruction-set
setting and the interworking define setting are empty in this SDK's GCCE configuration
**(read, confirmed exp 63)**. In the captured makefile the only instruction-set reference anywhere
in a compile rule is the Thumb setting, alongside the floating-point setting and the two define
settings; three of those four are empty in the GCCE configuration and the fourth is `-msoft-float`.
Nothing expands to `-mthumb`, `-mthumb-interwork`, `-D__MARM_THUMB__` or `-D__MARM_INTERWORK__`.

`ALWAYS_BUILD_AS_ARM` swaps the Thumb instruction-set reference for the ARM one and drops the Thumb
defines **(exp 63)**, so on GCCE — where both instruction-set settings are empty — **the directive
changes the compile line not at all**. It is meaningful only to the RVCT platforms.

This contradicts several public write-ups (and `symdev`'s current recorded compile flags, which came
from one of them). The capture settles what the SDK *does*; it does not settle what an E52 accepts.
Before changing anything in `symdev`, confirm which of the two produces a binary the device runs —
that is a build experiment, not a spec question. It is experiment 62, and §15 item 1.

### 8.5 `OPTION`

`OPTION <key> <text…>` requires at least two tokens; fewer warns and the line is dropped
**(read)**. The key is upper-cased. The remaining tokens are joined with single spaces. A second
`OPTION` with the same key **appends** its text (separated by a space) rather than replacing it —
but note the quirk: the "is this the first use" test is done before the first text token is taken,
so repeated use concatenates correctly while `LINKEROPTION` (which shares that test against the
wrong table) does not.

The key that matters is the **platform name**, upper-cased — for GCCE that is `GCCE`. The RVCT
platforms look up `ARMCC` instead. An `OPTION` whose key matches no platform is stored and never
read; it is not an error. So `OPTION ARMCC --diag_suppress 1234` in a GCCE build is silently inert,
which is exactly how portable `.mmp` files are written. Both halves were confirmed in one capture
**(exp 63)**: a `.mmp` carrying `OPTION GCCE -fmy-option` and `OPTION ARMCC --diag_suppress 1234`
put `-fmy-option` at position 12 of §8.4 and nothing at all from the `ARMCC` line.

`OPTION_REPLACE` uses the same syntax and the same key convention but, as §5.5 says, is only read by
the RVCT back end.

### 8.6 Per-extension language options and force-includes

Determined by the source file's extension, after the file name has been lower-cased
**(read; the `.cpp` and `.c` rows confirmed exp 63)**:

| Extension | Added before the includes | Notes |
|---|---|---|
| `.cpp`, `.cc`, `.cxx`, `.c++` | `-x c++ -include <EPOCROOT>EPOC32/INCLUDE/GCCE/GCCE.h` | the force-include path is written with **forward slashes** for GCCE specifically |
| `.c` | `-x c -include <EPOCROOT>EPOC32/INCLUDE/GCCE/GCCE.h` | |
| `.cia` | `-x c++ -S -Wa,-adln` | **no force-include**; CIA files are compiled to assembly and then assembled, and on GCCE there is no assembler-translation step configured, so the intermediate is produced by a preprocess-only pass first |
| `.s`, `.S`, anything else | *(nothing)* | still `arm-none-symbianelf-g++`, still every flag of §8.4, but no `-x` and **no force-include** |

The force-include file is `epoc32/include/gcce/gcce.h` on disk; the configuration file spells the
path in upper case. That file's first act is to `#include` whatever `__PRODUCT_INCLUDE__` names —
which is how the variant header reaches the compiler (§8.2 item 9).

Two spellings of that one path appear in the generated makefile, and they differ **(exp 63)**: the
compile command line carries the forward-slash form (the generator rewrites the configuration's
value for GCCE specifically, and `EPOCROOT` inside it is rewritten too), while the same file is
listed as a *prerequisite* of every object in its backslash form. An implementer needs only the
command-line form; the difference is noted so nobody treats one of them as a typo.

The object file for a `.cia` source is named `<base>_.o`, not `<base>.o` **(read)**. Everything else
is `<base>.o`, all of them in one flat build directory, so **two sources with the same basename in
different directories collide**. The SDK does not detect this.

---

## 9. `TARGETTYPE`

Matched case-insensitively. An unknown type is fatal. Each type fixes a "basic" type (which decides
UID1, the entry-point library and the link shape), optionally a UID2, optionally a default target
path or resource path, optionally required exports, and whether a `.def` file is expected
**(read)**.

| `TARGETTYPE` | Basic | UID2 | Entry-point lib | Default `TARGETPATH` | Resource path | `.def` needed | Notes |
|---|---|---|---|---|---|---|---|
| `EXE` | EXE | — | `EEXE.LIB` | — | — | no | the ordinary application/executable |
| `EXEXP` | EXE | — | `EEXE.LIB` | — | — | **yes** | an EXE that also exports |
| `EXEDLL` | EXEDLL | — | `EEXEDLL.LIB` | — | — | **yes** | EXE on device, DLL on emulator |
| `EPOCEXE` | EXEDLL | — | `EEXEDLL.LIB` | — | — | no | legacy |
| `DLL` | DLL | — | `EDLL.LIB` | — | — | **yes** | |
| `LIB` | LIB | — | (unused) | — | — | no | static library; `LIBRARY`/`STATICLIBRARY`/`DEBUGLIBRARY` are warned about and dropped |
| `IMPLIB` | IMPLIB | — | (unused) | — | — | **yes** | import library only; `SOURCE` is **fatal** |
| `NONE` | IMPLIB | — | (unused) | — | — | no | resource-only project; `TARGET` may be omitted, `SOURCE` is fatal |
| `APP` | DLL | `0x100039ce` | `EDLL.LIB` | — | — | no | **deprecated**, prints a migration note; requires UID3 |
| `PLUGIN` | DLL | `0x10009D8D` | `EDLL.LIB` | — | `Z\Resource\Plugins\` | no | ECOM plug-in |
| `ECOMIIC` | DLL | `0x10009D8D` | `EDLL.LIB` | `Z\System\Libs\Plugins\` | — | no | deprecated |
| `ANI` | DLL | `0x10003b22` | `EDLL.LIB` | — | — | no | |
| `CTL` | DLL | `0x10003a34` | `EDLL.LIB` | — | — | no | deprecated |
| `FSY` | DLL | `0x100039df` | `EDLL.LIB` | — | — | no | |
| `LDD` | DLL | `0x100000af` | `EDEV.LIB` | — | — | no | kernel-side |
| `PDD` | DLL | `0x100039d0` | `EDEV.LIB` | — | — | no | kernel-side |
| `KDLL` | DLL | — | `EKLL.LIB` | — | — | no | kernel-side |
| `KEXT` | DLL | — | `EEXT.LIB` | — | — | no | kernel-side |
| `KLIB` | LIB | — | (unused) | — | — | no | kernel-side |
| `VAR` | DLL | — | `EVAR.LIB` | — | — | no | kernel-side |
| `MDA` | DLL | `0x1000393f` | `EDLL.LIB` | — | — | no | deprecated |
| `MDL` | DLL | `0x10003a19` | `EDLL.LIB` | — | — | no | deprecated |
| `RDL` | DLL | `0x10003a37` | `EDLL.LIB` | — | — | no | deprecated |
| `NOTIFIER` | DLL | `0x10005522` | `EDLL.LIB` | `Z\System\Notifiers\` | — | no | deprecated |
| `NOTIFIER2` | DLL | `0x101fdfae` | `EDLL.LIB` | `Z\System\Notifiers\` | — | no | deprecated |
| `TEXTNOTIFIER2` | DLL | `0x101fe38b` | `EDLL.LIB` | `Z\System\Notifiers\` | — | no | |
| `PDL` | DLL | `0x10003b1c` | `EDLL.LIB` | — | `Z\Resource\Printers\` | no | |
| `STDEXE` | EXE | `0x20004C45` | `EEXE.LIB` | — | — | no | open-environment executable |
| `STDDLL` | DLL | `0x20004C45` | `EDLL.LIB` | — | — | **yes** | |
| `STDLIB` | LIB | — | (unused) | — | — | no | |

"Basic" `DLL` is the default for any type that does not name one. The entry-point library defaults
to `E` + basic type + `.LIB` when the type does not name one explicitly.

The `Exports` each type requires (a mangled symbol that must be exported, e.g. the application
factory function for `APP`) are recorded in the model but only checked on platforms that freeze
exports; they are listed in the SDK's type table and are not reproduced here because a plain
`TARGETTYPE EXE` application has none.

**In scope for symdev: `EXE` only.** Reject every other type with a message naming it and saying
which are implemented.

---

## 10. UIDs, secure ID, vendor ID, capabilities

### 10.1 The UID triple

`UID <a> [<b>]` supplies UID2 and UID3 **(read)**. UID1 is never written in a `.mmp`. Processing, in
order:

1. The list is padded with `0x00000000` until it has two entries.
2. If the target type declares a UID2 (§9) and the given UID2 is `0x00000000`, the type's UID2 is
   substituted. If the given UID2 differs from the type's, a warning is printed — **except** when it
   is exactly `0x01111111`, which is a documented escape for deliberately wrong test UIDs.
3. UID1 is prepended:
   * basic type `EXE` → `0x1000007a`
   * basic type `DLL` → `0x10000079`
   * basic type `EXEDLL` → `0x1000007a` on device, `0x10000079` on the emulator
   * anything else (`LIB`, `IMPLIB`) → `0x00000000`

The resulting triple is passed to the post-linker as `--uid1`, `--uid2`, `--uid3` in that order.

A second UID from a small deprecated set (`0x10005e32`, `0x10004cc1`, `0x10003a30`, `0x10003a19`,
`0x10003a37`, `0x10003a34`, and `0x100039ce` when the basic type is `DLL`) produces a migration note
**(read)**.

### 10.2 `SECUREID` and `VENDORID`

* `SECUREID <n>` — if absent, **the secure ID defaults to UID3** **(read)**. It is always passed to
  the post-linker (`--sid=<value>`), even when it was defaulted, so it always lands in the image's
  security section.
* `VENDORID <n>` — there is **no default**. When the directive is absent, no `--vid` argument is
  passed at all and the post-linker's own default (zero) applies **(read)**.

Both are single arguments; extra arguments warn. Both go through the number parser of §11.1, and a
malformed value is fatal.

Where they land in the E32 image is the post-linker's business, specified in
[elf2e32-options-spec.md](elf2e32-options-spec.md); the front end's contract is just the two values
plus the UID triple plus the capability mask.

### 10.3 `CAPABILITY`

One or more names, case-insensitive, each optionally prefixed with `-` to clear rather than set
**(read)**. The names and their bit positions in the first capability word:

| Bit | Name | Bit | Name |
|---|---|---|---|
| 0 | `TCB` | 10 | `NETWORKCONTROL` |
| 1 | `COMMDD` | 11 | `ALLFILES` |
| 2 | `POWERMGMT` | 12 | `SWEVENT` |
| 3 | `MULTIMEDIADD` | 13 | `NETWORKSERVICES` |
| 4 | `READDEVICEDATA` | 14 | `LOCALSERVICES` |
| 5 | `WRITEDEVICEDATA` | 15 | `READUSERDATA` |
| 6 | `DRM` | 16 | `WRITEUSERDATA` |
| 7 | `TRUSTEDUI` | 17 | `LOCATION` |
| 8 | `PROTSERV` | 18 | `SURROUNDINGSDD` |
| 9 | `DISKADMIN` | 19 | `USERENVIRONMENT` |

Plus:

* `ALL` — the bitwise OR of all of the above (bits 0–19 set).
* `NONE` — accepted, contributes nothing. This is also the default when `CAPABILITY` is absent.
* Ten obsolete names — `ROOT`, `MEDIADD`, `READSYSTEMDATA`, `WRITESYSTEMDATA`, `SOUNDDD`, `UIDD`,
  `KILLANYPROCESS`, `DEVMAN`, `PHONENETWORK`, `LOCALNETWORK` — are recognised, contribute nothing,
  and produce a diagnostic saying the old name was ignored.
* An unknown name produces a warning and is ignored (**not** an error).
* `-ALL` and `-NONE` are rejected with a warning.

The mask is 64 bits in two words; the second word is always zero on this release. The mask is
formatted as two `0x%08x` strings.

Separately, the SDK builds a **textual** capability string in source order: names joined with `+`,
with `-`-prefixed ones written with their leading `-`. That string — not the mask — is what is
passed to the post-linker as `--capability=`. When no capability was given the string is the literal
`none`.

A missing `CAPABILITY` on a DLL or `EXEXP` produces a migration note **(read)**.

Self-signing policy (which capabilities a developer certificate may grant) is **not** part of the
front end; see [uids-capabilities-signing.md](uids-capabilities-signing.md).

---

## 11. Numbers, versions and names

### 11.1 Number format

Every numeric argument (`UID`, `SECUREID`, `VENDORID`, `EPOCSTACKSIZE`, `EPOCHEAPSIZE`,
`EPOCDATALINKADDRESS`) goes through one parser **(read)**:

* the token is lower-cased;
* it must match either 1–10 decimal digits, or `0x` followed by 1–8 hexadecimal digits;
* the result is formatted as `0x%08x`.

Note the asymmetry: hexadecimal is capped at 8 digits, decimal at 10 characters — so `9999999999`
parses and overflows silently, and `0x100000000` is rejected. Anything that does not match is a
**fatal** error for the directive concerned.

### 11.2 `VERSION` and name decoration

`VERSION <major>.<minor> [explicit]` — both parts decimal, both < 32768 **(read)**. Anything else is
fatal, as is a second `VERSION`, as is a trailing token other than `explicit`.

When `VERSION` is absent, the default on an EABI platform (which includes GCCE) is **major 10, minor
0**; on non-EABI platforms it is 1.0. The SDK's own comment for this is that EABI versions start at
10 so they can coexist with the older GCC ABI.

The version turns into a **file-name decoration** written as `{%04x%04x}` of major and minor — so
the default is `{000a0000}`. Where it is applied **(read)**:

| Name | Decorated? |
|---|---|
| `TARGET` (the built file) | only if `explicit` was given |
| `LINKAS` (the recorded link name) | **always**, on device platforms |
| `EXPORTLIBRARY` (the `.dso` basename) | **always**, on device platforms |
| `.def` basename | only if `explicit` was given (and then instead of the `u` suffix) |

Decoration is skipped when the name already ends in a `{8 hex digits}` group.

The link name then gets a second decoration from UID3: if UID3 is not zero, the recorded name
becomes `<basename>[<uid3 without the 0x>]<extension>`. For `gui.exe` with UID3 `0xe7351c20` and the
default version that gives:

> `gui{000a0000}[e7351c20].exe`

This is the string the linker is told to record as the shared-object name **(read)**, and it is the
name other binaries will look for.

A `LIBRARY` or `DEBUGLIBRARY` name may itself carry a `{major.minor}` group in decimal, which is
rewritten to the `{%04x%04x}` form; a malformed group warns **(read)**.

---

## 12. The link and post-link, in outline

Included so the front end's outputs have somewhere to go. Authoritative detail for the post-linker
is [elf2e32-options-spec.md](elf2e32-options-spec.md).

Link, for basic type `EXE`, UREL, GCCE **(read, confirmed exp 63 except where the item says
otherwise)**:

* linker `arm-none-symbianelf-ld`;
* two `-L` search paths, derived by asking the compiler where its `libgcc` is: the toolchain's
  `arm-none-symbianelf/lib` directory and the `libgcc` directory itself;
* `--target1-abs --no-undefined -nostdlib`;
* `-shared`;
* `-Ttext 0x8000` and `-Tdata 0x400000`;
* `--default-symver`;
* `-soname <the decorated link name of §11.2>`;
* `--entry _E32Startup` and `-u _E32Startup` (a DLL uses `_E32Dll`);
* the entry-point static library from *EPOCPATH*`/release/armv5/urel/` — for basic type `EXE` that
  is `EEXE.LIB`. Other toolchains name one archive member here; the GCCE configuration leaves that
  setting empty, so the whole archive is passed;
* `-o <intermediate ELF>`, in the project's build directory — **not** the release directory; the
  post-linker is what writes the release directory;
* `-Map <map file>`, the map going straight to the release directory next to the finished binary.
  **Correction (exp 63):** in the generated makefile this is not a compiler-version probe; the
  option is present whenever the configuration file defines a map-file option, which the GCCE
  configuration does, and the rule deletes any previous map file first. The version probe, if it
  exists, is not what the makefile shows;
* the object files, passed through a linker script fragment listing them. **(exp 63)** For GCCE the
  configuration's "response file" option is empty, so the fragment's path is passed as a bare
  argument with no option in front of it;
* the static libraries, wrapped in `-( … -)`, with the C++ runtime support library `usrt2_2.lib`
  prepended;
* the import libraries: every `LIBRARY` name with its extension replaced by `.dso`, from
  *EPOCPATH*`/release/armv5/lib/`;
* the fixed runtime import libraries, in this order:
  `dfpaeabi.dso dfprvct2_2.dso drtaeabi.dso scppnwdl.dso drtrvct2_2.dso`
  (the configuration file notes the order matters — the C++ new/delete library must precede the
  runtime library). Any of these whose name matches the project's own import library is skipped;
* `-lsupc++ -lgcc`, last;
* a make-level linker-flags hook, **not** reachable from the `.mmp`.

For UDEB the differences are that `DEBUGLIBRARY` names are added to the import library list, that
the configuration's linker debug option is inserted after the `-soname` value, and that the
intermediate ELF is copied to the release directory as `<target>.sym` after the link and before the
post-link **(exp 63)**.

Post-link converts the ELF to an E32 image with, in order **(read, order and the always-present
options confirmed exp 63)**:
`--sid=`, optionally `--version=<major>.<minor>`, optionally `--dlldata`, optionally
`--datalinkaddress=`, optionally `--fixedaddress`, optionally `--heap=<min>,<max>`, optionally
`--priority=<name>`, optionally `--stack=<n>`, then `--uid1=`, `--uid2=`, `--uid3=`, optionally
`--vid=`, then `--capability=<text>`, `--fpu=softvfp|vfpv2`, `--targettype=<name>`, `--output=`,
and for exporting types `--ignorenoncallable` then `--definput=`/`--dso=`/`--defoutput=`, then
**`--elfinput=`, `--linkas=` and `--libpath=`**, then for a target with a system definition
`--sysdef=<symbol>,<ordinal>`, plus `--compressionmethod inflate|bytepair` and `--paged`/`--unpaged`
where the `.mmp` asked for them.

**Correction (exp 63):** the previous list stopped at `--defoutput=`/`--ignorenoncallable` and
omitted `--elfinput=`, `--linkas=`, `--libpath=` and `--sysdef=`. They are not optional extras:
`--elfinput=` names the ELF the link just produced, `--linkas=` carries the decorated name of
§11.2, and `--libpath=` points at *EPOCPATH*`/release/armv5/lib/`. `--ignorenoncallable` sits
immediately after `--output=`, before `--dso=`. The two captured shapes were:

```
<post-linker> --sid=<uid3> --version=10.0 --uid1= --uid2= --uid3=
    --capability=none --fpu=softvfp --targettype=EXE --output="<release>\gui.exe"
    --elfinput="<build>\gui.exe" --linkas=gui{000a0000}[e7351c20].exe
    --libpath="<EPOCPATH>\release\armv5\LIB"
```

```
<post-linker> --sid=<uid3> --version=10.0 --uid1= --uid2= --uid3= --vid=0x00000000
    --capability=<ten names joined with +> --fpu=softvfp --targettype=PLUGIN --output="…"
    --ignorenoncallable --dso=<build>\NPBitmap{000a0000}.dso
    --defoutput=<build>\NPBitmap{000a0000}.def --elfinput="…" --linkas=…
    --libpath="…" --sysdef=<mangled proxy symbol>,1
```

with a further recipe line copying the generated `.def` up out of the variant's build directory.
`VENDORID 0` really does produce `--vid=0x00000000` rather than omitting the option **(exp 63)**,
and a `.mmp` with no `VENDORID` at all produces no `--vid`.

---

## 13. `platform_paths.hrh` and the layer macros

**`platform_paths.hrh` does not exist in this SDK** **(exp)**. Neither does `data_caging_paths.hrh`,
and no header anywhere under `/home/genius/sdk/S60_3rd_FP2/` mentions `MW_LAYER_SYSTEMINCLUDE`,
`APP_LAYER_SYSTEMINCLUDE`, `OS_LAYER_SYSTEMINCLUDE`, `APP_LAYER_LIBC_SYSTEMINCLUDE` or
`OS_LAYER_LIBC_SYSTEMINCLUDE`.

Those macros come from the Symbian **platform source** distributions (the S60/Symbian^n build
environment where components are laid out in `os/`, `mw/` and `app/` layers), not from the public
S60 3rd Edition FP2 SDK. An `.mmp` written against a platform source tree and copied into an SDK
project therefore **cannot be preprocessed here**: the include fails and the preprocessor exits 33,
which the front end turns into a build-stopping error **(exp, §14.4)**.

The expansions of the layer macros on **this** SDK are therefore **unknown, and correctly so —
they have no expansion here**. Do not invent them and do not hard-code a guessed set of
`-I` directories under an `APP_LAYER_SYSTEMINCLUDE` name.

What a native front end must do:

* If an `.mmp` or `bld.inf` includes a header that cannot be found, report the failing include, the
  file and line, and the include search path that was used.
* If that header is `platform_paths.hrh`, add one sentence: it is part of a Symbian platform source
  tree, not of this SDK, and the `.mmp` needs its layer macros replaced by explicit
  `SYSTEMINCLUDE` lines — for an ordinary FP2 application, `SYSTEMINCLUDE \epoc32\include` and
  `SYSTEMINCLUDE \epoc32\include\variant` cover it.

If the user later supplies a platform source tree that does contain the header, no special handling
is needed: it is an ordinary `#include` and the preprocessor resolves it from the `.mmp`'s directory
or from `SYSTEMINCLUDE`-independent `-I` list of §1.2. The **only** thing that would need writing
down then is a new table of expansions, measured from *that* tree.

---

## 14. Experiments

All runs on Ubuntu, `wine`, with the SDK at `/home/genius/sdk/S60_3rd_FP2`. Scratch files were kept
outside the repository in `/tmp/claude-1000/mmp-frontend-spec-work/`. No `winedbg` process was
left behind.

### 14.1 Preprocessor identity

Running the SDK preprocessor with `-v` on empty input prints its banner:

> `GNU CPP version 2.9-psion-98r2 (Symbian build 546) (ARM/EPOC/PE)`
> `#include "..." search starts here:`
> `End of search list.`
> `# 1 ""`

confirming §1.1 and the `# <n> "<file>"` line-marker form.

### 14.2 Which macros are visible in a `.mmp`

A probe `.mmp` with one `MACRO SAW_x` line guarded by each candidate was run through

```
cpp -undef -nostdinc -+ -I "<SDK>\epoc32\include" -I . -I "<probe dir>" \
    -D GCCE=_____GCCE -D EPOC32=_____EPOC32 -D MARM=_____MARM -D EABI=_____EABI \
    -D GENERIC_MARM=_____GENERIC_MARM -D MARM_ARMV5=_____MARM_ARMV5 "<probe>"
```

Output contained `SAW_GCCE`, `SAW_EPOC32_MARM`, `SAW_MARM_ARMV5`, `SAW_GENERIC_MARM`, `SAW_EABI`
and **nothing else**: `UREL`, `_UNICODE`, `NDEBUG` and `__SYMBIAN32__` all produced empty output.
Confirms §1.3.

The same run showed `OPTION GCCE -O3 -DFOO=bar` emerging as `OPTION _____GCCE  -O3 -DFOO=bar`,
confirming the underscore trick and the extra space of §1.4, and showed both comment forms removed
with line numbering preserved (§2.1), and an unknown directive line passed through untouched.

### 14.3 `#if` versus `#ifdef`

A probe containing `#if GCCE` / `MACRO IF_GCCE_TRUE` / `#endif` produced **no** output line, while
`#ifndef WINSCW` produced its body. So with the underscore trick, `#if <MACRO>` evaluates the
identifier `_____GCCE`, which is undefined, hence zero — **`#if GCCE` is always false**. Only
`#ifdef`, `#ifndef` and `defined()` work on platform macros. This is a silent trap and should be
diagnosed by a native implementation.

### 14.4 Missing `platform_paths.hrh`

A probe whose first line is `#include <platform_paths.hrh>` produced

> `<probe>:1: platform_paths.hrh: No such file or directory`

and exit status **33**. A recursive search of the whole SDK for that file name, for
`data_caging_paths.hrh`, and for the string `MW_LAYER_SYSTEMINCLUDE` found nothing. Confirms §13.

### 14.5 Includes, continuations, quoting, tabs

A probe containing an `#include "inc.hrh"`, a backslash continuation, a tab-separated `LIBRARY`
line, a quoted path with spaces, and a leading-tab directive produced:

* nested line markers `# 1 "<inc.hrh>" 1` and `# 2 "<probe>" 2` around the included content;
* the continued `SOURCE a.cpp \` + `b.cpp` joined onto one line followed by one blank line;
* the tab preserved in the output text (and thus acting as a token separator);
* `"C:\Program Files\x"` intact as one quoted run;
* the leading tab before `TARGETTYPE` preserved and harmless.

Confirms §2.2–§2.4.

### 14.6 Variant header macro count

Running the preprocessor in "dump macros" mode on
`epoc32/include/variant/symbian_os_v9.3.hrh` (with the SDK include directory and the variant
directory on the search path) listed **79** `#define`s. Every one of them is visible to `#ifdef`
inside a `.mmp`, because that header is force-included (§1.5).

### 14.7 Not attempted at the time this was written

Running the SDK's project-file and makefile stages end to end. They are Perl programs that require a
Windows Perl (absent here), a drive-lettered working directory, a `Path` environment variable in
Windows form, and a preprocessor reachable under a Windows-shaped path. Reproducing that would have
meant patching copies of SDK modules, which is a larger experiment than the questions it would
settle. Everything in §5–§12 was therefore **(read)** and needed confirming against a real
generated makefile. §14.8 is that confirmation.

### 14.8 The generator, run on this host (experiment 63)

Both stages **do** run on Linux, under the host's own perl, and the makefile they produce has been
captured. The recipe is `sdk-generator.sh` beside this file; the full account, including every
stand-in and the one patched SDK module, is experiment 63 in
[experiment-backlog.md](experiment-backlog.md). A drive letter turned out not to be needed: the
generator's path code strips one if present and is otherwise indifferent. What was needed was an
`EPOCROOT` in Windows shape, a stand-in for the `cmd.exe` `set` builtin, a preprocessor and a `make`
reachable under the names the generator uses, and a preload that translates paths and command-line
quoting at the libc boundary.

Four projects were captured for GCCE: `examples/gui`, and from the SDK's own examples a plugin with
a bitmap block and a console application with two `.c` sources, plus a purpose-built `.mmp` for
`MACRO`, `OPTION` and `ALWAYS_BUILD_AS_ARM`. Everything in this document now marked **(exp 63)**
comes from those makefiles.

Two caveats travel with every **(exp 63)** mark:

* The makefiles were **not run**. Anything a recipe would have done at build time — what the
  resource and bitmap wrappers pass on to the compilers they front, whether the link succeeds —
  is still **(read)**.
* Two behaviours of the SDK's GCC 2.x preprocessor had to be reproduced by hand in the stand-in:
  the space it inserts after a macro expansion, and its habit of emitting a backslash-continued
  statement on one output line. Both were already **(exp)** here from the Wine runs of §14.1–§14.6,
  so the reproduction was checked against this document rather than the other way round. A reader
  who doubts either should trust §14.1–§14.6, not §14.8.

---

## 15. What is unknown, and what to confirm before trusting this

1. **`-mthumb` / `-mthumb-interwork`.** This SDK's GCCE configuration passes neither (§8.4) — now
   confirmed from a generated makefile, not only from a reading — yet `symdev`'s current recorded
   compile line and several public write-ups do. One of the two is wrong for the E52. Settle it by
   building the same source both ways and comparing the resulting E32 images and device behaviour
   — not by reading anything, and not by capturing anything either.
2. ~~**The exact generated makefile.**~~ **Settled** by experiment 63 (§14.8): the generator was
   made to run on this host and its makefile matched §8.2, §8.3, §8.4, §8.5 and §8.6 item by item.
   What remains open is what happens when those recipes are *executed*, which the experiment
   deliberately did not do.
3. **`EPOCSTACKSIZE`/`EPOCHEAPSIZE` limits.** The front end imposes none beyond the number format of
   §11.1. Whether the post-linker or the device rejects extreme values is **unknown** here.
4. **Behaviour of `START <platform> … END` on GCCE.** The block is captured and then nothing reads
   it (§5.2). Whether any real FP2 project relies on a `START GCCE` block is **unknown**; treat a
   non-empty one as a reason to warn.
5. **Two sources with the same basename.** The SDK silently produces one object file (§8.6). Whether
   any shipped project does this is **unknown**; a native builder should detect and refuse it.
6. **Case folding.** The SDK relies on a case-insensitive filesystem in at least four places:
   `bld.inf` tokens are upper-cased before the `.mmp` file is opened, bitmap sources are
   lower-cased, source file names are lower-cased then partially re-capitalised, and the GCCE
   force-include path is spelled in upper case while the file on disk is lower case. A native
   builder on Linux must choose a policy — case-insensitive lookup with a warning is the least
   surprising — and state it in its error messages. The SDK itself has no policy to copy.

---

## 16. Minimum viable model

For the record, the complete model a GCCE/UREL/EXE build needs out of the front end:

* target name, target type, device target path;
* UID triple, secure ID, vendor ID (optional), capability text and mask;
* ordered source list, each with its resolved absolute directory;
* ordered user-include and system-include directory lists;
* ordered `MACRO` list;
* the `OPTION GCCE` text;
* import library names (`.lib` → `.dso`), static library names, and the UDEB-only extra imports;
* stack size, heap min/max, paging mode, compression mode, process priority, FPU — each present or
  absent;
* version (major, minor, explicit flag) and the derived link name;
* zero or more resource blocks: source, basename, device target path, language list, header flag,
  optional UID pair;
* zero or more bitmap blocks: output name, device target path, header flag, ordered
  `(source, depth)` list.

Everything else in this document is either how those fields are spelled, or what happens to them
afterwards.
